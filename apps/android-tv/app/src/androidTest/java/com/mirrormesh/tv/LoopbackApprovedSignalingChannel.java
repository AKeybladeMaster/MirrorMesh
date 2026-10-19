package com.mirrormesh.tv;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import java.util.Queue;
import java.util.concurrent.CompletableFuture;
import org.webrtc.IceCandidate;
import org.webrtc.PeerConnection;
import org.webrtc.SdpObserver;
import org.webrtc.SessionDescription;

/** In-process signaling used only by the Android native WebRTC instrumentation test. */
final class LoopbackApprovedSignalingChannel implements ApprovedSignalingChannel {
    private final PeerConnection sender;
    private final Queue<IceCandidate> senderCandidates = new ArrayDeque<>();
    private final Queue<IceCandidate> receiverCandidates = new ArrayDeque<>();
    private RemoteIceCandidateListener remoteIceListener;
    private RemoteStartListener remoteStartListener;
    private RemoteOfferListener remoteOfferListener;
    private Runnable revokedListener;
    private Runnable remoteStopListener;
    private TextListener failureListener;
    private boolean senderRemoteDescriptionSet;
    private boolean closed;
    private final CompletableFuture<Void> answerApplied = new CompletableFuture<>();

    LoopbackApprovedSignalingChannel(PeerConnection sender) {
        this.sender = sender;
    }

    @Override
    public synchronized boolean isAuthenticatedAndApproved() {
        return !closed;
    }

    @Override
    public String peerFingerprint() {
        return "instrumentation-loopback";
    }

    @Override
    public List<PeerConnection.IceServer> iceServers() {
        return List.of();
    }

    @Override
    public void sendAnswer(String sdp) {
        sender.setRemoteDescription(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription description) {}

            @Override
            public void onSetSuccess() {
                List<IceCandidate> queued;
                synchronized (LoopbackApprovedSignalingChannel.this) {
                    senderRemoteDescriptionSet = true;
                    queued = new ArrayList<>(receiverCandidates);
                    receiverCandidates.clear();
                }
                for (IceCandidate candidate : queued) {
                    sender.addIceCandidate(candidate);
                }
                answerApplied.complete(null);
            }

            @Override
            public void onCreateFailure(String error) {
                answerApplied.completeExceptionally(new IllegalStateException(error));
                reportFailure("loopback_answer_set_failed");
            }

            @Override
            public void onSetFailure(String error) {
                answerApplied.completeExceptionally(new IllegalStateException(error));
                reportFailure("loopback_answer_set_failed");
            }
        }, new SessionDescription(SessionDescription.Type.ANSWER, sdp));
    }

    @Override
    public void sendLocalIceCandidate(String mediaId, int mediaLineIndex, String candidate) {
        IceCandidate value = new IceCandidate(mediaId, mediaLineIndex, candidate);
        synchronized (this) {
            if (!senderRemoteDescriptionSet) {
                receiverCandidates.add(value);
                return;
            }
        }
        if (!sender.addIceCandidate(value)) {
            reportFailure("loopback_receiver_ice_rejected");
        }
    }

    void onSenderIceCandidate(IceCandidate candidate) {
        RemoteIceCandidateListener listener;
        synchronized (this) {
            listener = remoteIceListener;
            if (listener == null) {
                senderCandidates.add(candidate);
                return;
            }
        }
        listener.onCandidate(candidate.sdpMid, candidate.sdpMLineIndex, candidate.sdp);
    }

    @Override
    public void setRemoteIceCandidateListener(RemoteIceCandidateListener listener) {
        List<IceCandidate> queued;
        synchronized (this) {
            remoteIceListener = listener;
            queued = new ArrayList<>(senderCandidates);
            senderCandidates.clear();
        }
        if (listener != null) {
            for (IceCandidate candidate : queued) {
                listener.onCandidate(
                        candidate.sdpMid, candidate.sdpMLineIndex, candidate.sdp);
            }
        }
    }

    @Override
    public synchronized void setRemoteStartListener(RemoteStartListener listener) {
        remoteStartListener = listener;
    }

    @Override
    public synchronized void setRemoteOfferListener(RemoteOfferListener listener) {
        remoteOfferListener = listener;
    }

    @Override
    public synchronized void setRevokedListener(Runnable listener) {
        revokedListener = listener;
    }

    @Override
    public synchronized void setRemoteStopListener(Runnable listener) {
        remoteStopListener = listener;
    }

    @Override
    public synchronized void setFailureListener(TextListener listener) {
        failureListener = listener;
    }

    @Override
    public synchronized void clearListeners() {
        remoteIceListener = null;
        remoteStartListener = null;
        remoteOfferListener = null;
        revokedListener = null;
        remoteStopListener = null;
        failureListener = null;
    }

    @Override
    public synchronized void close() {
        closed = true;
        clearListeners();
    }

    private void reportFailure(String value) {
        TextListener listener;
        synchronized (this) {
            listener = failureListener;
        }
        if (listener != null) {
            listener.onText(value);
        }
    }

    CompletableFuture<Void> answerApplied() {
        return answerApplied;
    }
}
