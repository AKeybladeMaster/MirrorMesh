package com.mirrormesh.tv;

/** Small JDK-only behavior test for the TV shell's local demonstration state. */
public final class PairingDemoStateTest {
    private PairingDemoStateTest() {}

    public static void main(String[] args) {
        PairingDemoState state = new PairingDemoState();
        expect(PairingDemoState.Phase.STANDBY, state.phase(), "initial phase");
        expect(false, state.accept(), "cannot accept without a request");
        expect(false, state.reject(), "cannot reject without a request");
        expect(false, state.disconnect(), "cannot disconnect without a session");

        expect(true, state.showDemoRequest(), "show explicit local request");
        expect(PairingDemoState.Phase.REQUEST_PENDING, state.phase(), "request phase");
        expect(false, state.isConnected(), "request must not auto-accept");
        expect(false, state.showDemoRequest(), "do not replace a pending request");
        expect(true, state.accept(), "accept pending request");
        expect(PairingDemoState.Phase.CONNECTED, state.phase(), "connected phase");
        expect(false, state.reject(), "cannot reject after acceptance");

        expect(true, state.disconnect(), "disconnect connected demo session");
        expect(PairingDemoState.Phase.STANDBY, state.phase(), "disconnect returns to standby");
        expect(false, state.disconnect(), "disconnect is idempotent in standby");

        expect(true, state.showDemoRequest(), "show another local request");
        expect(true, state.reject(), "reject pending request");
        expect(PairingDemoState.Phase.REJECTED, state.phase(), "rejected phase");
        expect(false, state.accept(), "rejected request cannot connect");
        expect(true, state.showDemoRequest(), "new request after rejection");
        expect(true, state.accept(), "accept new request explicitly");

        System.out.println("Android TV local demo state checks passed.");
    }

    private static void expect(Object expected, Object actual, String label) {
        if (expected == null ? actual != null : !expected.equals(actual)) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }
}
