package com.mirrormesh.tv;

import android.content.Context;
import java.util.ArrayList;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import org.junit.Assume;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.webrtc.EglBase;
import org.webrtc.IceCandidate;
import org.webrtc.MediaConstraints;
import org.webrtc.PeerConnection;
import org.webrtc.PeerConnectionFactory;
import org.webrtc.PlatformSoftwareVideoDecoderFactory;
import org.webrtc.RtpCapabilities;
import org.webrtc.RtpTransceiver;
import org.webrtc.SdpObserver;
import org.webrtc.SessionDescription;
import org.webrtc.SoftwareVideoDecoderFactory;
import org.webrtc.SoftwareVideoEncoderFactory;
import org.webrtc.SurfaceTextureHelper;
import org.webrtc.SurfaceViewRenderer;
import org.webrtc.VideoCodecInfo;
import org.webrtc.VideoSource;
import org.webrtc.VideoTrack;
import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

/** Exercises the production TV receiver with real native libwebrtc and generated video. */
@RunWith(AndroidJUnit4.class)
public final class TvWebRtcLoopbackIntegrationTest {
    private static final int WIDTH = 320;
    private static final int HEIGHT = 180;
    private static final int FRAMES_PER_SECOND = 15;
    private static final long CONNECTION_TIMEOUT_SECONDS = 20;

