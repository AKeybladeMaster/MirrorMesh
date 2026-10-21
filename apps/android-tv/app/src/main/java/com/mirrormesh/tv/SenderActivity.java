package com.mirrormesh.tv;

import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.media.projection.MediaProjectionManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.widget.*;
import org.json.JSONObject;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Pinned pairing and receiver approval precede the Android capture dialog. */
public final class SenderActivity extends Activity {
    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private final Handler handler = new Handler();
    private EditText invitation;
    private TextView status;
    private Button pair, share;
    private long requestId;
    private volatile boolean destroyed;
    private boolean transferred;

    static JSONObject command(JSONObject input) throws Exception {
        JSONObject result = new JSONObject(NativePairingBridge.connectionCommand(input.toString()));
        if (!result.optBoolean("ok")) throw new IllegalStateException(result.optString("error", "Connection failed"));
        return result;
    }

    @Override protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        NativePairingBridge.initializeRustAndroidContext(getApplicationContext());
        LinearLayout body = new LinearLayout(this);
        body.setOrientation(LinearLayout.VERTICAL);
        int padding = (int) (24 * getResources().getDisplayMetrics().density);
        body.setPadding(padding, padding, padding, padding);
        TextView title = new TextView(this); title.setText("Share this screen"); title.setTextSize(28); body.addView(title);
        status = new TextView(this); status.setText("Scan the receiver’s MirrorMesh QR with your camera, or paste its invitation. Pairing never starts capture."); status.setTextSize(18); body.addView(status);
        invitation = new EditText(this); invitation.setHint("Receiver invitation"); invitation.setSingleLine(false); invitation.setMaxLines(4); body.addView(invitation);
        if (getIntent().getData() != null) invitation.setText(getIntent().getDataString());
        pair = new Button(this); pair.setText("Pair with receiver"); body.addView(pair);
        share = new Button(this); share.setText("Choose screen to share"); share.setEnabled(false); body.addView(share);
        Button stop = new Button(this); stop.setText("Stop sharing / disconnect"); body.addView(stop);
        ScrollView scroll = new ScrollView(this); scroll.addView(body); setContentView(scroll);
        pair.setOnClickListener(view -> beginPairing());
        share.setOnClickListener(view -> {
            MediaProjectionManager manager = (MediaProjectionManager) getSystemService(MEDIA_PROJECTION_SERVICE);
            startActivityForResult(manager.createScreenCaptureIntent(), 88);
        });
        stop.setOnClickListener(view -> { stopService(new Intent(this, ScreenSenderService.class)); transferred = false; finish(); });
    }

    private void beginPairing() {
        if (Build.VERSION.SDK_INT >= 37 && checkSelfPermission("android.permission.ACCESS_LOCAL_NETWORK") != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[]{"android.permission.ACCESS_LOCAL_NETWORK"}, 37); return;
        }
        String text = invitation.getText().toString().trim();
        if (text.isEmpty() || text.length() > 1024) { status.setText("Paste a complete MirrorMesh receiver invitation."); return; }
        pair.setEnabled(false); status.setText("Verifying receiver identity…");
        worker.execute(() -> {
            try {
                requestId = command(new JSONObject().put("op", "send_pair").put("invitation", text)).getLong("id");
                if (destroyed) { command(new JSONObject().put("op", "send_cancel").put("id", requestId)); return; }
                pollPairing();
            } catch (Exception failure) { showFailure(failure); }
        });
    }

    private void pollPairing() {
        if (destroyed) return;
        try {
            JSONObject result = command(new JSONObject().put("op", "send_poll").put("id", requestId));
            int state = result.getInt("state");
            runOnUiThread(() -> {
                if (destroyed) return;
                if (state == 3) { share.setEnabled(true); status.setText("Receiver approved. Choose a screen using Android’s capture dialog. Everything visible in that screen may be shared."); }
                else if (state >= 4) { pair.setEnabled(true); status.setText("Pairing ended (MM-PAIR-" + result.optInt("error") + "). Refresh the receiver invitation before retrying."); }
                else status.setText("Waiting for explicit approval on the receiver…");
            });
            if (state < 3) handler.postDelayed(() -> { if (!destroyed) worker.execute(this::pollPairing); }, 300);
        } catch (Exception failure) { showFailure(failure); }
    }

    private void showFailure(Exception failure) {
        runOnUiThread(() -> { if (!destroyed) { pair.setEnabled(true); status.setText(failure.getMessage()); } });
    }

    @Override public void onRequestPermissionsResult(int code, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(code, permissions, results);
        if (code == 37 && results.length > 0 && results[0] == PackageManager.PERMISSION_GRANTED) beginPairing();
    }

    @Override protected void onActivityResult(int code, int result, Intent data) {
        super.onActivityResult(code, result, data);
        if (code != 88 || result != RESULT_OK || data == null) return;
        Intent service = new Intent(this, ScreenSenderService.class).putExtra("projection", data).putExtra("request", requestId);
        if (Build.VERSION.SDK_INT >= 26) startForegroundService(service); else startService(service);
        transferred = true; share.setEnabled(false); status.setText("Screen sharing started. Use Stop sharing or the Android notification to end it.");
    }

    @Override protected void onDestroy() {
        destroyed = true; handler.removeCallbacksAndMessages(null);
        if (!transferred && requestId > 0) worker.execute(() -> {
            try { command(new JSONObject().put("op", "send_cancel").put("id", requestId)); } catch (Exception ignored) { }
        });
        worker.shutdown(); super.onDestroy();
    }
}
