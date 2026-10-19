package com.mirrormesh.tv;

import android.app.Activity;
import android.content.ContentValues;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.os.Handler;
import android.os.Looper;
import android.provider.MediaStore;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.TextView;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.util.UUID;
import org.webrtc.SurfaceViewRenderer;

/** Fullscreen native renderer entered only through an approved signaling handoff. */
public final class TvVideoActivity extends Activity implements TvWebRtcReceiver.Listener {
    private TvPendingApprovedOffer pendingOffer;
    private TvWebRtcReceiver receiver;
    private TextView diagnosticsText;
    private Button screenshotButton;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private Runnable screenshotTimeout;
    private volatile boolean ended;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        getWindow().setFlags(
                WindowManager.LayoutParams.FLAG_FULLSCREEN,
                WindowManager.LayoutParams.FLAG_FULLSCREEN);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        setContentView(R.layout.activity_tv_video);

        SurfaceViewRenderer videoSurface = findViewById(R.id.video_surface);
        diagnosticsText = findViewById(R.id.video_diagnostics);
        screenshotButton = findViewById(R.id.video_screenshot_button);
        Button disconnectButton = findViewById(R.id.video_disconnect_button);
        screenshotButton.setOnClickListener(view -> requestScreenshotCapture());
        disconnectButton.setOnClickListener(view -> endSession());
        Button revokeButton = findViewById(R.id.video_revoke_button);
        revokeButton.setOnClickListener(view -> {
            if (ended || pendingOffer == null) return;
            revokeButton.setEnabled(false);
            pendingOffer.signaling.revoke(message -> runOnUiThread(() -> {
                if (isDestroyed()) return;
                if (message.isEmpty()) {
                    pendingOffer.state.revoke();
                    finish();
                }
                else diagnosticsText.setText(message);
            }));
            closeReceiver();
        });

        pendingOffer = TvApprovedSessionLauncher.takePending();
        if (pendingOffer == null
                || !pendingOffer.signaling.isAuthenticatedAndApproved()
                || pendingOffer.state.phase() != TvReceiverState.Phase.APPROVED) {
            if (pendingOffer != null) {
                pendingOffer.signaling.close();
            }
            finish();
            return;
        }

