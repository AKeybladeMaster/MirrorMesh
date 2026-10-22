package com.mirrormesh.tv;

/** Validates bounded source metadata received over the approved control channel. */
final class SessionStartNotice {
    private static final int MAX_SOURCE_ID_LENGTH = 128;

    private SessionStartNotice() {}

    static String validateSourceId(String sourceId) {
        if (sourceId == null || sourceId.isEmpty() || sourceId.length() > MAX_SOURCE_ID_LENGTH) {
            throw new IllegalArgumentException("The sender source ID is invalid.");
        }
        for (int index = 0; index < sourceId.length(); index++) {
            if (Character.isISOControl(sourceId.charAt(index))) {
                throw new IllegalArgumentException("The sender source ID is invalid.");
            }
        }
        return sourceId;
    }
}
