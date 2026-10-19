package com.mirrormesh.tv;

/** One bounded offer delivered only after identity verification and explicit TV approval. */
final class TvPendingApprovedOffer {
    final ApprovedSignalingChannel signaling;
    final String offerSdp;
    final TvReceiverState state;

    TvPendingApprovedOffer(
            ApprovedSignalingChannel signaling,
            String offerSdp,
            TvReceiverState state) {
        if (signaling == null || !signaling.isAuthenticatedAndApproved()) {
            throw new SecurityException("A verified, approved signaling session is required.");
        }
        if (state == null || state.phase() != TvReceiverState.Phase.APPROVED) {
            throw new IllegalStateException("The TV user must approve the session first.");
        }
        H264SdpPolicy.requireH264VideoCodec(offerSdp);
        this.signaling = signaling;
        this.offerSdp = offerSdp;
        this.state = state;
    }
}