        try {
            receiver = TvWebRtcReceiver.create(this, videoSurface, this, pendingOffer.state);
            receiver.start(pendingOffer.signaling, pendingOffer.offerSdp);
        } catch (RuntimeException error) {
            pendingOffer.state.fail();
            if (receiver != null) {
                receiver.close();
                receiver = null;
            } else {
                pendingOffer.signaling.close();
            }
            diagnosticsText.setText(R.string.video_session_failed);
        }
    }

    @Override
    public void onStatus(String status) {
        runOnUiThread(() -> {
            if (diagnosticsText != null && !ended) {
                diagnosticsText.setText(status);
            }
        });
    }

    @Override
    public void onFrameReceived(int width, int height, long frameCount) {
        if (frameCount % 30 != 0) {
            return;
        }
        runOnUiThread(() -> {
            if (diagnosticsText != null && !ended) {
                diagnosticsText.setText(getString(
                        R.string.video_frame_diagnostics, width, height, frameCount));
            }
        });
    }

    @Override
    public void onFirstFrameRendered() {
        onStatus(getString(R.string.video_receiving));
        runOnUiThread(() -> {
            if (screenshotButton != null && !ended) {
                screenshotButton.setEnabled(true);
            }
        });
    }

    @Override
    public void onScreenshotCaptured(byte[] jpeg, int width, int height) {
        clearScreenshotTimeout();
        if (ended) {
            return;
        }
        try {
            String location = saveScreenshot(jpeg);
            runOnUiThread(() -> {
                if (diagnosticsText != null && screenshotButton != null && !ended) {
                    screenshotButton.setEnabled(true);
                    diagnosticsText.setText(getString(
                            R.string.video_screenshot_saved, width, height, location));
                }
            });
        } catch (IOException | RuntimeException error) {
            onScreenshotFailed();
        }
    }

    @Override
    public void onScreenshotFailed() {
        clearScreenshotTimeout();
        runOnUiThread(() -> {
            if (diagnosticsText != null && screenshotButton != null && !ended) {
                screenshotButton.setEnabled(
                        receiver != null && receiver.diagnostics().receivedFrameCount > 0);
                diagnosticsText.setText(R.string.video_screenshot_failed);
            }
        });
    }

    @Override
    public void onFailure(String code) {
        runOnUiThread(() -> {
            if (diagnosticsText != null && !ended) {
                if (screenshotButton != null) {
                    screenshotButton.setEnabled(false);
                }
                int status = "reconnect_timeout".equals(code)
                        ? R.string.video_reconnect_failed
                        : R.string.video_session_failed;
                diagnosticsText.setText(getString(status));
            }
        });
    }

    @Override
    public void onRevoked() {
        runOnUiThread(() -> {
            if (diagnosticsText != null) {
                diagnosticsText.setText(getString(R.string.video_session_revoked));
            }
            finish();
        });
    }

    @Override
    public void onBackPressed() {
        endSession();
    }

    @Override
    protected void onStop() {
        super.onStop();
        if (!isChangingConfigurations()) {
            closeReceiver();
            finish();
        }
    }

    @Override
    protected void onDestroy() {
        closeReceiver();
        super.onDestroy();
    }

    private void endSession() {
        closeReceiver();
        finish();
    }

    private void closeReceiver() {
        if (ended) {
            return;
        }
        ended = true;
        clearScreenshotTimeout();
        if (screenshotButton != null) {
            screenshotButton.setEnabled(false);
        }
        if (pendingOffer != null) {
            pendingOffer.state.disconnect();
        }
        if (receiver != null) {
            receiver.close();
            receiver = null;
        } else if (pendingOffer != null) {
            pendingOffer.signaling.clearListeners();
            pendingOffer.signaling.close();
        }
    }

    private void requestScreenshotCapture() {
        TvWebRtcReceiver activeReceiver = receiver;
        if (ended || activeReceiver == null || !activeReceiver.requestScreenshotCapture()) {
            return;
        }
        screenshotButton.setEnabled(false);
        diagnosticsText.setText(R.string.video_screenshot_waiting);
        screenshotTimeout = () -> {
            screenshotTimeout = null;
            if (receiver != null) {
                receiver.cancelScreenshotCapture();
            }
            if (!ended && screenshotButton != null && diagnosticsText != null) {
                screenshotButton.setEnabled(
                        receiver != null && receiver.diagnostics().receivedFrameCount > 0);
                diagnosticsText.setText(R.string.video_screenshot_timeout);
            }
        };
        mainHandler.postDelayed(screenshotTimeout, 5000);
    }

    private void clearScreenshotTimeout() {
        Runnable timeout = screenshotTimeout;
        screenshotTimeout = null;
        if (timeout != null) {
            mainHandler.removeCallbacks(timeout);
        }
    }

    private String saveScreenshot(byte[] jpeg) throws IOException {
        if (jpeg == null || jpeg.length == 0) {
            throw new IOException("The screenshot is empty.");
        }
        String filename = "MirrorMesh-" + UUID.randomUUID() + ".jpg";
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ContentValues values = new ContentValues();
            values.put(MediaStore.Images.Media.DISPLAY_NAME, filename);
            values.put(MediaStore.Images.Media.MIME_TYPE, "image/jpeg");
            values.put(
                    MediaStore.Images.Media.RELATIVE_PATH,
                    Environment.DIRECTORY_PICTURES + "/MirrorMesh");
            values.put(MediaStore.Images.Media.IS_PENDING, 1);
            Uri imageUri = getContentResolver().insert(
                    MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values);
            if (imageUri == null) {
                throw new IOException("The screenshot destination is unavailable.");
            }
            try {
                try (OutputStream output = getContentResolver().openOutputStream(imageUri, "w")) {
                    if (output == null) {
                        throw new IOException("The screenshot could not be opened for writing.");
                    }
                    output.write(jpeg);
                }
                ContentValues published = new ContentValues();
                published.put(MediaStore.Images.Media.IS_PENDING, 0);
                if (getContentResolver().update(imageUri, published, null, null) <= 0) {
                    throw new IOException("The screenshot could not be published.");
                }
            } catch (IOException | RuntimeException error) {
                getContentResolver().delete(imageUri, null, null);
                throw error;
            }
            return "Pictures/MirrorMesh";
        }

        File pictures = getExternalFilesDir(Environment.DIRECTORY_PICTURES);
        if (pictures == null) {
            throw new IOException("The app screenshot directory is unavailable.");
        }
        File directory = new File(pictures, "MirrorMesh");
        if (!directory.isDirectory() && !directory.mkdirs()) {
            throw new IOException("The app screenshot directory could not be created.");
        }
        File target = new File(directory, filename);
        try {
            try (OutputStream output = new FileOutputStream(target)) {
                output.write(jpeg);
            }
        } catch (IOException | RuntimeException error) {
            target.delete();
            throw error;
        }
        return target.getAbsolutePath();
    }
}
