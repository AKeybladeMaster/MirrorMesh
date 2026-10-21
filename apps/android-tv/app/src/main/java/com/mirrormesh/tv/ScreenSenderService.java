package com.mirrormesh.tv;

import android.app.*;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.media.projection.MediaProjection;
import android.os.*;
import org.json.JSONObject;
import org.webrtc.*;
import java.util.*;
import java.util.concurrent.*;

/** User-consented native capture. No screen pixels cross JNI or Dart. */
public final class ScreenSenderService extends Service {
    private final ScheduledExecutorService worker = Executors.newSingleThreadScheduledExecutor();
    private EglBase egl;
    private PeerConnectionFactory factory;
    private PeerConnection peer;
    private ScreenCapturerAndroid capturer;
    private SurfaceTextureHelper texture;
    private VideoSource source;
    private VideoTrack track;
    private long requestId;
    private boolean offered, answered, started;
    private final List<IceCandidate> localIce = new ArrayList<>(), remoteIce = new ArrayList<>();
    private ScheduledFuture<?> polling, deadline;
    private volatile boolean closing;

    @Override public IBinder onBind(Intent intent) { return null; }

    @Override public int onStartCommand(Intent intent, int flags, int id) {
        if (intent == null || "stop".equals(intent.getAction())) { stopSelf(); return START_NOT_STICKY; }
        if (requestId != 0) return START_NOT_STICKY;
        requestId = intent.getLongExtra("request", 0);
        Intent projection = intent.getParcelableExtra("projection");
        if (requestId <= 0 || projection == null) { stopSelf(); return START_NOT_STICKY; }
        NotificationManager manager = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (Build.VERSION.SDK_INT >= 26) manager.createNotificationChannel(new NotificationChannel("screen-sharing", "Screen sharing", NotificationManager.IMPORTANCE_LOW));
        PendingIntent stop = PendingIntent.getService(this, 0, new Intent(this, ScreenSenderService.class).setAction("stop"), PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        Notification.Builder builder = Build.VERSION.SDK_INT >= 26 ? new Notification.Builder(this, "screen-sharing") : new Notification.Builder(this);
        Notification notification = builder.setSmallIcon(R.drawable.mm_icon).setContentTitle("MirrorMesh is sharing your screen")
                .setContentText("Everything visible in the selected screen may be shared.").setOngoing(true)
                .addAction(new Notification.Action.Builder(android.R.drawable.ic_media_pause, "Stop sharing", stop).build()).build();
        if (Build.VERSION.SDK_INT >= 29) startForeground(52, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION);
        else startForeground(52, notification);
        worker.execute(() -> {
            try { startCapture(projection); }
            catch (Exception failure) { stopSelf(); }
        });
        return START_NOT_STICKY;
    }

    private JSONObject command(String op) throws Exception {
        return SenderActivity.command(new JSONObject().put("op", op).put("id", requestId));
    }

    private void startCapture(Intent projection) throws Exception {
        if (closing) return;
        if (command("send_poll").getInt("state") != 3) throw new IllegalStateException("Receiver approval is no longer active");
        TvWebRtcReceiver.initializeFactory(getApplicationContext());
        egl = EglBase.create();
        factory = PeerConnectionFactory.builder()
                .setVideoEncoderFactory(new DefaultVideoEncoderFactory(egl.getEglBaseContext(), true, true))
                .setVideoDecoderFactory(new DefaultVideoDecoderFactory(egl.getEglBaseContext())).createPeerConnectionFactory();
        PeerConnection.RTCConfiguration configuration = new PeerConnection.RTCConfiguration(Collections.emptyList());
        configuration.sdpSemantics = PeerConnection.SdpSemantics.UNIFIED_PLAN;
        peer = factory.createPeerConnection(configuration, new PeerConnection.Observer() {
            @Override public void onSignalingChange(PeerConnection.SignalingState state) { }
            @Override public void onIceConnectionChange(PeerConnection.IceConnectionState state) { }
            @Override public void onIceConnectionReceivingChange(boolean receiving) { }
            @Override public void onIceGatheringChange(PeerConnection.IceGatheringState state) { }
            @Override public void onIceCandidate(IceCandidate candidate) { submit(() -> sendIce(candidate)); }
            @Override public void onIceCandidatesRemoved(IceCandidate[] candidates) { }
            @Override public void onAddStream(MediaStream stream) { }
            @Override public void onRemoveStream(MediaStream stream) { }
            @Override public void onDataChannel(DataChannel channel) { channel.close(); }
            @Override public void onRenegotiationNeeded() { }
            @Override public void onAddTrack(RtpReceiver receiver, MediaStream[] streams) { }
            @Override public void onConnectionChange(PeerConnection.PeerConnectionState state) {
                if (state == PeerConnection.PeerConnectionState.CONNECTED) submit(() -> {
                    if (!started) {
                        SenderActivity.command(new JSONObject().put("op", "send_start").put("id", requestId).put("source", "android-screen"));
                        started = true;
                        if (deadline != null) deadline.cancel(false);
                    }
                });
                if (state == PeerConnection.PeerConnectionState.FAILED || state == PeerConnection.PeerConnectionState.CLOSED) stopSelf();
            }
        });
        if (peer == null) throw new IllegalStateException("WebRTC unavailable");
        source = factory.createVideoSource(true);
        texture = SurfaceTextureHelper.create("MirrorMeshCapture", egl.getEglBaseContext());
        capturer = new ScreenCapturerAndroid(projection, new MediaProjection.Callback() {
            @Override public void onStop() { stopSelf(); }
        });
        capturer.initialize(texture, getApplicationContext(), source.getCapturerObserver());
        track = factory.createVideoTrack("screen", source);
        RtpTransceiver transceiver = peer.addTransceiver(track, new RtpTransceiver.RtpTransceiverInit(RtpTransceiver.RtpTransceiverDirection.SEND_ONLY));
        List<RtpCapabilities.CodecCapability> codecs = new ArrayList<>();
        for (RtpCapabilities.CodecCapability codec : factory.getRtpSenderCapabilities(MediaStreamTrack.MediaType.MEDIA_TYPE_VIDEO).codecs) {
            if ("H264".equalsIgnoreCase(codec.name)) codecs.add(codec);
        }
        if (codecs.isEmpty()) throw new IllegalStateException("H.264 encoding unavailable");
        transceiver.setCodecPreferences(codecs);
        capturer.startCapture(1280, 720, 30);
        peer.createOffer(new SdpAdapter() {
            @Override public void onCreateSuccess(SessionDescription description) {
                submit(() -> peer.setLocalDescription(new SdpAdapter() {
                    @Override public void onSetSuccess() { submit(() -> {
                        SenderActivity.command(new JSONObject().put("op", "send_offer").put("id", requestId).put("sdp", description.description));
                        offered = true;
                        for (IceCandidate candidate : new ArrayList<>(localIce)) sendIce(candidate);
                        localIce.clear();
                    }); }
                }, description));
            }
        }, new MediaConstraints());
        polling = worker.scheduleWithFixedDelay(() -> {
            try { pollEvent(); } catch (Exception failure) { stopSelf(); }
        }, 0, 100, TimeUnit.MILLISECONDS);
        deadline = worker.schedule(() -> { stopSelf(); }, 30, TimeUnit.SECONDS);
    }

    private void pollEvent() throws Exception {
        JSONObject event = command("send_event");
        switch (event.optString("kind")) {
            case "answer":
                peer.setRemoteDescription(new SdpAdapter() {
                    @Override public void onSetSuccess() { submit(() -> {
                        answered = true;
                        for (IceCandidate candidate : remoteIce) peer.addIceCandidate(candidate);
                        remoteIce.clear();
                    }); }
                }, new SessionDescription(SessionDescription.Type.ANSWER, event.getString("sdp")));
                break;
            case "ice":
                IceCandidate candidate = new IceCandidate(event.getString("mid"), event.getInt("line"), event.getString("candidate"));
                if (answered) peer.addIceCandidate(candidate);
                else if (remoteIce.size() < 64) remoteIce.add(candidate);
                else throw new IllegalStateException("Too many remote candidates");
                break;
            case "ended": stopSelf(); break;
        }
    }

    private void sendIce(IceCandidate candidate) throws Exception {
        if (!offered) {
            if (localIce.size() >= 64) throw new IllegalStateException("Too many local candidates");
            localIce.add(candidate); return;
        }
        SenderActivity.command(new JSONObject().put("op", "send_ice").put("id", requestId)
                .put("candidate", candidate.sdp).put("mid", candidate.sdpMid == null ? "0" : candidate.sdpMid).put("line", candidate.sdpMLineIndex));
    }

    private interface Operation { void run() throws Exception; }
    private void submit(Operation operation) {
        if (closing) return;
        try { worker.execute(() -> { if (!closing) try { operation.run(); } catch (Exception failure) { stopSelf(); } }); }
        catch (RejectedExecutionException ignored) { }
    }

    private class SdpAdapter implements SdpObserver {
        @Override public void onCreateSuccess(SessionDescription description) { }
        @Override public void onSetSuccess() { }
        @Override public void onCreateFailure(String message) { stopSelf(); }
        @Override public void onSetFailure(String message) { stopSelf(); }
    }

    @Override public void onDestroy() {
        closing = true;
        if (polling != null) polling.cancel(false);
        if (deadline != null) deadline.cancel(false);
        worker.execute(() -> {
            if (track != null) track.setEnabled(false);
            try { if (capturer != null) capturer.stopCapture(); } catch (RuntimeException ignored) { }
            try { if (offered) command("send_stop"); } catch (Exception ignored) { }
            if (capturer != null) capturer.dispose();
            if (peer != null) { peer.close(); peer.dispose(); }
            if (track != null) track.dispose();
            if (source != null) source.dispose();
            if (texture != null) texture.dispose();
            if (factory != null) factory.dispose();
            if (egl != null) egl.release();
            try { command("send_cancel"); } catch (Exception ignored) { }
        });
        worker.shutdown(); stopForeground(true); super.onDestroy();
    }
}
