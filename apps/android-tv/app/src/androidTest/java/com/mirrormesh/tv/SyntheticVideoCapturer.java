package com.mirrormesh.tv;

import android.content.Context;
import java.nio.ByteBuffer;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import org.webrtc.JavaI420Buffer;
import org.webrtc.SurfaceTextureHelper;
import org.webrtc.VideoCapturer;
import org.webrtc.VideoFrame;

/** Test-only moving color bars delivered through libwebrtc's native video source. */
final class SyntheticVideoCapturer implements VideoCapturer {
    private volatile CapturerObserver observer;
    private volatile int width = 320;
    private volatile int height = 180;
    private volatile int framesPerSecond = 15;
    private volatile long frameIndex;
    private ScheduledExecutorService executor;

    @Override
    public void initialize(
            SurfaceTextureHelper surfaceTextureHelper,
            Context applicationContext,
            CapturerObserver capturerObserver) {
        if (capturerObserver == null || observer != null) {
            throw new IllegalStateException("Synthetic capturer initialization is invalid.");
        }
        observer = capturerObserver;
    }

    @Override
    public synchronized void startCapture(int requestedWidth, int requestedHeight, int fps) {
        if (observer == null || executor != null) {
            throw new IllegalStateException("Synthetic capturer is not ready to start.");
        }
        changeCaptureFormat(requestedWidth, requestedHeight, fps);
        observer.onCapturerStarted(true);
        executor = Executors.newSingleThreadScheduledExecutor(task -> {
            Thread thread = new Thread(task, "mirrormesh-test-video-source");
            thread.setDaemon(true);
            return thread;
        });
        long periodNanos = TimeUnit.SECONDS.toNanos(1) / framesPerSecond;
        executor.scheduleAtFixedRate(this::captureFrame, 0, periodNanos, TimeUnit.NANOSECONDS);
    }

    private void captureFrame() {
        CapturerObserver currentObserver = observer;
        if (currentObserver == null) {
            return;
        }
        int frameWidth = width;
        int frameHeight = height;
        long frameNumber = frameIndex++;
        JavaI420Buffer buffer = JavaI420Buffer.allocate(frameWidth, frameHeight);
        fillColorBars(buffer, frameWidth, frameHeight, frameNumber);
        VideoFrame frame = new VideoFrame(buffer, 0, System.nanoTime());
        try {
            currentObserver.onFrameCaptured(frame);
        } finally {
            frame.release();
        }
    }

    private static void fillColorBars(
            JavaI420Buffer buffer, int width, int height, long frameIndex) {
        ByteBuffer y = buffer.getDataY();
        ByteBuffer u = buffer.getDataU();
        ByteBuffer v = buffer.getDataV();
        int[] luma = {235, 210, 170, 145, 106, 81, 41, 16};
        int barWidth = Math.max(1, width / luma.length);
        int shift = (int) (frameIndex % width);
        for (int row = 0; row < height; row++) {
            for (int column = 0; column < width; column++) {
                int bar = ((column + shift) / barWidth) % luma.length;
                y.put(row * buffer.getStrideY() + column, (byte) luma[bar]);
            }
        }
        for (int row = 0; row < height / 2; row++) {
            for (int column = 0; column < width / 2; column++) {
                int bar = (((column * 2) + shift) / barWidth) % luma.length;
                int offsetU = row * buffer.getStrideU() + column;
                int offsetV = row * buffer.getStrideV() + column;
                u.put(offsetU, (byte) (bar % 2 == 0 ? 90 : 166));
                v.put(offsetV, (byte) (bar % 2 == 0 ? 240 : 16));
            }
        }
    }

    @Override
    public synchronized void stopCapture() throws InterruptedException {
        ScheduledExecutorService running = executor;
        executor = null;
        if (running != null) {
            running.shutdownNow();
            if (!running.awaitTermination(5, TimeUnit.SECONDS)) {
                throw new IllegalStateException("Synthetic capture did not stop.");
            }
            CapturerObserver currentObserver = observer;
            if (currentObserver != null) {
                currentObserver.onCapturerStopped();
            }
        }
    }

    @Override
    public void changeCaptureFormat(int requestedWidth, int requestedHeight, int fps) {
        if (requestedWidth < 16 || requestedHeight < 16
                || (requestedWidth & 1) != 0 || (requestedHeight & 1) != 0
                || fps < 1 || fps > 60) {
            throw new IllegalArgumentException("Synthetic capture format is unsupported.");
        }
        width = requestedWidth;
        height = requestedHeight;
        framesPerSecond = fps;
    }

    @Override
    public synchronized void dispose() {
        try {
            stopCapture();
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        }
        observer = null;
    }

    @Override
    public boolean isScreencast() {
        return false;
    }
}
