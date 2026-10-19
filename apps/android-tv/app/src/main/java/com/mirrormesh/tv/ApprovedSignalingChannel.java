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

    void setRemoteOfferListener(RemoteOfferListener listener);

    void setRevokedListener(Runnable listener);

    void clearListeners();

    @Override
    void close();

    interface RemoteIceCandidateListener {
        void onCandidate(String mediaId, int mediaLineIndex, String candidate);
    }

    interface RemoteOfferListener {
        void onOffer(String sdp);
    }
}
