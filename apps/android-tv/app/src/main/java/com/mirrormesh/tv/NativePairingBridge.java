package com.mirrormesh.tv;

/** Narrow JNI bridge to the Rust-owned Android pairing runtime and approved WSS channel. */
final class NativePairingBridge {
    static {
        System.loadLibrary("mm_ffi");
    }

    private NativePairingBridge() {}

    static native boolean initializeRustAndroidContext(Object applicationContext);

    /** Starts pairing on an explicitly selected numeric local endpoint and returns JSON. */
    static native String startPairing(String socketAddress);

    /** Blocking wait; call only from a background thread. Returns verified request metadata JSON. */
    static native String waitForPairingRequest();

    /** Persists only the selected grant subset and returns the approved channel handle as JSON. */
    static native String approvePairing(
            long requestId, boolean view, boolean send, boolean annotate);

    /** Sends pair.reject without persisting receiver trust. */
    static native String rejectPairing(long requestId);

    /** Releases an approved signaling connection when the TV session ends. */
    static native boolean closeControlChannel(long connectionId);
}
