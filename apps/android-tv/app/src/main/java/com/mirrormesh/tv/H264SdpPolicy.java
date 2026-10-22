package com.mirrormesh.tv;

import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/** Bounded SDP policy for the native Android TV H.264-only receive surface. */
public final class H264SdpPolicy {
    public static final int MAX_SDP_BYTES = 256 * 1024;

    private H264SdpPolicy() {}

    /** Returns true only for bounded SDP with one m=video section advertising H264/90000. */
    public static boolean hasH264VideoCodec(String sdp) {
        if (sdp == null || sdp.isEmpty() || sdp.length() > MAX_SDP_BYTES
                || sdp.getBytes(StandardCharsets.UTF_8).length > MAX_SDP_BYTES) {
            return false;
        }

        boolean inVideoSection = false;
        boolean foundH264 = false;
        int videoSectionCount = 0;
        Set<String> videoPayloads = new HashSet<>();
        for (String line : sdp.split("\\r\\n|\\n|\\r")) {
            if (line.startsWith("m=")) {
                inVideoSection = false;
                videoPayloads.clear();
                if (line.startsWith("m=video")) {
                    videoSectionCount++;
                    if (videoSectionCount > 1) {
                        return false;
                    }
                    String[] fields = line.trim().split("\\s+");
                    if (fields.length < 4 || !fields[0].equals("m=video")
                            || !isActivePort(fields[1])) {
                        continue;
                    }
                    inVideoSection = true;
                    for (int index = 3; index < fields.length; index++) {
                        if (!isPayloadType(fields[index])) {
                            return false;
                        }
                        videoPayloads.add(fields[index]);
                    }
                }
            } else if (inVideoSection && line.regionMatches(true, 0, "a=rtpmap:", 0, 9)) {
                if (isH264MappingForVideoPayload(line, videoPayloads)) {
                    foundH264 = true;
                }
            }
        }
        return videoSectionCount == 1 && foundH264;
    }

    /** Returns true only when every negotiated video payload is H.264. */
    public static boolean hasOnlyH264VideoCodec(String sdp) {
        if (!hasH264VideoCodec(sdp)) {
            return false;
        }

        boolean inVideoSection = false;
        Set<String> videoPayloads = new HashSet<>();
        Map<String, String> videoMappings = new HashMap<>();
        for (String line : sdp.split("\\r\\n|\\n|\\r")) {
            if (line.startsWith("m=")) {
                inVideoSection = false;
                if (line.startsWith("m=video")) {
                    String[] fields = line.trim().split("\\s+");
                    if (fields.length < 4 || !fields[0].equals("m=video")
                            || !isActivePort(fields[1])) {
                        return false;
                    }
                    inVideoSection = true;
                    for (int index = 3; index < fields.length; index++) {
                        if (!isPayloadType(fields[index]) || !videoPayloads.add(fields[index])) {
                            return false;
                        }
                    }
                }
            } else if (inVideoSection && line.regionMatches(true, 0, "a=rtpmap:", 0, 9)) {
                int colon = line.indexOf(':');
                int separator = line.indexOf(' ', colon + 1);
                if (colon < 0 || separator < 0) {
                    return false;
                }
                String payload = line.substring(colon + 1, separator).trim();
                if (!videoPayloads.contains(payload)) {
                    continue;
                }
                String encoding = line.substring(separator + 1).trim().toUpperCase(Locale.ROOT);
                if (videoMappings.containsKey(payload)) {
                    return false;
                }
                videoMappings.put(payload, encoding);
            }
        }

        for (String payload : videoPayloads) {
            if (!"H264/90000".equals(videoMappings.get(payload))) {
                return false;
            }
        }
        return !videoPayloads.isEmpty();
    }

    /** Rejects non-H.264, malformed, absent, or oversized SDP before WebRTC sees it. */
    public static void requireH264VideoCodec(String sdp) {
        if (!hasH264VideoCodec(sdp)) {
            throw new IllegalArgumentException("Remote SDP must contain bounded H.264 video.");
        }
    }

    /** Rejects an answer that leaves a non-H.264 video codec negotiable. */
    public static void requireOnlyH264VideoCodec(String sdp) {
        if (!hasOnlyH264VideoCodec(sdp)) {
            throw new IllegalArgumentException("Negotiated video must contain only H.264.");
        }
    }

    private static boolean isH264MappingForVideoPayload(String line, Set<String> videoPayloads) {
        int colon = line.indexOf(':');
        int separator = line.indexOf(' ', colon + 1);
        if (colon < 0 || separator < 0) {
            return false;
        }
        String payload = line.substring(colon + 1, separator).trim();
        String encoding = line.substring(separator + 1).trim().toUpperCase(Locale.ROOT);
        return videoPayloads.contains(payload) && encoding.equals("H264/90000");
    }

    private static boolean isPayloadType(String value) {
        if (value.isEmpty() || value.length() > 3) {
            return false;
        }
        try {
            int payload = Integer.parseInt(value);
            return payload >= 0 && payload <= 127;
        } catch (NumberFormatException ignored) {
            return false;
        }
    }

    private static boolean isActivePort(String value) {
        int slash = value.indexOf('/');
        String portText = slash < 0 ? value : value.substring(0, slash);
        try {
            int port = Integer.parseInt(portText);
            return port > 0 && port <= 65535;
        } catch (NumberFormatException ignored) {
            return false;
        }
    }
}
