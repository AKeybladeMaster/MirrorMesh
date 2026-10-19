package com.mirrormesh.tv;

/** JDK-only checks for the receiver's H.264 offer gate. */
public final class H264SdpPolicyTest {
    private H264SdpPolicyTest() {}

    public static void main(String[] args) {
        acceptsH264AdvertisedInTheVideoMediaSection();
        acceptsAnH264OnlyAnswer();
        rejectsOffersWithoutH264Video();
        rejectsDisabledVideoMediaSections();
        rejectsAnswersThatLeaveAnotherVideoCodecNegotiable();
        ignoresH264AdvertisedOnlyForAudio();
        rejectsAmbiguousMultipleVideoSections();
        rejectsOversizedOrMissingOffers();
        rejectsOffersOversizedInUtf8Bytes();
        System.out.println("Android TV H.264 offer policy checks passed.");
    }

    private static void acceptsH264AdvertisedInTheVideoMediaSection() {
        String offer = "v=0\r\n"
                + "o=- 1 1 IN IP4 127.0.0.1\r\n"
                + "s=-\r\n"
                + "t=0 0\r\n"
                + "m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n"
                + "a=rtpmap:111 opus/48000/2\r\n"
                + "m=video 9 UDP/TLS/RTP/SAVPF 96 97\r\n"
                + "a=rtpmap:96 VP8/90000\r\n"
                + "a=rtpmap:97 H264/90000\r\n"
                + "a=fmtp:97 packetization-mode=1;profile-level-id=42e01f\r\n";
        expect(true, H264SdpPolicy.hasH264VideoCodec(offer), "H.264 video offer");
    }

    private static void rejectsOffersWithoutH264Video() {
        String offer = "v=0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96\r\na=rtpmap:96 VP8/90000\r\n";
        expect(false, H264SdpPolicy.hasH264VideoCodec(offer), "VP8-only video offer");
    }

    private static void rejectsDisabledVideoMediaSections() {
        String offer = "v=0\r\nm=video 0 UDP/TLS/RTP/SAVPF 97\r\n"
                + "a=rtpmap:97 H264/90000\r\n";
        expect(false, H264SdpPolicy.hasH264VideoCodec(offer), "disabled video section");
    }

    private static void acceptsAnH264OnlyAnswer() {
        String answer = "v=0\r\nm=video 9 UDP/TLS/RTP/SAVPF 97\r\n"
                + "a=rtpmap:97 H264/90000\r\n";
        expect(true, H264SdpPolicy.hasOnlyH264VideoCodec(answer), "H.264-only answer");
    }

    private static void rejectsAnswersThatLeaveAnotherVideoCodecNegotiable() {
        String answer = "v=0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96 97\r\n"
                + "a=rtpmap:96 VP8/90000\r\n"
                + "a=rtpmap:97 H264/90000\r\n";
        expect(true, H264SdpPolicy.hasH264VideoCodec(answer), "H.264 offered with VP8");
        expect(false, H264SdpPolicy.hasOnlyH264VideoCodec(answer), "mixed-codec answer");
    }

    private static void ignoresH264AdvertisedOnlyForAudio() {
        String offer = "v=0\r\n"
                + "m=audio 9 UDP/TLS/RTP/SAVPF 96\r\n"
                + "a=rtpmap:96 H264/90000\r\n"
                + "m=video 9 UDP/TLS/RTP/SAVPF 97\r\n"
                + "a=rtpmap:97 VP8/90000\r\n";
        expect(false, H264SdpPolicy.hasH264VideoCodec(offer), "audio-only H.264 mapping");
    }

    private static void rejectsAmbiguousMultipleVideoSections() {
        String offer = "v=0\r\n"
                + "m=video 9 UDP/TLS/RTP/SAVPF 96\r\n"
                + "a=rtpmap:96 H264/90000\r\n"
                + "m=video 9 UDP/TLS/RTP/SAVPF 97\r\n"
                + "a=rtpmap:97 VP8/90000\r\n";
        expect(false, H264SdpPolicy.hasH264VideoCodec(offer), "multiple video sections");
    }

    private static void rejectsOversizedOrMissingOffers() {
        expect(false, H264SdpPolicy.hasH264VideoCodec(null), "missing offer");
        expect(false, H264SdpPolicy.hasH264VideoCodec("x".repeat(H264SdpPolicy.MAX_SDP_BYTES + 1)),
                "oversized offer");
    }

    private static void rejectsOffersOversizedInUtf8Bytes() {
        String offer = "v=0\r\nm=video 9 UDP/TLS/RTP/SAVPF 96\r\n"
                + "a=rtpmap:96 H264/90000\r\n"
                + "a=x-" + "\u20ac".repeat(H264SdpPolicy.MAX_SDP_BYTES / 3 + 1);
        expect(false, H264SdpPolicy.hasH264VideoCodec(offer), "UTF-8 oversized offer");
    }

    private static void expect(boolean expected, boolean actual, String label) {
        if (expected != actual) {
            throw new AssertionError(label + ": expected " + expected + ", got " + actual);
        }
    }
}