    @Test
    public void approvedReceiverGetsDecodedH264FramesAndCapturesScreenshot() throws Exception {
        Context context = InstrumentationRegistry.getInstrumentation().getTargetContext();
        TvWebRtcReceiver.initializeFactory(context);

        SoftwareVideoEncoderFactory senderEncoderFactory = new SoftwareVideoEncoderFactory();
        PlatformSoftwareVideoDecoderFactory receiverDecoderFactory =
                new PlatformSoftwareVideoDecoderFactory(null);
        Assume.assumeTrue(
                "Pinned libwebrtc does not include a software H.264 encoder",
                supportsH264(senderEncoderFactory.getSupportedCodecs()));
        Assume.assumeTrue(
                "Emulator does not expose a software H.264 MediaCodec decoder",
                supportsH264(receiverDecoderFactory.getSupportedCodecs()));

        EglBase senderEgl = EglBase.create();
        SurfaceTextureHelper surfaceTextureHelper = SurfaceTextureHelper.create(
                "mirrormesh-test-capture", senderEgl.getEglBaseContext());
        PeerConnectionFactory senderFactory = PeerConnectionFactory.builder()
                .setVideoEncoderFactory(senderEncoderFactory)
                .setVideoDecoderFactory(new SoftwareVideoDecoderFactory())
                .createPeerConnectionFactory();
        SyntheticVideoCapturer capturer = new SyntheticVideoCapturer();
        VideoSource source = senderFactory.createVideoSource(false);
        VideoTrack track = senderFactory.createVideoTrack("mm-synthetic-h264", source);
        CountDownLatch frameReceived = new CountDownLatch(1);
        CountDownLatch screenshotCaptured = new CountDownLatch(1);
        CountDownLatch connectionEstablished = new CountDownLatch(1);
        AtomicInteger receivedWidth = new AtomicInteger();
        AtomicInteger receivedHeight = new AtomicInteger();
        AtomicReference<byte[]> screenshot = new AtomicReference<>();
        AtomicReference<String> receiverFailure = new AtomicReference<>();
        AtomicReference<SurfaceViewRenderer> rendererReference = new AtomicReference<>();
        AtomicReference<TvWebRtcReceiver> receiverReference = new AtomicReference<>();
        AtomicReference<LoopbackApprovedSignalingChannel> signalingReference =
                new AtomicReference<>();
        AtomicReference<PeerConnection> senderReference = new AtomicReference<>();

        PeerConnection sender = senderFactory.createPeerConnection(
                new PeerConnection.RTCConfiguration(new ArrayList<>()),
                new PeerConnection.Observer() {
                    @Override
                    public void onIceCandidate(IceCandidate candidate) {
                        LoopbackApprovedSignalingChannel signaling = signalingReference.get();
                        if (signaling != null) {
                            signaling.onSenderIceCandidate(candidate);
                        }
                    }

                    @Override
                    public void onSignalingChange(PeerConnection.SignalingState state) {}

                    @Override
                    public void onIceConnectionChange(PeerConnection.IceConnectionState state) {}

                    @Override
                    public void onIceConnectionReceivingChange(boolean receiving) {}

                    @Override
                    public void onIceGatheringChange(PeerConnection.IceGatheringState state) {}

                    @Override
                    public void onIceCandidatesRemoved(IceCandidate[] candidates) {}

                    @Override
                    public void onAddStream(org.webrtc.MediaStream stream) {}

                    @Override
                    public void onRemoveStream(org.webrtc.MediaStream stream) {}

                    @Override
                    public void onDataChannel(org.webrtc.DataChannel dataChannel) {}

                    @Override
                    public void onRenegotiationNeeded() {}

                    @Override
                    public void onTrack(RtpTransceiver transceiver) {}

                    @Override
                    public void onConnectionChange(PeerConnection.PeerConnectionState state) {}
                });
        assertNotNull("libwebrtc sender peer was not created", sender);
        senderReference.set(sender);

        RtpTransceiver senderVideo = sender.addTransceiver(
                track,
                new RtpTransceiver.RtpTransceiverInit(
                        RtpTransceiver.RtpTransceiverDirection.SEND_ONLY));
        assertNotNull("libwebrtc send-only video transceiver was not created", senderVideo);
        ArrayList<RtpCapabilities.CodecCapability> h264SendCodecs = new ArrayList<>();
        RtpCapabilities senderCapabilities = senderFactory.getRtpSenderCapabilities(
                org.webrtc.MediaStreamTrack.MediaType.MEDIA_TYPE_VIDEO);
        for (RtpCapabilities.CodecCapability codec : senderCapabilities.codecs) {
            if (codec != null && codec.mimeType != null
                    && codec.mimeType.equalsIgnoreCase("video/H264")) {
                h264SendCodecs.add(codec);
            }
        }
        assertFalse("PeerConnectionFactory does not advertise H.264 for the test sender",
                h264SendCodecs.isEmpty());
        senderVideo.setCodecPreferences(h264SendCodecs);

        LoopbackApprovedSignalingChannel signaling =
                new LoopbackApprovedSignalingChannel(sender);
        signalingReference.set(signaling);
        capturer.initialize(
                surfaceTextureHelper,
                context,
                source.getCapturerObserver());

        TvReceiverState receiverState = new TvReceiverState();
        assertTrue(receiverState.showAuthenticatedRequest());
        assertTrue(receiverState.approve());
        InstrumentationRegistry.getInstrumentation().runOnMainSync(
                () -> rendererReference.set(new SurfaceViewRenderer(context)));
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() ->
                receiverReference.set(TvWebRtcReceiver.createWithDecoderFactoryForTest(
                        context,
                        rendererReference.get(),
                        new TvWebRtcReceiver.Listener() {
                            @Override
                            public void onStatus(String status) {
                                if (status.toLowerCase(java.util.Locale.ROOT)
                                        .contains("connected")) {
                                    connectionEstablished.countDown();
                                }
                            }

                            @Override
                            public void onFrameReceived(int width, int height, long frameCount) {
                                receivedWidth.set(width);
                                receivedHeight.set(height);
                                frameReceived.countDown();
                            }

                            @Override
                            public void onFirstFrameRendered() {}

                            @Override
                            public void onScreenshotCaptured(byte[] jpeg, int width, int height) {
                                screenshot.set(jpeg);
                                screenshotCaptured.countDown();
                            }

                            @Override
                            public void onScreenshotFailed() {
                                receiverFailure.compareAndSet(null, "screenshot_failed");
                                screenshotCaptured.countDown();
                            }

                            @Override
                            public void onFailure(String code) {
                                receiverFailure.compareAndSet(null, code);
                            }

                            @Override
                            public void onRevoked() {
                                receiverFailure.compareAndSet(null, "unexpected_revoke");
                            }
                        },
                        receiverState,
                        receiverDecoderFactory)));
        TvWebRtcReceiver receiver = receiverReference.get();

