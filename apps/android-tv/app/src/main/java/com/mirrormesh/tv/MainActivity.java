package com.mirrormesh.tv;

import android.app.Activity;
import android.app.AlertDialog;
import android.widget.ScrollView;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Intent;
import android.content.ActivityNotFoundException;
import android.content.pm.PackageManager;
import android.content.res.Configuration;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.os.Bundle;
import android.os.Build;
import android.view.View;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

import com.google.zxing.BarcodeFormat;
import com.google.zxing.EncodeHintType;
import com.google.zxing.WriterException;
import com.google.zxing.common.BitMatrix;
import com.google.zxing.qrcode.QRCodeWriter;

import org.json.JSONException;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Native TV pairing UI with an approved signaling handoff to the fullscreen receiver. */
public final class MainActivity extends Activity {
    private static final int PAIRING_PORT = 8765;
    private static final int LAN_PERMISSION_REQUEST = 37;
    private static final String LAN_PERMISSION = "android.permission.ACCESS_LOCAL_NETWORK";

    private final TvPairingState pairingState = new TvPairingState();
    private final ExecutorService controlWorker = Executors.newSingleThreadExecutor();
    private final ExecutorService acceptWorker = Executors.newSingleThreadExecutor();
    private final List<LocalEndpoint> localEndpoints = new ArrayList<>();

    private Button startButton;
    private Button stopButton;
    private Button waitButton;
    private Button approveButton;
    private Button rejectButton;
    private Button closeChannelButton;
    private Button refreshQrButton;
    private TextView sessionState;
    private TextView serviceState;
    private TextView requestDetails;
    private TextView helpText;
    private ImageView invitationQr;
    private TextView invitationText;
    private CheckBox grantView;
    private CheckBox grantSend;
    private CheckBox grantAnnotate;
    private RustApprovedSignalingChannel activeSignaling;
    private TvReceiverState tvReceiverState;

    private boolean nativeReady;
    private boolean decisionInProgress;
    private boolean destroyed;
    private int serviceGeneration;
    private int waitGeneration;
    private ReceiverAdvertisement advertisement;
    private ConnectionDialogs dialogs;
    private AlertDialog approvalDialog;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        bindViews();
        dialogs = new ConnectionDialogs(this);
        advertisement = new ReceiverAdvertisement(this);
        adaptLayout();
        View root = findViewById(android.R.id.content);
        root.setOnApplyWindowInsetsListener((view, insets) -> {
            if (Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets safe = insets.getInsets(
                        android.view.WindowInsets.Type.systemBars() | android.view.WindowInsets.Type.displayCutout());
                view.setPadding(safe.left, safe.top, safe.right, safe.bottom);
                return insets;
            }
            view.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(),
                    insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
            return insets;
        });
        configureEndpoints();
        initializeNativeContext();

