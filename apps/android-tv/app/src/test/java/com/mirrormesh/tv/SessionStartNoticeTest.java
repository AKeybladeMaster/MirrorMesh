package com.mirrormesh.tv;

/** JDK-only checks for bounded sender source-start metadata. */
public final class SessionStartNoticeTest {
    public static void main(String[] args) {
        expect("desktop-capture".equals(SessionStartNotice.validateSourceId("desktop-capture")),
                "valid source ID must be preserved");
        expectInvalid(null);
        expectInvalid("");
        expectInvalid("x".repeat(129));
        expectInvalid("window\nname");
    }

    private static void expectInvalid(String sourceId) {
        try {
            SessionStartNotice.validateSourceId(sourceId);
            throw new AssertionError("invalid source ID must be rejected");
        } catch (IllegalArgumentException expected) {
            // Expected input rejection.
        }
    }

    private static void expect(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message);
        }
    }
}
