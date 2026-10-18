package com.mirrormesh.tv;

/** Pure local state for demonstrating explicit TV session approval. */
public final class PairingDemoState {
    public enum Phase {
        STANDBY,
        REQUEST_PENDING,
        CONNECTED,
        REJECTED
    }

    private Phase phase = Phase.STANDBY;

    public Phase phase() {
        return phase;
    }

    public boolean isConnected() {
        return phase == Phase.CONNECTED;
    }

    public boolean showDemoRequest() {
        if (phase != Phase.STANDBY && phase != Phase.REJECTED) {
            return false;
        }
        phase = Phase.REQUEST_PENDING;
        return true;
    }

    public boolean accept() {
        if (phase != Phase.REQUEST_PENDING) {
            return false;
        }
        phase = Phase.CONNECTED;
        return true;
    }

    public boolean reject() {
        if (phase != Phase.REQUEST_PENDING) {
            return false;
        }
        phase = Phase.REJECTED;
        return true;
    }

    public boolean disconnect() {
        if (phase != Phase.CONNECTED) {
            return false;
        }
        phase = Phase.STANDBY;
        return true;
    }
}
