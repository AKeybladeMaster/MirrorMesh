package com.mirrormesh.tv;

/** Host-runnable state checks for the explicit Android TV pairing flow. */
public final class TvPairingStateTest {
    public static void main(String[] args) {
        requestRequiresASeparateDecisionAndRejectReturnsToReady();
        approvedChannelMustBeExplicitlyClosedBeforeWaitingAgain();
        cancelledWaitReturnsToReadyForInvitationRefresh();
        stoppedReceiverDropsPendingHandles();
        System.out.println("Android TV explicit pairing state checks passed.");
    }

    private static void requestRequiresASeparateDecisionAndRejectReturnsToReady() {
        TvPairingState state = new TvPairingState();
        check(state.canStart(), "stopped receiver should allow start");
        state.receiverStarted();
        check(state.beginWait(), "started receiver should allow explicit wait");
        check(state.requestReceived(7), "verified request should become pending");
        check(state.phase() == TvPairingState.Phase.REQUEST_PENDING,
                "arrival must not imply approval");
        check(state.canApproveOrReject(), "pending request should expose user decisions");
        check(state.requestRejected(), "explicit rejection should clear the request");
        check(state.phase() == TvPairingState.Phase.READY,
                "rejection should require an explicit next wait");
        check(state.canWait(), "receiver should allow another wait after rejection");
    }

    private static void approvedChannelMustBeExplicitlyClosedBeforeWaitingAgain() {
        TvPairingState state = new TvPairingState();
        state.receiverStarted();
        check(state.beginWait(), "receiver should begin waiting");
        check(state.requestReceived(8), "verified request should become pending");
        check(state.requestApproved(42), "approval should retain a control-channel handle");
        check(state.phase() == TvPairingState.Phase.PAIRED,
                "approval should represent pairing only");
        check(!state.canWait(), "paired receiver must close channel before another wait");
        check(state.controlChannelClosed(), "explicit close should release the handle");
        check(state.canWait(), "closed channel should allow another pairing request");
    }

    private static void stoppedReceiverDropsPendingHandles() {
        TvPairingState state = new TvPairingState();
        state.receiverStarted();
        state.beginWait();
        state.requestReceived(9);
        state.receiverStopped();
        check(state.phase() == TvPairingState.Phase.STOPPED,
                "stop should return the receiver to stopped");
        check(state.requestId() == 0 && state.connectionId() == 0,
                "stop should clear local request and channel handles");
        check(!state.canApproveOrReject(), "stopped request cannot be approved or rejected");
    }

    private static void cancelledWaitReturnsToReadyForInvitationRefresh() {
        TvPairingState state = new TvPairingState();
        state.receiverStarted();
        check(state.beginWait(), "receiver should begin waiting");
        state.waitFailed();
        check(state.phase() == TvPairingState.Phase.READY,
                "cancelling the idle wait should leave the listener ready");
        check(state.canWait(), "receiver should permit another wait after a code refresh");
    }

    private static void check(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }
}
