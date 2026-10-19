package com.mirrormesh.tv;

/** Pure pairing UI state; network success and explicit user decisions drive every transition. */
final class TvPairingState {
    enum Phase {
        STOPPED,
        READY,
        WAITING_FOR_REQUEST,
        REQUEST_PENDING,
        PAIRED
    }

    private Phase phase = Phase.STOPPED;
    private long requestId;
    private long connectionId;

    Phase phase() {
        return phase;
    }

    long requestId() {
        return requestId;
    }

    long connectionId() {
        return connectionId;
    }

    boolean canStart() {
        return phase == Phase.STOPPED;
    }

    boolean canWait() {
        return phase == Phase.READY;
    }

    boolean canApproveOrReject() {
        return phase == Phase.REQUEST_PENDING;
    }

    boolean canCloseChannel() {
        return phase == Phase.PAIRED;
    }

    void receiverStarted() {
        phase = Phase.READY;
        requestId = 0;
        connectionId = 0;
    }

    boolean beginWait() {
        if (!canWait()) {
            return false;
        }
        phase = Phase.WAITING_FOR_REQUEST;
        return true;
    }

    boolean requestReceived(long receivedRequestId) {
        if (phase != Phase.WAITING_FOR_REQUEST || receivedRequestId <= 0) {
            return false;
        }
        requestId = receivedRequestId;
        phase = Phase.REQUEST_PENDING;
        return true;
    }

    boolean decisionFailed() {
        if (phase != Phase.REQUEST_PENDING) {
            return false;
        }
        requestId = 0;
        phase = Phase.READY;
        return true;
    }

    boolean requestRejected() {
        return decisionFailed();
    }

    boolean requestApproved(long approvedConnectionId) {
        if (phase != Phase.REQUEST_PENDING || approvedConnectionId <= 0) {
            return false;
        }
        requestId = 0;
        connectionId = approvedConnectionId;
        phase = Phase.PAIRED;
        return true;
    }

    boolean controlChannelClosed() {
        if (!canCloseChannel()) {
            return false;
        }
        connectionId = 0;
        phase = Phase.READY;
        return true;
    }

    void waitFailed() {
        if (phase == Phase.WAITING_FOR_REQUEST) {
            phase = Phase.READY;
        }
    }

    void receiverStopped() {
        phase = Phase.STOPPED;
        requestId = 0;
        connectionId = 0;
    }
}
