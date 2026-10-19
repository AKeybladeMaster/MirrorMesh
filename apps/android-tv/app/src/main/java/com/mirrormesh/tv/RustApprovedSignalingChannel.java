package com.mirrormesh.tv;

import android.util.Log;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Queue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;
import org.json.JSONException;
import org.json.JSONObject;
import org.webrtc.PeerConnection;

/** Java WebRTC signaling adapter backed by the Rust-authenticated approved WSS connection. */
final class RustApprovedSignalingChannel implements ApprovedSignalingChannel {
    private static final String TAG = "MirrorMeshSignaling";
    private static final int MAX_BUFFERED_ICE_CANDIDATES = 64;
    private static final int MAX_BUFFERED_REMOTE_OFFERS = 4;

    private static final class RemoteIceCandidate {
        final String mediaId;
        final int mediaLineIndex;
        final String candidate;

        RemoteIceCandidate(String mediaId, int mediaLineIndex, String candidate) {
            this.mediaId = mediaId;
            this.mediaLineIndex = mediaLineIndex;
            this.candidate = candidate;
        }
    }

    private final long connectionId;
    private final String fingerprint;
    private final ExecutorService receiveWorker = Executors.newSingleThreadExecutor();
    private final AtomicBoolean closed = new AtomicBoolean();
    private final Object listenerLock = new Object();
    private final Queue<RemoteIceCandidate> bufferedIceCandidates = new ArrayDeque<>();
    private final Queue<String> bufferedRemoteOffers = new ArrayDeque<>();

    private volatile RemoteIceCandidateListener remoteIceListener;
    private volatile RemoteOfferListener remoteOfferListener;
    private volatile Runnable revokedListener;
    private volatile Runnable remoteStopListener;
    private volatile TextListener failureListener;
    private volatile TextListener initialOfferListener;
    private volatile boolean initialOfferReceived;

    RustApprovedSignalingChannel(long connectionId, String fingerprint) {
        if (connectionId <= 0 || fingerprint == null || fingerprint.length() != 64) {
            throw new IllegalArgumentException("An approved connection and peer fingerprint are required.");
        }
        this.connectionId = connectionId;
        this.fingerprint = fingerprint;
    }

    void start(TextListener onInitialOffer) {
        if (onInitialOffer == null || initialOfferListener != null) {
            throw new IllegalStateException("The approved signaling receiver can start only once.");
        }
        initialOfferListener = onInitialOffer;
        receiveWorker.execute(this::receiveLoop);
    }

    @Override
    public boolean isAuthenticatedAndApproved() {
        return !closed.get();
    }

    @Override
    public String peerFingerprint() {
        return fingerprint;
    }

    @Override
    public List<PeerConnection.IceServer> iceServers() {
        return Collections.emptyList();
    }

    @Override
    public void sendAnswer(String sdp) {
        requireSuccess(NativePairingBridge.sendSessionAnswer(connectionId, sdp));
    }

    @Override
    public void sendLocalIceCandidate(String mediaId, int mediaLineIndex, String candidate) {
        requireSuccess(NativePairingBridge.sendSessionIce(
                connectionId, mediaId, mediaLineIndex, candidate));
    }

    @Override
    public void setRemoteIceCandidateListener(RemoteIceCandidateListener listener) {
        List<RemoteIceCandidate> buffered;
        synchronized (listenerLock) {
            remoteIceListener = listener;
            buffered = new ArrayList<>(bufferedIceCandidates);
            bufferedIceCandidates.clear();
        }
        if (listener != null) {
            for (RemoteIceCandidate candidate : buffered) {
                listener.onCandidate(
                        candidate.mediaId, candidate.mediaLineIndex, candidate.candidate);
            }
        }
    }

    @Override
    public void setRemoteOfferListener(RemoteOfferListener listener) {
        List<String> buffered;
        synchronized (listenerLock) {
            remoteOfferListener = listener;
            buffered = new ArrayList<>(bufferedRemoteOffers);
            bufferedRemoteOffers.clear();
        }
        if (listener != null) {
            for (String offer : buffered) {
                listener.onOffer(offer);
            }
        }
    }

    @Override
    public void setRevokedListener(Runnable listener) {
        revokedListener = listener;
    }

