package com.mirrormesh.tv;

import android.content.Context;
import android.graphics.ImageFormat;
import android.graphics.Rect;
import android.graphics.YuvImage;
import java.io.ByteArrayOutputStream;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Queue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.atomic.AtomicBoolean;
import org.webrtc.AudioTrack;
import org.webrtc.EglBase;
import org.webrtc.HardwareVideoDecoderFactory;
import org.webrtc.IceCandidate;
import org.webrtc.MediaStream;
import org.webrtc.MediaStreamTrack;
import org.webrtc.MediaConstraints;
import org.webrtc.PeerConnection;
import org.webrtc.PeerConnectionFactory;
import org.webrtc.RtpCapabilities;
import org.webrtc.RtpReceiver;
import org.webrtc.RtpTransceiver;
import org.webrtc.SdpObserver;
import org.webrtc.SessionDescription;
import org.webrtc.SurfaceViewRenderer;
import org.webrtc.VideoCodecInfo;
import org.webrtc.VideoDecoderFactory;
import org.webrtc.VideoFrame;
import org.webrtc.VideoSink;
import org.webrtc.VideoTrack;
import org.webrtc.RendererCommon;

/** Native receive-only WebRTC engine. It requires a verified, explicitly approved session. */
public final class TvWebRtcReceiver implements AutoCloseable {
    private static final int MAX_REMOTE_ICE_CANDIDATES = 64;
    private static final int MAX_CANDIDATE_CHARS = 4096;
    private static final int MAX_MEDIA_ID_CHARS = 256;
    private static final int MAX_ICE_SERVERS = 8;
    private static final int JPEG_QUALITY = 90;
    private static final int MAX_JPEG_BYTES = 32 * 1024 * 1024;
    private static final Object FACTORY_LOCK = new Object();
    private static boolean factoryInitialized;

    public interface Listener {
        void onStatus(String status);

        void onFrameReceived(int width, int height, long frameCount);

        void onFirstFrameRendered();

        void onScreenshotCaptured(byte[] jpeg, int width, int height);

        void onScreenshotFailed();

        void onFailure(String code);

        void onRevoked();
    }

    public static final class Diagnostics {
        public final String sessionPhase;
        public final String peerFingerprint;
        public final String connectionState;
        public final long receivedFrameCount;
        public final int frameWidth;
        public final int frameHeight;

        private Diagnostics(
                String sessionPhase,
                String peerFingerprint,
                String connectionState,
                long receivedFrameCount,
                int frameWidth,
                int frameHeight) {
            this.sessionPhase = sessionPhase;
            this.peerFingerprint = peerFingerprint;
            this.connectionState = connectionState;
            this.receivedFrameCount = receivedFrameCount;
            this.frameWidth = frameWidth;
            this.frameHeight = frameHeight;
        }
    }

    private static final class RemoteCandidate {
        final String mediaId;
        final int mediaLineIndex;
        final String candidate;

        RemoteCandidate(String mediaId, int mediaLineIndex, String candidate) {
            this.mediaId = mediaId;
            this.mediaLineIndex = mediaLineIndex;
            this.candidate = candidate;
        }
    }

    private static final class BoundedJpegOutputStream extends ByteArrayOutputStream {
        @Override
        public synchronized void write(int value) {
            requireCapacity(1);
            super.write(value);
        }

        @Override
        public synchronized void write(byte[] bytes, int offset, int length) {
            requireCapacity(length);
            super.write(bytes, offset, length);
        }

        private void requireCapacity(int incomingBytes) {
            if (incomingBytes < 0 || incomingBytes > MAX_JPEG_BYTES - count) {
                throw new IllegalStateException("JPEG output exceeds the screenshot limit.");
            }
        }
    }

    private final EglBase eglBase;
    private final PeerConnectionFactory factory;
    private final VideoDecoderFactory decoderFactory;
    private final SurfaceViewRenderer renderer;
    private final Listener listener;
    private final TvReceiverState state;
    private final AtomicBoolean closed = new AtomicBoolean();
    private final Object candidateLock = new Object();
    private final Queue<RemoteCandidate> pendingRemoteCandidates = new ArrayDeque<>();
    private final Queue<IceCandidate> pendingLocalCandidates = new ArrayDeque<>();
    private final AtomicBoolean screenshotRequested = new AtomicBoolean();
    private final ExecutorService screenshotExecutor = Executors.newSingleThreadExecutor(task -> {
        Thread thread = new Thread(task, "mirrormesh-tv-screenshot");
        thread.setDaemon(true);
        return thread;
    });

