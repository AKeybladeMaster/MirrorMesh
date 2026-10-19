package com.mirrormesh.tv;

/** JDK-only lifecycle checks for TV session admission and cleanup. */
public final class TvReceiverStateTest {
    private TvReceiverStateTest() {}

    public static void main(String[] args) {
        TvReceiverState state = new TvReceiverState();
        expect(TvReceiverState.Phase.STANDBY, state.phase(), "initial state");
        expect(false, state.approve(), "cannot approve without an authenticated request");
        expect(false, state.beginConnecting(), "cannot connect before explicit approval");

        expect(true, state.showAuthenticatedRequest(), "show authenticated request");
        expect(true, state.reject(), "explicit rejection");
        expect(TvReceiverState.Phase.REJECTED, state.phase(), "rejected state");
        expect(false, state.beginConnecting(), "rejected request cannot connect");

        expect(true, state.showAuthenticatedRequest(), "new request after rejection");
        expect(true, state.approve(), "explicit approval");
        expect(true, state.beginConnecting(), "start transport after approval");
        expect(true, state.onFirstFramePresented(), "stream after a presented frame");
        expect(TvReceiverState.Phase.STREAMING, state.phase(), "streaming state");
        expect(true, state.onConnectionInterrupted(), "recoverable disconnect");
        expect(TvReceiverState.Phase.RECONNECTING, state.phase(), "interrupted state");
        expect(true, state.onConnectionRestored(), "same peer recovers");
        expect(TvReceiverState.Phase.CONNECTING, state.phase(), "recovered peer awaits video");
        expect(true, state.onDecodedFrame(), "decoded frame resumes stream");
        expect(TvReceiverState.Phase.STREAMING, state.phase(), "recovered stream state");
        expect(true, state.onConnectionInterrupted(), "second recoverable disconnect");
        expect(true, state.beginReconnect(), "explicit reconnect attempt");
        expect(true, state.onFirstFramePresented(), "resume after a presented frame");
        expect(true, state.revoke(), "revocation closes the active session");
        expect(TvReceiverState.Phase.REVOKED, state.phase(), "revoked state");
        expect(false, state.beginReconnect(), "revoked session cannot reconnect");

        TvReceiverState failedStream = new TvReceiverState();
        failedStream.showAuthenticatedRequest();
        failedStream.approve();
        failedStream.beginConnecting();
        failedStream.onFirstFramePresented();
        expect(true, failedStream.fail(), "streaming session can enter failed state");
        expect(TvReceiverState.Phase.FAILED, failedStream.phase(), "failed stream state");

        TvReceiverState senderStoppedStream = new TvReceiverState();
        senderStoppedStream.showAuthenticatedRequest();
        senderStoppedStream.approve();
        senderStoppedStream.beginConnecting();
        senderStoppedStream.onFirstFramePresented();
        expect(true, senderStoppedStream.disconnect(), "sender stop ends the active session");
        expect(TvReceiverState.Phase.STOPPED, senderStoppedStream.phase(), "sender stop state");
        expect(false, senderStoppedStream.beginReconnect(), "stopped session cannot reconnect");

        TvReceiverState reconnectExpired = new TvReceiverState();
        reconnectExpired.showAuthenticatedRequest();
        reconnectExpired.approve();
        reconnectExpired.beginConnecting();
        expect(true, reconnectExpired.onConnectionInterrupted(), "initial connection can retry");
        expect(true, reconnectExpired.fail(), "expired reconnect enters failed state");
        expect(TvReceiverState.Phase.FAILED, reconnectExpired.phase(), "reconnect timeout state");
        System.out.println("Android TV receiver lifecycle checks passed.");
    }

    private static void expect(Object expected, Object actual, String label) {
        if (expected == null ? actual != null : !expected.equals(actual)) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }
}
