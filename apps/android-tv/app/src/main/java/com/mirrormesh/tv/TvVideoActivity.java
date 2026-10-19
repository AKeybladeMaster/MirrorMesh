package com.mirrormesh.tv;

import android.app.Activity;
import android.os.Bundle;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.TextView;
import org.webrtc.SurfaceViewRenderer;

/** Fullscreen native renderer entered only through an approved signaling handoff. */
public final class TvVideoActivity extends Activity implements TvWebRtcReceiver.Listener {
    private TvPendingApprovedOffer pendingOffer;
    private TvWebRtcReceiver receiver;
    private TextView diagnosticsText;
    private boolean ended;

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
        Button disconnectButton = findViewById(R.id.video_disconnect_button);
        disconnectButton.setOnClickListener(view -> endSession());

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
    }

    @Override
    public void onFailure(String code) {
        runOnUiThread(() -> {
            if (diagnosticsText != null && !ended) {
                diagnosticsText.setText(getString(R.string.video_session_failed));
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
}
