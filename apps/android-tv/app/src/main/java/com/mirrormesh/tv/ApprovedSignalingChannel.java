package com.mirrormesh.tv;

import java.util.List;
import org.webrtc.PeerConnection;

/**
 * Authenticated, user-approved signaling supplied by the pairing/session runtime.
 * Implementations must originate from the Rust-backed identity and trust flow.
 */
interface ApprovedSignalingChannel extends AutoCloseable {
    boolean isAuthenticatedAndApproved();

    String peerFingerprint();

    List<PeerConnection.IceServer> iceServers();

    void sendAnswer(String sdp);

    void sendLocalIceCandidate(String mediaId, int mediaLineIndex, String candidate);

    void setRemoteIceCandidateListener(RemoteIceCandidateListener listener);

    void setRemoteStartListener(RemoteStartListener listener);

    void setRemoteOfferListener(RemoteOfferListener listener);

    void setRevokedListener(Runnable listener);

    void setRemoteStopListener(Runnable listener);

    void setFailureListener(TextListener listener);

    void clearListeners();

    /** Immediately blocks further signaling, then persists revocation on a worker. */
    void revoke(TextListener completion);

    @Override
    void close();

    interface RemoteIceCandidateListener {
        void onCandidate(String mediaId, int mediaLineIndex, String candidate);
    }

    interface RemoteOfferListener {
        void onOffer(String sdp);
    }

    interface RemoteStartListener {
        void onStarted(String sourceId);
    }

    interface TextListener {
        void onText(String value);
    }
}