    private volatile ApprovedSignalingChannel signaling;
    private volatile PeerConnection peerConnection;
    private volatile VideoTrack remoteVideoTrack;
    private volatile VideoSink videoSink;
    private volatile boolean remoteDescriptionSet;
    private volatile boolean answerSent;
    private volatile int lastFrameWidth;
    private volatile int lastFrameHeight;
    private volatile long receivedFrameCount;
    private volatile PeerConnection.PeerConnectionState connectionState =
            PeerConnection.PeerConnectionState.NEW;

    private TvWebRtcReceiver(
            Context context,
            SurfaceViewRenderer renderer,
            Listener listener,
            TvReceiverState state,
            VideoDecoderFactory testDecoderFactory) {
        this.renderer = renderer;
        this.listener = listener;
        this.state = state;
        Context applicationContext = context.getApplicationContext();

        initializeFactory(applicationContext);

        eglBase = EglBase.create();
        decoderFactory = testDecoderFactory == null
                ? new HardwareVideoDecoderFactory(eglBase.getEglBaseContext())
                : testDecoderFactory;
        if (testDecoderFactory == null
                && !supportsHardwareH264(decoderFactory.getSupportedCodecs())) {
            eglBase.release();
            throw new IllegalStateException("No hardware H.264 decoder is available.");
        }

        boolean rendererInitialized = false;
        PeerConnectionFactory createdFactory = null;
        try {
            renderer.init(eglBase.getEglBaseContext(), new RendererCommon.RendererEvents() {
                @Override
                public void onFirstFrameRendered() {
                    if (closed.get()) {
                        return;
                    }
                    state.onFirstFramePresented();
                    listener.onFirstFrameRendered();
                    listener.onStatus("Receiving H.264 video");
                }

                @Override
                public void onFrameResolutionChanged(int width, int height, int rotation) {
                    if (!closed.get()) {
                        listener.onStatus("Video surface: " + width + " × " + height);
                    }
                }
            });
            rendererInitialized = true;
            renderer.setScalingType(RendererCommon.ScalingType.SCALE_ASPECT_FIT);
            renderer.setEnableHardwareScaler(true);
            renderer.setMirror(false);
            createdFactory = PeerConnectionFactory.builder()
                    .setVideoDecoderFactory(decoderFactory)
                    .createPeerConnectionFactory();
        } catch (RuntimeException error) {
            if (createdFactory != null) {
                createdFactory.dispose();
            }
            if (rendererInitialized) {
                renderer.release();
            }
            eglBase.release();
            throw error;
        }
        factory = createdFactory;
    }

    static void initializeFactory(Context context) {
        synchronized (FACTORY_LOCK) {
            if (!factoryInitialized) {
                PeerConnectionFactory.initialize(
                        PeerConnectionFactory.InitializationOptions
                                .builder(context.getApplicationContext())
                                .createInitializationOptions());
                factoryInitialized = true;
            }
        }
    }

    public static TvWebRtcReceiver create(
            Context context,
            SurfaceViewRenderer renderer,
            Listener listener,
            TvReceiverState state) {
        if (context == null || renderer == null || listener == null || state == null) {
            throw new IllegalArgumentException("TV receiver dependencies are required.");
        }
        return new TvWebRtcReceiver(context, renderer, listener, state, null);
    }

    static TvWebRtcReceiver createWithDecoderFactoryForTest(
            Context context,
            SurfaceViewRenderer renderer,
            Listener listener,
            TvReceiverState state,
            VideoDecoderFactory decoderFactory) {
        if (context == null || renderer == null || listener == null || state == null
                || decoderFactory == null) {
            throw new IllegalArgumentException("TV receiver test dependencies are required.");
        }
        return new TvWebRtcReceiver(context, renderer, listener, state, decoderFactory);
    }

    public void start(ApprovedSignalingChannel approvedSignaling, String remoteOfferSdp) {
        if (approvedSignaling == null || !approvedSignaling.isAuthenticatedAndApproved()) {
            throw new SecurityException("A verified, approved signaling session is required.");
        }
        H264SdpPolicy.requireH264VideoCodec(remoteOfferSdp);
        if (!state.beginConnecting()) {
            throw new IllegalStateException("The receiver must be explicitly approved first.");
        }
        signaling = approvedSignaling;
        attachSignalingListeners(approvedSignaling);
        createPeerConnectionAndApplyOffer(remoteOfferSdp);
    }