        startButton.setOnClickListener(view -> startReceiver());
        findViewById(R.id.open_sender_button).setOnClickListener(view -> {
            stopReceiver();
            startActivity(new Intent(this, SenderActivity.class));
        });
        stopButton.setOnClickListener(view -> stopReceiver());
        waitButton.setOnClickListener(view -> waitForSender());
        approveButton.setOnClickListener(view -> approveRequest());
        rejectButton.setOnClickListener(view -> rejectRequest());
        closeChannelButton.setOnClickListener(view -> closeChannel());
        refreshQrButton.setOnClickListener(view -> refreshInvitation());
        findViewById(R.id.copy_invitation_button).setOnClickListener(view -> shareInvitation(false));
        findViewById(R.id.share_invitation_button).setOnClickListener(view -> shareInvitation(true));
        grantView.setOnCheckedChangeListener((button, checked) -> renderState());
        grantSend.setOnCheckedChangeListener((button, checked) -> renderState());
        grantAnnotate.setOnCheckedChangeListener((button, checked) -> renderState());
        renderState();
        startButton.requestFocus();
    }

    private void bindViews() {
        startButton = findViewById(R.id.start_receiver_button);
        stopButton = findViewById(R.id.stop_receiver_button);
        waitButton = findViewById(R.id.wait_button);
        approveButton = findViewById(R.id.approve_button);
        rejectButton = findViewById(R.id.reject_button);
        closeChannelButton = findViewById(R.id.close_channel_button);
        refreshQrButton = findViewById(R.id.refresh_qr_button);
        sessionState = findViewById(R.id.session_state);
        serviceState = findViewById(R.id.service_state);
        requestDetails = findViewById(R.id.request_details);
        helpText = findViewById(R.id.help_text);
        invitationQr = findViewById(R.id.invitation_qr);
        invitationText = findViewById(R.id.invitation_text);
        grantView = findViewById(R.id.grant_view);
        grantSend = findViewById(R.id.grant_send);
        grantAnnotate = findViewById(R.id.grant_annotate);
    }

    private void adaptLayout() {
        Configuration configuration = getResources().getConfiguration();
        boolean compact = configuration.screenWidthDp / configuration.fontScale < 700;
        LinearLayout columns = findViewById(R.id.pairing_columns);
        columns.setOrientation(compact ? LinearLayout.VERTICAL : LinearLayout.HORIZONTAL);
        View codePanel = findViewById(R.id.pairing_code_panel);
        LinearLayout.LayoutParams codeParams = (LinearLayout.LayoutParams) codePanel.getLayoutParams();
        codeParams.width = compact ? LinearLayout.LayoutParams.MATCH_PARENT
                : (int) (320 * getResources().getDisplayMetrics().density);
        codeParams.setMarginEnd(compact ? 0 : (int) (28 * getResources().getDisplayMetrics().density));
        codePanel.setLayoutParams(codeParams);
        View controls = findViewById(R.id.pairing_controls);
        LinearLayout.LayoutParams controlsParams = (LinearLayout.LayoutParams) controls.getLayoutParams();
        controlsParams.width = compact ? LinearLayout.LayoutParams.MATCH_PARENT : 0;
        controlsParams.weight = compact ? 0 : 1;
        controlsParams.topMargin = compact ? (int) (24 * getResources().getDisplayMetrics().density) : 0;
        controls.setLayoutParams(controlsParams);
    }

    @Override
    public void onConfigurationChanged(Configuration configuration) {
        super.onConfigurationChanged(configuration);
        adaptLayout();
    }

    private void shareInvitation(boolean share) {
        String invitation = invitationText.getText().toString();
        if (invitation.isEmpty() || invitationQr.getVisibility() != View.VISIBLE) return;
        if (share) {
            Intent intent = new Intent(Intent.ACTION_SEND).setType("text/plain")
                    .putExtra(Intent.EXTRA_TEXT, invitation);
            try { startActivity(Intent.createChooser(intent, "Share receiver invitation")); }
            catch (ActivityNotFoundException error) { shareInvitation(false); }
        } else {
            ClipboardManager clipboard = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
            if (clipboard != null) clipboard.setPrimaryClip(ClipData.newPlainText("MirrorMesh invitation", invitation));
            dialogs.notice("Invitation copied", "Paste it into MirrorMesh on the sending device before it expires.");
        }
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(requestCode, permissions, results);
        if (requestCode != LAN_PERMISSION_REQUEST) return;
        if (results.length > 0 && results[0] == PackageManager.PERMISSION_GRANTED) {
            configureEndpoints();
            startReceiver();
        } else {
            serviceState.setText("Local network access is required to receive a PC screen. Allow it in Android Settings, then retry.");
        }
    }

    private void initializeNativeContext() {
        try {
            nativeReady = NativePairingBridge.initializeRustAndroidContext(getApplicationContext());
        } catch (LinkageError | RuntimeException error) {
            nativeReady = false;
        }
        if (!nativeReady) {
            serviceState.setText("The Rust receiver could not initialize. Rebuild the Android TV package with its native library.");
        }
    }

    private void configureEndpoints() {
        localEndpoints.clear();
        try {
            String address = DeviceContext.lanAddress(this);
            if (address != null) localEndpoints.add(new LocalEndpoint("Active network", address));
        } catch (SecurityException error) {
            serviceState.setText("Allow local network access, then try again.");
        }
        if (localEndpoints.isEmpty()) {
            serviceState.setText(R.string.pairing_no_address);
        }
    }

    private void startReceiver() {
        if (!nativeReady || !pairingState.canStart()) {
            return;
        }
        if (Build.VERSION.SDK_INT >= 37 && checkSelfPermission(LAN_PERMISSION) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] { LAN_PERMISSION }, LAN_PERMISSION_REQUEST);
            return;
        }
        configureEndpoints();
        LocalEndpoint endpoint = localEndpoints.isEmpty() ? null : localEndpoints.get(0);
        if (endpoint == null) {
            dialogs.notice("Network unavailable", "Connect to Wi-Fi or Ethernet, then start receiving again.");
            return;
        }
        startButton.setEnabled(false);
        dialogs.progress("Starting receiver", "Preparing a private connection on your active network.");
        int startGeneration = serviceGeneration;
        controlWorker.execute(() -> {
            try {
                DeviceContext.configure(getApplicationContext());
                JSONObject response = requireSuccess(NativePairingBridge.startPairing(
                        endpoint.address + ":" + PAIRING_PORT));
                String invitation = response.getString("invitation");
                Bitmap qrBitmap = createQrBitmap(invitation, 640);
                runOnUiThread(() -> {
                    if (destroyed || startGeneration != serviceGeneration) {
                        return;
                    }
                    invitationQr.setImageBitmap(qrBitmap);
                    invitationQr.setVisibility(View.VISIBLE);
                    invitationQr.setContentDescription(getString(R.string.pairing_accessibility_qr));
                    invitationText.setText(invitation);
                    invitationText.setVisibility(View.GONE);
                    dialogs.endProgress();
                    pairingState.receiverStarted();
                    advertisement.start(response.optString("device_id"),
                            "MirrorMesh " + (getPackageName().endsWith(".tv") ? "TV" : "Android")
                                    + " " + android.os.Build.MODEL.replace('.', '-'), response.optString("fingerprint"));
                    serviceState.setText(R.string.pairing_ready);
                    requestDetails.setText("Waiting for an explicitly selected sender.");
                    renderState();
                    waitForSender();
                });
            } catch (Exception | LinkageError error) {
                try {
                    NativePairingBridge.stopPairing();
                } catch (LinkageError ignored) {
                    // Start cleanup is best effort if the native library is unavailable.
                }
                runOnUiThread(() -> {
                    if (!destroyed && startGeneration == serviceGeneration) {
                        dialogs.notice("Receiver start failed", "Check the active Wi-Fi or Ethernet connection, then retry.");
                        renderState();
                    }
                });
            }
        });
    }

    private void waitForSender() {
        if (!pairingState.beginWait()) {
            return;
        }
        requestDetails.setText("Waiting for a device to request a connection.");
        serviceState.setText("Pairing is active. No screen video is connected.");
        renderState();
        int generation = ++waitGeneration;
        acceptWorker.execute(() -> {
            try {
                JSONObject request = requireSuccess(NativePairingBridge.waitForPairingRequest());
                runOnUiThread(() -> {
                    if (!destroyed && generation == waitGeneration) showPendingRequest(request);
                });
            } catch (Exception | LinkageError error) {
                runOnUiThread(() -> {
                    if (destroyed || generation != waitGeneration
                            || pairingState.phase() != TvPairingState.Phase.WAITING_FOR_REQUEST) {
                        return;
                    }
                    String message = error.getMessage() == null ? "Pairing wait failed" : error.getMessage();
                    if (message.contains("stopped")) {
                        pairingState.receiverStopped();
                        clearInvitation();
                        serviceState.setText(R.string.pairing_stopped);
                    } else {
                        pairingState.waitFailed();
                        dialogs.notice("Connection request ended", "Refresh the invitation to accept a new request.");
                    }
                    renderState();
                });
            }
        });
    }

    private void showPendingRequest(JSONObject request) {
        if (destroyed || !pairingState.requestReceived(request.optLong("request_id"))) {
            return;
        }
        String name = request.optString("display_name", "Unknown sender");
        String sourceIp = request.optString("source_ip", "Unknown address");
        String fingerprint = request.optString("fingerprint", "Unavailable");
        String verification = request.optString("verification_code", "");
        boolean directRequest = !verification.isEmpty() && !"null".equals(verification);
        JSONObject grants = request.optJSONObject("requested_grants");
        boolean requestedView = grants != null && grants.optBoolean("view");
        boolean requestedSend = grants != null && grants.optBoolean("send");
        boolean requestedAnnotate = grants != null && grants.optBoolean("annotate");

        grantView.setChecked(false);
        grantSend.setChecked(false);
        grantAnnotate.setChecked(false);
        grantView.setText(directRequest ? "The code matches the PC. Allow it to send its screen." : getString(R.string.grant_view_label));
        grantView.setVisibility(requestedView ? View.VISIBLE : View.GONE);
        grantSend.setVisibility(requestedSend ? View.VISIBLE : View.GONE);
        grantAnnotate.setVisibility(requestedAnnotate ? View.VISIBLE : View.GONE);
        invitationQr.setVisibility(View.GONE);
        invitationText.setVisibility(View.GONE);
        grantView.setEnabled(requestedView);
        grantSend.setEnabled(requestedSend);
        grantAnnotate.setEnabled(requestedAnnotate);
        requestDetails.setText("A device is requesting a connection.");
        serviceState.setText(R.string.pairing_request_pending);
        renderState();
        JSONObject details = request.optJSONObject("device_details");
        String facts = "Device: " + DeviceContext.clean(name)
                + "\nModel: " + fact(details, "model")
                + "\nManufacturer: " + fact(details, "manufacturer")
                + "\nSystem: " + fact(details, "os_version")
                + "\nAddress: " + DeviceContext.clean(sourceIp)
                + "\nIdentity fingerprint: " + DeviceContext.clean(fingerprint)
                + "\n\nDevice details are reported by the sender. Approve only the device you selected."
                + (directRequest ? "\n\nConnection code: " + DeviceContext.clean(verification)
                    + "\nCompare every group with the sending device." : "");
        LinearLayout body = new LinearLayout(this);
        body.setOrientation(LinearLayout.VERTICAL);
        int padding = (int) (24 * getResources().getDisplayMetrics().density);
        body.setPadding(padding, 0, padding, 0);
        TextView factsView = new TextView(this);
        factsView.setText(facts); factsView.setTextSize(18); factsView.setTextIsSelectable(true);
        body.addView(factsView);
        CheckBox[] grantsUi = new CheckBox[3];
        CheckBox[] grantsState = {grantView, grantSend, grantAnnotate};
        boolean[] requested = {requestedView, requestedSend, requestedAnnotate};
        String[] labels = {directRequest ? "The codes match. Allow screen viewing" : "Allow screen viewing",
                "Allow sending", "Allow annotation"};
        for (int i = 0; i < grantsUi.length; i++) {
            final int index = i;
            CheckBox box = new CheckBox(this); grantsUi[i] = box;
            box.setText(labels[i]); box.setTextSize(18); box.setMinHeight((int) (56 * getResources().getDisplayMetrics().density));
            box.setVisibility(requested[i] ? View.VISIBLE : View.GONE);
            box.setOnCheckedChangeListener((button, checked) -> {
                grantsState[index].setChecked(checked);
                if (approvalDialog != null) approvalDialog.getButton(AlertDialog.BUTTON_POSITIVE)
                        .setEnabled(grantView.isChecked() || grantSend.isChecked() || grantAnnotate.isChecked());
            });
            body.addView(box);
        }
        ScrollView scroll = new ScrollView(this); scroll.addView(body);
        approvalDialog = new AlertDialog.Builder(this).setTitle("Receive this device's screen?")
                .setView(scroll).setCancelable(false)
                .setNegativeButton("Reject", (dialog, which) -> { approvalDialog = null; rejectRequest(); })
                .setPositiveButton("Approve", (dialog, which) -> { approvalDialog = null; approveRequest(); }).create();
        dialogs.show(approvalDialog);
        approvalDialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(false);
    }

    private static String fact(JSONObject details, String key) {
        String value = details == null ? "" : DeviceContext.clean(details.optString(key, ""));
        return value.isEmpty() ? "Not supplied" : value;
    }

    private void approveRequest() {
        if (!pairingState.canApproveOrReject() || decisionInProgress) {
            return;
        }
        boolean view = grantView.isChecked();
        boolean send = grantSend.isChecked();
        boolean annotate = grantAnnotate.isChecked();
        if (!view && !send && !annotate) {
            serviceState.setText("Select at least one requested permission, or reject this request.");
            return;
        }
        decisionInProgress = true;
        dialogs.progress("Approving connection", "Applying the permissions you selected.");
        renderState();
        long requestId = pairingState.requestId();
        controlWorker.execute(() -> {
            try {
                JSONObject result = requireSuccess(NativePairingBridge.approvePairing(
                        requestId, view, send, annotate));
                long connectionId = result.getLong("connection_id");
                String peerId = result.optString("peer_id", "paired peer");
                RustApprovedSignalingChannel signaling = view
                        ? new RustApprovedSignalingChannel(
                                connectionId, result.getString("peer_fingerprint"))
                        : null;
                TvReceiverState sessionState = null;
                if (view) {
                    sessionState = new TvReceiverState();
                    if (!sessionState.showAuthenticatedRequest() || !sessionState.approve()) {
                        throw new IllegalStateException("Approved receiver state could not start.");
                    }
                }
                TvReceiverState approvedSessionState = sessionState;
                runOnUiThread(() -> {
                    if (destroyed) {
                        if (signaling != null) {
                            signaling.close();
                        } else {
                            NativePairingBridge.closeControlChannel(connectionId);
                        }
                        return;
                    }
                    decisionInProgress = false;
                    if (!pairingState.requestApproved(connectionId)) {
                        if (signaling != null) {
                            signaling.close();
                        }
                        return;
                    }
                    dialogs.notice("Connection approved", "Waiting for the sender to choose a screen.");
                    activeSignaling = signaling;
                    tvReceiverState = approvedSessionState;
                    requestDetails.setText("Paired sender: " + peerId
                            + (view
                                    ? "\nWaiting for the sender's H.264 screen offer."
                                    : "\nScreen viewing was not approved."));
                    invitationQr.setVisibility(View.GONE);
                    invitationText.setVisibility(View.GONE);
                    serviceState.setText(R.string.pairing_paired);
                    helpText.setText(view
                            ? "The approved sender may begin a screen session. Press Back during video to disconnect."
                            : "Close channel ends this connection. The approved peer remains trusted.");
                    clearGrantSelection();
                    renderState();
                    closeChannelButton.requestFocus();
                    if (signaling != null) {
                        signaling.setRevokedListener(() -> runOnUiThread(() -> {
                            if (tvReceiverState != null) {
                                tvReceiverState.revoke();
                            }
                            finishReturnedVideoSession();
                        }));
                        signaling.setRemoteStopListener(() -> runOnUiThread(
                                this::finishReturnedVideoSession));
                        signaling.setFailureListener(code -> runOnUiThread(
                                this::finishReturnedVideoSession));
                        signaling.start(sdp -> runOnUiThread(() ->
                                launchApprovedOffer(signaling, approvedSessionState, sdp)));
                    }
                });
            } catch (Exception | LinkageError error) {
                runOnUiThread(() -> {
                    decisionInProgress = false;
                    if (pairingState.decisionFailed()) {
                        dialogs.notice("Approval failed", "The request expired or was interrupted. Stop and restart the receiver.");
                        renderState();
                    }
                });
            }
        });
    }

    private void rejectRequest() {
        if (!pairingState.canApproveOrReject() || decisionInProgress) {
            return;
        }
        decisionInProgress = true;
        dialogs.progress("Rejecting connection", "This device will not be allowed to share.");
        renderState();
        long requestId = pairingState.requestId();
        controlWorker.execute(() -> {
            try {
                requireSuccess(NativePairingBridge.rejectPairing(requestId));
                FreshInvitation freshInvitation = null;
                String refreshFailure = null;
                try {
                    freshInvitation = fetchFreshInvitation();
                } catch (Exception | LinkageError error) {
                    refreshFailure = error.getMessage() == null
                            ? "Could not create a new pairing code." : error.getMessage();
                }
                FreshInvitation refreshed = freshInvitation;
                String invitationError = refreshFailure;
                runOnUiThread(() -> {
                    if (destroyed) {
                        return;
                    }
                    decisionInProgress = false;
                    if (!pairingState.requestRejected()) {
                        return;
                    }
                    dialogs.notice("Connection rejected", "No screen access was granted.");
                    clearGrantSelection();
                    requestDetails.setText("Sender rejected. No trusted-peer record was added.");
                    if (refreshed != null) {
                        displayInvitation(refreshed);
                        serviceState.setText(R.string.pairing_ready);
                    } else {
                        serviceState.setText("Sender rejected. Pairing code refresh failed: " + invitationError);
                    }
                    renderState();
                    waitButton.requestFocus();
                });
            } catch (Exception | LinkageError error) {
                runOnUiThread(() -> {
                    decisionInProgress = false;
                    if (pairingState.decisionFailed()) {
                        dialogs.notice("Rejection failed", "Stop and restart the receiver before accepting another device.");
                        renderState();
                    }
                });
            }
        });
    }

    private void closeChannel() {
        if (!pairingState.canCloseChannel() || decisionInProgress) {
            return;
        }
        decisionInProgress = true;
        dialogs.progress("Disconnecting", "Closing the approved connection.");
        renderState();
        long connectionId = pairingState.connectionId();
        controlWorker.execute(() -> {
            try {
                boolean closedChannel = activeSignaling == null
                        ? NativePairingBridge.closeControlChannel(connectionId)
                        : activeSignaling.closeForUser();
                if (!closedChannel) {
                    throw new IllegalStateException("The signaling channel could not be closed cleanly.");
                }
                FreshInvitation freshInvitation = null;
                String refreshFailure = null;
                try {
                    freshInvitation = fetchFreshInvitation();
                } catch (Exception | LinkageError error) {
                    refreshFailure = error.getMessage() == null
                            ? "Could not create a new pairing code." : error.getMessage();
                }
                FreshInvitation refreshed = freshInvitation;
                String invitationError = refreshFailure;
                runOnUiThread(() -> {
                    if (destroyed) {
                        return;
                    }
                    decisionInProgress = false;
                    if (!pairingState.controlChannelClosed()) {
                        return;
                    }
                    dialogs.notice("Disconnected", "Screen access for this session has ended.");
                    activeSignaling = null;
                    tvReceiverState = null;
                    requestDetails.setText(R.string.pairing_trust_remains);
                    if (refreshed != null) {
                        displayInvitation(refreshed);
                        serviceState.setText(R.string.pairing_ready);
                    } else {
                        serviceState.setText("Channel closed. Pairing code refresh failed: " + invitationError);
                    }
                    renderState();
                    waitButton.requestFocus();
                });
            } catch (Exception | LinkageError error) {
                showOperationFailure("Channel close failed", error);
            }
        });
    }

    private void launchApprovedOffer(
            RustApprovedSignalingChannel signaling,
            TvReceiverState state,
            String offerSdp) {
        if (destroyed || signaling != activeSignaling || state == null
                || state.phase() != TvReceiverState.Phase.APPROVED) {
            signaling.close();
            return;
        }
        try {
            dialogs.close();
            TvApprovedSessionLauncher.open(this, signaling, offerSdp, state);
            serviceState.setText("Starting the approved H.264 screen session...");
        } catch (RuntimeException error) {
            state.fail();
            signaling.close();
            serviceState.setText("Screen offer rejected: "
                    + (error.getMessage() == null ? "unsupported offer" : error.getMessage()));
            finishReturnedVideoSession();
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (pairingState.canCloseChannel()
                && activeSignaling != null
                && !activeSignaling.isAuthenticatedAndApproved()) {
            finishReturnedVideoSession();
        }
    }

    private void finishReturnedVideoSession() {
        boolean revoked = tvReceiverState != null
                && tvReceiverState.phase() == TvReceiverState.Phase.REVOKED;
        activeSignaling = null;
        tvReceiverState = null;
        if (!pairingState.controlChannelClosed()) {
            return;
        }
        decisionInProgress = true;
        serviceState.setText(revoked
                ? "Sender revoked this connection. Refreshing the pairing code..."
                : "Screen session ended. Refreshing the pairing code...");
        renderState();
        int refreshGeneration = serviceGeneration;
        controlWorker.execute(() -> {
            FreshInvitation freshInvitation = null;
            String refreshFailure = null;
            try {
                freshInvitation = fetchFreshInvitation();
            } catch (Exception | LinkageError error) {
                refreshFailure = error.getMessage() == null
                        ? "Could not create a new pairing code." : error.getMessage();
            }
            FreshInvitation refreshed = freshInvitation;
            String invitationError = refreshFailure;
            runOnUiThread(() -> {
                if (destroyed || refreshGeneration != serviceGeneration) {
                    return;
                }
                decisionInProgress = false;
                requestDetails.setText(revoked
                        ? "Access revoked. A fresh invitation and explicit approval are needed to pair again."
                        : getString(R.string.pairing_trust_remains));
                if (refreshed != null) {
                    displayInvitation(refreshed);
                    serviceState.setText(R.string.pairing_ready);
                } else {
                    serviceState.setText("Session ended. Pairing code refresh failed: " + invitationError);
                }
                renderState();
                waitButton.requestFocus();
            });
        });
    }

    private void refreshInvitation() {
        TvPairingState.Phase phase = pairingState.phase();
        boolean cancelWait = phase == TvPairingState.Phase.WAITING_FOR_REQUEST;
        if (decisionInProgress || (!cancelWait && phase != TvPairingState.Phase.READY)) {
            return;
        }
        if (cancelWait) {
            ++waitGeneration;
            pairingState.waitFailed();
        }
        decisionInProgress = true;
        dialogs.progress("Refreshing invitation", "Creating a new one-time QR invitation.");
        renderState();
        int refreshGeneration = serviceGeneration;
        controlWorker.execute(() -> {
            try {
                if (cancelWait) {
                    requireSuccess(NativePairingBridge.cancelPairingWait());
                }
                FreshInvitation freshInvitation = fetchFreshInvitation();
                runOnUiThread(() -> {
                    if (destroyed || refreshGeneration != serviceGeneration) {
                        return;
                    }
                    decisionInProgress = false;
                    if (pairingState.phase() != TvPairingState.Phase.READY) {
                        renderState();
                        return;
                    }
                    dialogs.endProgress();
                    displayInvitation(freshInvitation);
                    requestDetails.setText("Fresh invitation created. Ready for the PC to pair.");
                    serviceState.setText(R.string.pairing_ready);
                    renderState();
                    refreshQrButton.requestFocus();
                    waitForSender();
                });
            } catch (Exception | LinkageError error) {
                runOnUiThread(() -> {
                    if (destroyed || refreshGeneration != serviceGeneration) {
                        return;
                    }
                    decisionInProgress = false;
                    dialogs.notice("Invitation refresh failed", "Check the network connection, then stop and restart receiving.");
                    renderState();
                });
            }
        });
    }

    private FreshInvitation fetchFreshInvitation() throws Exception {
        JSONObject response = requireSuccess(NativePairingBridge.refreshPairingInvitation());
        String invitation = response.getString("invitation");
        return new FreshInvitation(invitation, createQrBitmap(invitation, 640));
    }

    private void displayInvitation(FreshInvitation invitation) {
        invitationQr.setImageBitmap(invitation.bitmap);
        invitationQr.setContentDescription(getString(R.string.pairing_accessibility_qr));
        invitationQr.setVisibility(View.VISIBLE);
        invitationText.setText(invitation.uri);
        invitationText.setVisibility(View.GONE);
    }

    private void stopReceiver() {
        if (dialogs != null) dialogs.close();
        approvalDialog = null;
        if (advertisement != null) advertisement.stop();
        boolean wasActive = pairingState.phase() != TvPairingState.Phase.STOPPED;
        RustApprovedSignalingChannel signaling = activeSignaling;
        activeSignaling = null;
        tvReceiverState = null;
        serviceGeneration++;
        waitGeneration++;
        pairingState.receiverStopped();
        decisionInProgress = false;
        clearGrantSelection();
        clearInvitation();
        serviceState.setText(R.string.pairing_stopped);
        requestDetails.setText("Pairing stopped. Previously approved peer trust remains stored.");
        renderState();
        if (wasActive || nativeReady) {
            controlWorker.execute(() -> {
                try {
                    if (signaling != null) {
                        signaling.close();
                    }
                    NativePairingBridge.stopPairing();
                } catch (LinkageError ignored) {
                    // The activity remains usable if the packaged Rust library is unavailable.
                }
            });
        }
    }

    private void clearInvitation() {
        invitationQr.setImageBitmap(null);
        invitationQr.setVisibility(View.GONE);
        invitationQr.setContentDescription(getString(R.string.pairing_accessibility_qr));
        invitationText.setText("");
        invitationText.setVisibility(View.GONE);
    }

    private void clearGrantSelection() {
        grantView.setChecked(false);
        grantSend.setChecked(false);
        grantAnnotate.setChecked(false);
        grantView.setVisibility(View.GONE);
        grantSend.setVisibility(View.GONE);
        grantAnnotate.setVisibility(View.GONE);
    }

    private void renderState() {
        TvPairingState.Phase phase = pairingState.phase();
        boolean stopped = phase == TvPairingState.Phase.STOPPED;
        boolean pending = phase == TvPairingState.Phase.REQUEST_PENDING;
        boolean waiting = phase == TvPairingState.Phase.WAITING_FOR_REQUEST;
        boolean paired = phase == TvPairingState.Phase.PAIRED;
        boolean selectedGrant = grantView.isChecked() || grantSend.isChecked() || grantAnnotate.isChecked();

        if (stopped) {
            sessionState.setText(R.string.pairing_stopped);
            if (nativeReady && localEndpoints.isEmpty()) {
                serviceState.setText(R.string.pairing_no_address);
            }
        } else if (pending) {
            sessionState.setText("Sender approval required");
        } else if (paired) {
            sessionState.setText("Sender paired; no video session");
        } else if (waiting) {
            sessionState.setText("Waiting for sender approval request");
        } else {
            sessionState.setText("Receiver ready for a sender");
        }

        startButton.setEnabled(nativeReady && stopped && !decisionInProgress);
        stopButton.setEnabled(!stopped);
        startButton.setVisibility(stopped ? View.VISIBLE : View.GONE);
        stopButton.setVisibility(stopped ? View.GONE : View.VISIBLE);
        findViewById(R.id.pairing_columns).setVisibility(stopped ? View.GONE : View.VISIBLE);
        findViewById(R.id.pairing_code_panel).setVisibility(pending || paired ? View.GONE : View.VISIBLE);
        findViewById(R.id.pairing_controls).setVisibility(pending || paired || pairingState.canWait()
                ? View.VISIBLE : View.GONE);
        boolean hasInvitation = invitationQr.getVisibility() == View.VISIBLE && !invitationText.getText().toString().isEmpty();
        findViewById(R.id.copy_invitation_button).setEnabled(hasInvitation && !decisionInProgress);
        findViewById(R.id.share_invitation_button).setEnabled(hasInvitation && !decisionInProgress);
        approveButton.setVisibility(View.GONE);
        rejectButton.setVisibility(View.GONE);
        grantView.setVisibility(View.GONE);
        grantSend.setVisibility(View.GONE);
        grantAnnotate.setVisibility(View.GONE);
        closeChannelButton.setVisibility(paired ? View.VISIBLE : View.GONE);
        if (!pending) {
            grantView.setVisibility(View.GONE);
            grantSend.setVisibility(View.GONE);
            grantAnnotate.setVisibility(View.GONE);
        }
        waitButton.setVisibility(pairingState.canWait() ? View.VISIBLE : View.GONE);
        waitButton.setEnabled(pairingState.canWait() && !decisionInProgress);
        refreshQrButton.setVisibility(!stopped ? View.VISIBLE : View.GONE);
        refreshQrButton.setEnabled((pairingState.canWait() || waiting) && !decisionInProgress);
        approveButton.setEnabled(pending && selectedGrant && !decisionInProgress);
        rejectButton.setEnabled(pending && !decisionInProgress);
        closeChannelButton.setEnabled(paired && !decisionInProgress);
        grantView.setEnabled(pending && !decisionInProgress && grantView.getVisibility() == View.VISIBLE);
        grantSend.setEnabled(pending && !decisionInProgress && grantSend.getVisibility() == View.VISIBLE);
        grantAnnotate.setEnabled(pending && !decisionInProgress && grantAnnotate.getVisibility() == View.VISIBLE);
        if (paired) {
            helpText.setText("Waiting for the approved sender to select a screen or window. No capture starts automatically.");
        } else if (stopped) {
            helpText.setText("Connect to Wi-Fi or Ethernet, then start the receiver. A sender request will never be auto-approved.");
        } else if (waiting) {
            helpText.setText("The receiver is waiting for one verified sender request. Stop receiver cancels the wait.");
        }
    }

    private JSONObject requireSuccess(String rawResponse) throws JSONException {
        JSONObject response = new JSONObject(rawResponse);
        if (!response.optBoolean("ok")) {
            throw new IllegalStateException(response.optString("error", "Native pairing operation failed."));
        }
        return response;
    }

    private Bitmap createQrBitmap(String contents, int size) throws WriterException {
        Map<EncodeHintType, Object> hints = new TreeMap<>();
        hints.put(EncodeHintType.MARGIN, 2);
        BitMatrix matrix = new QRCodeWriter().encode(contents, BarcodeFormat.QR_CODE, size, size, hints);
        int[] pixels = new int[size * size];
        for (int y = 0; y < size; y++) {
            for (int x = 0; x < size; x++) {
                pixels[y * size + x] = matrix.get(x, y) ? Color.BLACK : Color.WHITE;
            }
        }
        Bitmap bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        bitmap.setPixels(pixels, 0, size, 0, 0, size, size);
        return bitmap;
    }

    private void showOperationFailure(String title, Throwable error) {
        runOnUiThread(() -> {
            if (destroyed) {
                return;
            }
            decisionInProgress = false;
            if (pairingState.phase() == TvPairingState.Phase.REQUEST_PENDING) {
                pairingState.decisionFailed();
            } else if (pairingState.phase() == TvPairingState.Phase.WAITING_FOR_REQUEST) {
                pairingState.waitFailed();
            }
            dialogs.notice(title, "The connection was interrupted. Stop and restart the receiver.");
            renderState();
        });
    }

    @Override
    protected void onDestroy() {
        destroyed = true;
        dialogs.close();
        stopReceiver();
        controlWorker.shutdown();
        acceptWorker.shutdown();
        super.onDestroy();
    }

    private static final class LocalEndpoint {
        private final String interfaceName;
        private final String address;

        private LocalEndpoint(String interfaceName, String address) {
            this.interfaceName = interfaceName;
            this.address = address;
        }


    }

    private static final class FreshInvitation {
        private final String uri;
        private final Bitmap bitmap;

        private FreshInvitation(String uri, Bitmap bitmap) {
            this.uri = uri;
            this.bitmap = bitmap;
        }
    }
}