    @Override
    public void setRemoteStopListener(Runnable listener) {
        remoteStopListener = listener;
    }

    @Override
    public void setFailureListener(TextListener listener) {
        failureListener = listener;
    }

    @Override
    public void clearListeners() {
        synchronized (listenerLock) {
            remoteIceListener = null;
            remoteOfferListener = null;
            bufferedIceCandidates.clear();
            bufferedRemoteOffers.clear();
        }
        revokedListener = null;
        remoteStopListener = null;
        failureListener = null;
        initialOfferListener = null;
    }

    @Override
    public void close() {
        closeForUser();
    }

    boolean closeForUser() {
        if (!closed.compareAndSet(false, true)) {
            return true;
        }
        receiveWorker.shutdownNow();
        clearListeners();
        try {
            return NativePairingBridge.closeControlChannel(connectionId);
        } catch (LinkageError error) {
            Log.w(TAG, "Rust control-channel close failed", error);
            return false;
        }
    }

    private void receiveLoop() {
        while (!closed.get()) {
            try {
                JSONObject message = requireSuccess(
                        NativePairingBridge.pollControlMessage(connectionId));
                if (message.optBoolean("pending")) {
                    continue;
                }
                switch (message.optString("kind")) {
                    case "offer" -> receiveOffer(message.getString("sdp"));
                    case "ice" -> receiveIce(
                            message.optString("media_id"),
                            message.getInt("media_line_index"),
                            message.getString("candidate"));
                    case "stop" -> receiveRemoteStop();
                    case "revoke" -> receiveRevocation();
                    case "start" -> { }
                    default -> throw new IllegalStateException(
                            "Rust signaling returned an unknown message kind.");
                }
            } catch (Exception | LinkageError error) {
                if (!closed.get()) {
                    notifyFailure("signaling_channel_failed");
                    close();
                }
                return;
            }
        }
    }

    private void receiveOffer(String sdp) {
        TextListener initialListener = null;
        RemoteOfferListener reconnectListener = null;
        synchronized (listenerLock) {
            if (!initialOfferReceived) {
                initialOfferReceived = true;
                initialListener = initialOfferListener;
            } else if (remoteOfferListener != null) {
                reconnectListener = remoteOfferListener;
            } else if (bufferedRemoteOffers.size() < MAX_BUFFERED_REMOTE_OFFERS) {
                bufferedRemoteOffers.add(sdp);
                return;
            }
        }
        if (initialListener != null) {
            initialListener.onText(sdp);
        } else if (reconnectListener != null) {
            reconnectListener.onOffer(sdp);
        } else {
            notifyFailure("remote_offer_queue_full");
        }
    }

    private void receiveIce(String mediaId, int mediaLineIndex, String candidate) {
        RemoteIceCandidateListener listener;
        boolean queueFull = false;
        synchronized (listenerLock) {
            listener = remoteIceListener;
            if (listener == null) {
                if (bufferedIceCandidates.size() >= MAX_BUFFERED_ICE_CANDIDATES) {
                    queueFull = true;
                } else {
                    bufferedIceCandidates.add(
                            new RemoteIceCandidate(mediaId, mediaLineIndex, candidate));
                    return;
                }
            }
        }
        if (queueFull) {
            notifyFailure("remote_candidate_queue_full");
            return;
        }
        listener.onCandidate(mediaId, mediaLineIndex, candidate);
    }

    private void receiveRemoteStop() {
        Runnable listener = remoteStopListener;
        if (listener != null) {
            listener.run();
        }
        close();
    }

    private void receiveRevocation() {
        Runnable listener = revokedListener;
        if (listener != null) {
            listener.run();
        }
        close();
    }

    private void notifyFailure(String code) {
        TextListener listener = failureListener;
        if (listener != null) {
            listener.onText(code);
        }
    }

    private static JSONObject requireSuccess(String response) {
        try {
            JSONObject result = new JSONObject(response);
            if (!result.optBoolean("ok")) {
                throw new IllegalStateException(result.optString("error", "Signaling failed."));
            }
            return result;
        } catch (JSONException error) {
            throw new IllegalStateException("Rust signaling returned malformed JSON.", error);
        }
    }
}