    /** Returns metadata only; encoded or decoded pixels are never copied for diagnostics. */
    public Diagnostics diagnostics() {
        ApprovedSignalingChannel channel = signaling;
        return new Diagnostics(
                state.phase().name(),
                channel == null ? "" : channel.peerFingerprint(),
                connectionState.name(),
                receivedFrameCount,
                lastFrameWidth,
                lastFrameHeight);
    }

    /** Captures only the next decoded frame after an explicit user action. */
    public boolean requestScreenshotCapture() {
        return !closed.get() && receivedFrameCount > 0
                && screenshotRequested.compareAndSet(false, true);
    }

    public void cancelScreenshotCapture() {
        screenshotRequested.set(false);
    }

    private void attachSignalingListeners(ApprovedSignalingChannel channel) {
        channel.setRemoteIceCandidateListener(this::addRemoteIceCandidate);
        channel.setRemoteStartListener(sourceId -> {
            if (!closed.get() && receivedFrameCount == 0) {
                listener.onStatus("Sender reports capture started");
            }
        });
        channel.setRemoteOfferListener(this::reconnectWithOffer);
        channel.setRevokedListener(() -> {
            if (state.revoke()) {
                listener.onRevoked();
            }
            close();
        });
        channel.setRemoteStopListener(this::stopByRemoteSender);
        channel.setFailureListener(this::fail);
    }

    private void stopByRemoteSender() {
        if (closed.get()) {
            return;
        }
        if (state.disconnect()) {
            listener.onStatus("Sender disconnected");
        }
        close();
    }

    private void reconnectWithOffer(String remoteOfferSdp) {
        if (closed.get() || signaling == null || !signaling.isAuthenticatedAndApproved()) {
            close();
            return;
        }
        try {
            H264SdpPolicy.requireH264VideoCodec(remoteOfferSdp);
        } catch (IllegalArgumentException error) {
            fail("unsupported_remote_offer");
            return;
        }
        if (!state.beginReconnect()) {
            fail("unexpected_reconnect_offer");
            return;
        }
        resetPeerConnection();
        remoteDescriptionSet = false;
        answerSent = false;
        synchronized (candidateLock) {
            pendingRemoteCandidates.clear();
            pendingLocalCandidates.clear();
        }
        createPeerConnectionAndApplyOffer(remoteOfferSdp);
    }

