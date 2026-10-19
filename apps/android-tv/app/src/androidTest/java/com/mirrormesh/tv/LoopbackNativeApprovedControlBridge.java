package com.mirrormesh.tv;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.json.JSONException;
import org.json.JSONObject;
import org.webrtc.IceCandidate;
import org.webrtc.PeerConnection;
import org.webrtc.SdpObserver;
import org.webrtc.SessionDescription;

/** Test double for the Rust JNI surface; media still flows through native libwebrtc. */
final class LoopbackNativeApprovedControlBridge
        implements RustApprovedSignalingChannel.NativeApprovedControlBridge {
    private final PeerConnection sender;
    private final BlockingQueue<String> incomingMessages = new LinkedBlockingQueue<>();
    private final List<IceCandidate> senderCandidatesWaitingForAnswer = new ArrayList<>();
    private final AtomicInteger senderIceCandidatesSent = new AtomicInteger();
    private final AtomicInteger localIceCandidatesSent = new AtomicInteger();
    private final CompletableFuture<Void> answerApplied = new CompletableFuture<>();
    private boolean senderRemoteDescriptionSet;
    private volatile boolean closed;

    LoopbackNativeApprovedControlBridge(PeerConnection sender) {
        this.sender = sender;
    }

    void sendOffer(String sdp) {
        enqueue(put(message("offer"), "sdp", sdp));
    }

    void onSenderIceCandidate(IceCandidate candidate) {
        senderIceCandidatesSent.incrementAndGet();
        JSONObject message = message("ice");
        put(message, "media_id", candidate.sdpMid);
        put(message, "media_line_index", candidate.sdpMLineIndex);
        put(message, "candidate", candidate.sdp);
        enqueue(message);
    }

    CompletableFuture<Void> answerApplied() {
        return answerApplied;
    }

    int localIceCandidatesSent() {
        return localIceCandidatesSent.get();
    }

    int senderIceCandidatesSent() {
        return senderIceCandidatesSent.get();
    }

    @Override
    public String pollControlMessage(long connectionId) throws InterruptedException {
        String message = incomingMessages.poll(250, TimeUnit.MILLISECONDS);
        if (message != null) {
            return message;
        }
        JSONObject pending = new JSONObject();
        put(pending, "ok", true);
        put(pending, "pending", true);
        return pending.toString();
    }

    @Override
    public String sendSessionAnswer(long connectionId, String sdp) {
        sender.setRemoteDescription(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription description) {}

            @Override
            public void onSetSuccess() {
                List<IceCandidate> pendingCandidates;
                synchronized (senderCandidatesWaitingForAnswer) {
                    senderRemoteDescriptionSet = true;
                    pendingCandidates = new ArrayList<>(senderCandidatesWaitingForAnswer);
                    senderCandidatesWaitingForAnswer.clear();
                }
                for (IceCandidate candidate : pendingCandidates) {
                    if (!sender.addIceCandidate(candidate)) {
                        answerApplied.completeExceptionally(
                                new IllegalStateException("Sender rejected queued TV ICE."));
                        return;
                    }
                }
                answerApplied.complete(null);
            }

            @Override
            public void onCreateFailure(String error) {
                answerApplied.completeExceptionally(new IllegalStateException(error));
            }

            @Override
            public void onSetFailure(String error) {
                answerApplied.completeExceptionally(new IllegalStateException(error));
            }
        }, new SessionDescription(SessionDescription.Type.ANSWER, sdp));
        return success();
    }

    @Override
    public String sendSessionIce(
            long connectionId,
            String mediaId,
            int mediaLineIndex,
            String candidateText) {
        IceCandidate candidate = new IceCandidate(mediaId, mediaLineIndex, candidateText);
        synchronized (senderCandidatesWaitingForAnswer) {
            if (!senderRemoteDescriptionSet) {
                senderCandidatesWaitingForAnswer.add(candidate);
                localIceCandidatesSent.incrementAndGet();
                return success();
            }
        }
        if (!sender.addIceCandidate(candidate)) {
            return failure("sender rejected TV ICE");
        }
        localIceCandidatesSent.incrementAndGet();
        return success();
    }

    @Override
    public boolean closeControlChannel(long connectionId) {
        closed = true;
        incomingMessages.clear();
        return true;
    }

    private JSONObject message(String kind) {
        JSONObject message = new JSONObject();
        put(message, "ok", true);
        put(message, "pending", false);
        put(message, "kind", kind);
        return message;
    }

    private void enqueue(JSONObject message) {
        if (!closed) {
            incomingMessages.add(message.toString());
        }
    }

    private static String success() {
        JSONObject response = new JSONObject();
        put(response, "ok", true);
        return response.toString();
    }

    private static String failure(String reason) {
        JSONObject response = new JSONObject();
        put(response, "ok", false);
        put(response, "error", reason);
        return response.toString();
    }

    private static JSONObject put(JSONObject object, String name, Object value) {
        try {
            object.put(name, value);
            return object;
        } catch (JSONException error) {
            throw new IllegalStateException("Could not encode loopback signaling JSON.", error);
        }
    }
}
