package com.mirrormesh.tv;

import android.app.Activity;
import android.content.Intent;
import java.util.concurrent.atomic.AtomicReference;

/** Internal handoff from the authenticated pairing flow to the native video activity. */
final class TvApprovedSessionLauncher {
    private static final AtomicReference<TvPendingApprovedOffer> PENDING = new AtomicReference<>();

    private TvApprovedSessionLauncher() {}

    static void open(
            Activity activity,
            ApprovedSignalingChannel signaling,
            String offerSdp,
            TvReceiverState state) {
        TvPendingApprovedOffer pending = new TvPendingApprovedOffer(signaling, offerSdp, state);
        if (!PENDING.compareAndSet(null, pending)) {
            signaling.close();
            throw new IllegalStateException("A TV session is already being opened.");
        }
        try {
            activity.startActivity(new Intent(activity, TvVideoActivity.class));
        } catch (RuntimeException error) {
            PENDING.compareAndSet(pending, null);
            signaling.close();
            throw error;
        }
    }

    static TvPendingApprovedOffer takePending() {
        return PENDING.getAndSet(null);
    }
}