        try {
            SessionDescription offer = createOffer(sender);
            setDescription(sender, offer);
            receiver.start(signaling, offer.description);
            capturer.startCapture(WIDTH, HEIGHT, FRAMES_PER_SECOND);

            assertTrue(
                    "ICE did not connect; receiver failure=" + receiverFailure.get(),
                    connectionEstablished.await(CONNECTION_TIMEOUT_SECONDS, TimeUnit.SECONDS));
            assertTrue(
                    "no decoded frame reached the TV receiver; state=" + receiverState.phase()
                            + ", failure=" + receiverFailure.get(),
                    frameReceived.await(CONNECTION_TIMEOUT_SECONDS, TimeUnit.SECONDS));
            assertTrue("decoded width was invalid", receivedWidth.get() > 0);
            assertTrue("decoded height was invalid", receivedHeight.get() > 0);
            assertTrue("receiver did not accept the explicit screenshot request",
                    receiver.requestScreenshotCapture());
            assertTrue("no screenshot was produced from a received frame",
                    screenshotCaptured.await(10, TimeUnit.SECONDS));
            assertNotNull("received frame screenshot is empty", screenshot.get());
            assertTrue("received frame screenshot is too short", screenshot.get().length > 4);
            assertArrayEquals(
                    "received frame screenshot has no JPEG marker",
                    new byte[] {(byte) 0xff, (byte) 0xd8},
                    new byte[] {screenshot.get()[0], screenshot.get()[1]});
            signaling.answerApplied().get(10, TimeUnit.SECONDS);
        } finally {
            capturer.dispose();
            if (receiverReference.get() != null) {
                InstrumentationRegistry.getInstrumentation().runOnMainSync(
                        () -> receiverReference.get().close());
            }
            if (senderReference.get() != null) {
                senderReference.get().close();
                senderReference.get().dispose();
            }
            track.dispose();
            source.dispose();
            surfaceTextureHelper.dispose();
            senderFactory.dispose();
            senderEgl.release();
        }
    }

    private static SessionDescription createOffer(PeerConnection peer) throws Exception {
        CompletableFuture<SessionDescription> created = new CompletableFuture<>();
        peer.createOffer(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription description) {
                created.complete(description);
            }

            @Override
            public void onSetSuccess() {}

            @Override
            public void onCreateFailure(String error) {
                created.completeExceptionally(new IllegalStateException(error));
            }

            @Override
            public void onSetFailure(String error) {
                created.completeExceptionally(new IllegalStateException(error));
            }
        }, new MediaConstraints());
        return created.get(10, TimeUnit.SECONDS);
    }

    private static void setDescription(PeerConnection peer, SessionDescription description)
            throws Exception {
        CompletableFuture<Void> set = new CompletableFuture<>();
        peer.setLocalDescription(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription value) {}

            @Override
            public void onSetSuccess() {
                set.complete(null);
            }

            @Override
            public void onCreateFailure(String error) {
                set.completeExceptionally(new IllegalStateException(error));
            }

            @Override
            public void onSetFailure(String error) {
                set.completeExceptionally(new IllegalStateException(error));
            }
        }, description);
        set.get(10, TimeUnit.SECONDS);
    }

    private static boolean supportsH264(VideoCodecInfo[] codecs) {
        if (codecs == null) {
            return false;
        }
        for (VideoCodecInfo codec : codecs) {
            if (codec != null && codec.name != null
                    && codec.name.equalsIgnoreCase("H264")) {
                return true;
            }
        }
        return false;
    }
}