    private void createPeerConnectionAndApplyOffer(String remoteOfferSdp) {
        ApprovedSignalingChannel channel = signaling;
        if (closed.get() || channel == null || !channel.isAuthenticatedAndApproved()) {
            fail("session_no_longer_approved");
            return;
        }

        List<PeerConnection.IceServer> configuredIceServers = channel.iceServers();
        if (configuredIceServers == null || configuredIceServers.size() > MAX_ICE_SERVERS) {
            fail("invalid_ice_configuration");
            return;
        }

        PeerConnection.RTCConfiguration configuration =
                new PeerConnection.RTCConfiguration(new ArrayList<>(configuredIceServers));
        PeerConnection connection = factory.createPeerConnection(configuration, new ReceiverObserver());
        if (connection == null) {
            fail("peer_connection_unavailable");
            return;
        }
        peerConnection = connection;
        if (!configureReceiveOnlyH264(connection)) {
            fail("hardware_h264_unavailable");
            return;
        }
        listener.onStatus("Authenticated session approved; negotiating H.264");
        connection.setRemoteDescription(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription description) {}

            @Override
            public void onSetSuccess() {
                remoteDescriptionSet = true;
                drainRemoteCandidates();
                createAndSendAnswer(connection, channel);
            }

            @Override
            public void onCreateFailure(String error) {
                fail("remote_offer_rejected");
            }

            @Override
            public void onSetFailure(String error) {
                fail("remote_offer_rejected");
            }
        }, new SessionDescription(SessionDescription.Type.OFFER, remoteOfferSdp));
    }

    private boolean configureReceiveOnlyH264(PeerConnection connection) {
        try {
            RtpTransceiver transceiver = connection.addTransceiver(
                    MediaStreamTrack.MediaType.MEDIA_TYPE_VIDEO,
                    new RtpTransceiver.RtpTransceiverInit(
                            RtpTransceiver.RtpTransceiverDirection.RECV_ONLY));
            RtpCapabilities receiverCapabilities = factory.getRtpReceiverCapabilities(
                    MediaStreamTrack.MediaType.MEDIA_TYPE_VIDEO);
            if (transceiver == null || receiverCapabilities == null
                    || receiverCapabilities.codecs == null) {
                return false;
            }

            ArrayList<RtpCapabilities.CodecCapability> h264Capabilities = new ArrayList<>();
            for (RtpCapabilities.CodecCapability codec : receiverCapabilities.codecs) {
                if (codec != null && codec.mimeType != null
                        && codec.mimeType.equalsIgnoreCase("video/H264")) {
                    h264Capabilities.add(codec);
                }
            }
            if (h264Capabilities.isEmpty()) {
                return false;
            }

            // The factory uses HardwareVideoDecoderFactory only. Restricting
            // this receive-only transceiver prevents VP8/VP9/AV1 negotiation.
            transceiver.setCodecPreferences(h264Capabilities);
            return true;
        } catch (RuntimeException error) {
            return false;
        }
    }

    private void createAndSendAnswer(
            PeerConnection connection,
            ApprovedSignalingChannel channel) {
        connection.createAnswer(new SdpObserver() {
            @Override
            public void onCreateSuccess(SessionDescription answer) {
                try {
                    H264SdpPolicy.requireOnlyH264VideoCodec(answer.description);
                } catch (IllegalArgumentException error) {
                    fail("h264_answer_unavailable");
                    return;
                }
                connection.setLocalDescription(new SdpObserver() {
                    @Override
                    public void onCreateSuccess(SessionDescription description) {}

                    @Override
                    public void onSetSuccess() {
                        if (closed.get() || !channel.isAuthenticatedAndApproved()) {
                            fail("session_no_longer_approved");
                            return;
                        }
                        try {
                            channel.sendAnswer(answer.description);
                            answerSent = true;
                            drainLocalCandidates(channel);
                            listener.onStatus("H.264 negotiated; waiting for video");
                        } catch (RuntimeException error) {
                            fail("answer_delivery_failed");
                        }
                    }

                    @Override
                    public void onCreateFailure(String error) {
                        fail("local_description_failed");
                    }

                    @Override
                    public void onSetFailure(String error) {
                        fail("local_description_failed");
                    }
                }, answer);
            }

            @Override
            public void onSetSuccess() {}

            @Override
            public void onCreateFailure(String error) {
                fail("answer_creation_failed");
            }

            @Override
            public void onSetFailure(String error) {
                fail("answer_creation_failed");
            }
        }, new MediaConstraints());
    }

    private void addRemoteIceCandidate(String mediaId, int mediaLineIndex, String candidateText) {
        ApprovedSignalingChannel channel = signaling;
        if (closed.get() || channel == null || !channel.isAuthenticatedAndApproved()) {
            close();
            return;
        }
        if (!isValidCandidate(mediaId, mediaLineIndex, candidateText)) {
            fail("invalid_remote_candidate");
            return;
        }

        RemoteCandidate candidate = new RemoteCandidate(mediaId, mediaLineIndex, candidateText);
        synchronized (candidateLock) {
            if (!remoteDescriptionSet) {
                if (pendingRemoteCandidates.size() >= MAX_REMOTE_ICE_CANDIDATES) {
                    fail("remote_candidate_queue_full");
                    return;
                }
                pendingRemoteCandidates.add(candidate);
                return;
            }
        }
        addCandidateToPeer(candidate);
    }

    private void drainRemoteCandidates() {
        synchronized (candidateLock) {
            while (!pendingRemoteCandidates.isEmpty()) {
                addCandidateToPeer(pendingRemoteCandidates.remove());
            }
        }
    }

    private void addCandidateToPeer(RemoteCandidate candidate) {
        PeerConnection connection = peerConnection;
        if (connection == null || closed.get()) {
            return;
        }
        boolean accepted = connection.addIceCandidate(new IceCandidate(
                candidate.mediaId, candidate.mediaLineIndex, candidate.candidate));
        if (!accepted) {
            fail("remote_candidate_rejected");
        }
    }

    private void sendOrQueueLocalCandidate(IceCandidate candidate) {
        ApprovedSignalingChannel channel = signaling;
        if (closed.get() || channel == null || !channel.isAuthenticatedAndApproved()) {
            close();
            return;
        }
        if (!isValidCandidate(candidate.sdpMid, candidate.sdpMLineIndex, candidate.sdp)) {
            fail("invalid_local_candidate");
            return;
        }
        synchronized (candidateLock) {
            if (!answerSent) {
                if (pendingLocalCandidates.size() >= MAX_REMOTE_ICE_CANDIDATES) {
                    fail("local_candidate_queue_full");
                    return;
                }
                pendingLocalCandidates.add(candidate);
                return;
            }
        }
        sendLocalCandidate(channel, candidate);
    }

    private void drainLocalCandidates(ApprovedSignalingChannel channel) {
        synchronized (candidateLock) {
            while (!pendingLocalCandidates.isEmpty()) {
                sendLocalCandidate(channel, pendingLocalCandidates.remove());
            }
        }
    }

    private void sendLocalCandidate(ApprovedSignalingChannel channel, IceCandidate candidate) {
        try {
            if (channel.isAuthenticatedAndApproved()) {
                channel.sendLocalIceCandidate(
                        candidate.sdpMid, candidate.sdpMLineIndex, candidate.sdp);
            } else {
                fail("session_no_longer_approved");
            }
        } catch (RuntimeException error) {
            fail("candidate_delivery_failed");
        }
    }

    private void attachRemoteVideoTrack(MediaStreamTrack track) {
        if (closed.get()) {
            return;
        }
        if (!(track instanceof VideoTrack videoTrack)) {
            if (track instanceof AudioTrack) {
                track.setEnabled(false);
            }
            return;
        }
        if (remoteVideoTrack == videoTrack) {
            return;
        }
        VideoTrack previous = remoteVideoTrack;
        VideoSink previousSink = videoSink;
        if (previous != null && previousSink != null) {
            previous.removeSink(previousSink);
        }

        VideoSink sink = frame -> {
            if (closed.get()) {
                return;
            }
            int width = frame.getBuffer().getWidth();
            int height = frame.getBuffer().getHeight();
            lastFrameWidth = width;
            lastFrameHeight = height;
            long count = ++receivedFrameCount;
            listener.onFrameReceived(width, height, count);
            renderer.onFrame(frame);
            if (screenshotRequested.compareAndSet(true, false)) {
                queueScreenshotEncoding(frame);
            }
        };
        remoteVideoTrack = videoTrack;
        videoSink = sink;
        videoTrack.addSink(sink);
    }

    private void queueScreenshotEncoding(VideoFrame frame) {
        VideoFrame.I420Buffer i420 = null;
        byte[] nv21;
        int width;
        int height;
        try {
            i420 = frame.getBuffer().toI420();
            width = i420.getWidth();
            height = i420.getHeight();
            nv21 = TvScreenshotFrame.toNv21(
                    width,
                    height,
                    i420.getDataY(),
                    i420.getStrideY(),
                    i420.getDataU(),
                    i420.getStrideU(),
                    i420.getDataV(),
                    i420.getStrideV());
        } catch (RuntimeException error) {
            if (!closed.get()) {
                listener.onScreenshotFailed();
            }
            return;
        } finally {
            if (i420 != null) {
                i420.release();
            }
        }

        try {
            screenshotExecutor.execute(() -> {
                if (closed.get()) {
                    return;
                }
                try {
                    byte[] jpeg = encodeJpeg(nv21, width, height);
                    if (!closed.get()) {
                        listener.onScreenshotCaptured(jpeg, width, height);
                    }
                } catch (RuntimeException error) {
                    if (!closed.get()) {
                        listener.onScreenshotFailed();
                    }
                }
            });
        } catch (RejectedExecutionException error) {
            if (!closed.get()) {
                listener.onScreenshotFailed();
            }
        }
    }

    private static byte[] encodeJpeg(byte[] nv21, int width, int height) {
        YuvImage image = new YuvImage(nv21, ImageFormat.NV21, width, height, null);
        BoundedJpegOutputStream output = new BoundedJpegOutputStream();
        if (!image.compressToJpeg(new Rect(0, 0, width, height), JPEG_QUALITY, output)) {
            throw new IllegalStateException("JPEG compression failed.");
        }
        byte[] jpeg = output.toByteArray();
        if (jpeg.length == 0 || jpeg.length > MAX_JPEG_BYTES) {
            throw new IllegalStateException("JPEG output exceeds the screenshot limit.");
        }
        return jpeg;
    }

    private void fail(String code) {
        if (closed.get()) {
            return;
        }
        state.fail();
        listener.onFailure(code);
        close();
    }

    private void resetPeerConnection() {
        VideoTrack track = remoteVideoTrack;
        VideoSink sink = videoSink;
        remoteVideoTrack = null;
        videoSink = null;
        if (track != null && sink != null) {
            track.removeSink(sink);
        }
        PeerConnection previous = peerConnection;
        peerConnection = null;
        if (previous != null) {
            previous.close();
            previous.dispose();
        }
    }

    private static boolean isValidCandidate(String mediaId, int mediaLineIndex, String candidate) {
        return mediaId != null
                && !mediaId.trim().isEmpty()
                && mediaId.length() <= MAX_MEDIA_ID_CHARS
                && mediaLineIndex >= 0
                && mediaLineIndex <= 15
                && candidate != null
                && !candidate.trim().isEmpty()
                && candidate.length() <= MAX_CANDIDATE_CHARS
                && candidate.startsWith("candidate:");
    }

    private static boolean supportsHardwareH264(VideoCodecInfo[] codecs) {
        if (codecs == null) {
            return false;
        }
        for (VideoCodecInfo codec : codecs) {
            if (codec != null && codec.name != null
                    && "H264".equals(codec.name.toUpperCase(Locale.ROOT))) {
                return true;
            }
        }
        return false;
    }

    private final class ReceiverObserver implements PeerConnection.Observer {
        @Override
        public void onSignalingChange(PeerConnection.SignalingState state) {}

        @Override
        public void onIceConnectionChange(PeerConnection.IceConnectionState iceState) {
            if (closed.get()) {
                return;
            }
            switch (iceState) {
                case CONNECTED, COMPLETED -> listener.onStatus("Connected; waiting for video");
                case DISCONNECTED -> {
                    TvWebRtcReceiver.this.state.onConnectionInterrupted();
                    listener.onStatus("Connection interrupted; waiting to reconnect");
                }
                case FAILED -> {
                    TvWebRtcReceiver.this.state.onConnectionInterrupted();
                    listener.onStatus("Connection failed; waiting for reconnection");
                }
                case CLOSED -> listener.onStatus("Connection closed");
                case NEW, CHECKING -> listener.onStatus("Connecting to approved sender");
            }
        }

        @Override
        public void onConnectionChange(PeerConnection.PeerConnectionState newState) {
            connectionState = newState;
        }

        @Override
        public void onIceConnectionReceivingChange(boolean receiving) {}

        @Override
        public void onIceGatheringChange(PeerConnection.IceGatheringState gatheringState) {}

        @Override
        public void onIceCandidate(IceCandidate candidate) {
            sendOrQueueLocalCandidate(candidate);
        }

        @Override
        public void onIceCandidatesRemoved(IceCandidate[] candidates) {}

        @Override
        public void onAddStream(MediaStream stream) {
            for (VideoTrack track : stream.videoTracks) {
                attachRemoteVideoTrack(track);
            }
        }

        @Override
        public void onRemoveStream(MediaStream stream) {
            VideoTrack track = remoteVideoTrack;
            VideoSink sink = videoSink;
            if (track != null && sink != null && stream.videoTracks.contains(track)) {
                track.removeSink(sink);
                remoteVideoTrack = null;
                videoSink = null;
            }
        }

        @Override
        public void onDataChannel(org.webrtc.DataChannel dataChannel) {
            dataChannel.close();
            dataChannel.dispose();
        }

        @Override
        public void onRenegotiationNeeded() {}

        @Override
        public void onTrack(RtpTransceiver transceiver) {
            RtpReceiver receiver = transceiver.getReceiver();
            attachRemoteVideoTrack(receiver.track());
        }
    }

    @Override
    public void close() {
        if (!closed.compareAndSet(false, true)) {
            return;
        }
        ApprovedSignalingChannel channel = signaling;
        signaling = null;
        if (channel != null) {
            channel.clearListeners();
            channel.close();
        }
        resetPeerConnection();
        factory.dispose();
        renderer.release();
        eglBase.release();
        screenshotRequested.set(false);
        screenshotExecutor.shutdownNow();
    }
}
