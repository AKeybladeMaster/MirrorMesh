package com.mirrormesh.tv;

/** Explicit approval and native video-session lifecycle for the TV receiver. */
public final class TvReceiverState {
    public enum Phase {
        STANDBY,
        REQUEST_PENDING,
        APPROVED,
        CONNECTING,
        STREAMING,
        RECONNECTING,
        REJECTED,
        REVOKED,
        FAILED,
        STOPPED
    }

    private Phase phase = Phase.STANDBY;

    public synchronized Phase phase() {
        return phase;
    }

    public synchronized boolean showAuthenticatedRequest() {
        if (phase != Phase.STANDBY && phase != Phase.REJECTED && phase != Phase.STOPPED) {
            return false;
        }
        phase = Phase.REQUEST_PENDING;
        return true;
    }

    public synchronized boolean approve() {
        return transition(Phase.REQUEST_PENDING, Phase.APPROVED);
    }

    public synchronized boolean reject() {
        return transition(Phase.REQUEST_PENDING, Phase.REJECTED);
    }

    public synchronized boolean beginConnecting() {
        return transition(Phase.APPROVED, Phase.CONNECTING);
    }

    public synchronized boolean onFirstFramePresented() {
        if (phase != Phase.CONNECTING && phase != Phase.RECONNECTING) {
            return false;
        }
        phase = Phase.STREAMING;
        return true;
    }

    public synchronized boolean onDecodedFrame() {
        if (phase != Phase.CONNECTING && phase != Phase.RECONNECTING) {
            return false;
        }
        phase = Phase.STREAMING;
        return true;
    }

    public synchronized boolean onConnectionInterrupted() {
        if (phase != Phase.CONNECTING && phase != Phase.STREAMING) {
            return false;
        }
        phase = Phase.RECONNECTING;
        return true;
    }

    public synchronized boolean beginReconnect() {
        return transition(Phase.RECONNECTING, Phase.CONNECTING);
    }

    public synchronized boolean onConnectionRestored() {
        return transition(Phase.RECONNECTING, Phase.CONNECTING);
    }

    public synchronized boolean revoke() {
        if (phase != Phase.APPROVED
                && phase != Phase.CONNECTING
                && phase != Phase.STREAMING
                && phase != Phase.RECONNECTING
                && phase != Phase.STOPPED) {
            return false;
        }
        phase = Phase.REVOKED;
        return true;
    }

    public synchronized boolean fail() {
        if (phase != Phase.APPROVED
                && phase != Phase.CONNECTING
                && phase != Phase.STREAMING
                && phase != Phase.RECONNECTING) {
            return false;
        }
        phase = Phase.FAILED;
        return true;
    }

    public synchronized boolean disconnect() {
        if (phase != Phase.APPROVED
                && phase != Phase.CONNECTING
                && phase != Phase.STREAMING
                && phase != Phase.RECONNECTING
                && phase != Phase.FAILED) {
            return false;
        }
        phase = Phase.STOPPED;
        return true;
    }

    private synchronized boolean transition(Phase expected, Phase next) {
        if (phase != expected) {
            return false;
        }
        phase = next;
        return true;
    }
}
