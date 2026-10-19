package com.mirrormesh.tv;

import android.app.Activity;
import android.graphics.Bitmap;
import android.graphics.Color;
import android.os.Bundle;
import android.view.View;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.ImageView;
import android.widget.Spinner;
import android.widget.TextView;

import com.google.zxing.BarcodeFormat;
import com.google.zxing.EncodeHintType;
import com.google.zxing.WriterException;
import com.google.zxing.common.BitMatrix;
import com.google.zxing.qrcode.QRCodeWriter;

import org.json.JSONException;
import org.json.JSONObject;

import java.net.Inet4Address;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.net.SocketException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Enumeration;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Native TV pairing UI. Approved pairing opens signaling only; media is not connected yet. */
public final class MainActivity extends Activity {
    private static final int PAIRING_PORT = 8765;

    private final TvPairingState pairingState = new TvPairingState();
    private final ExecutorService controlWorker = Executors.newSingleThreadExecutor();
    private final ExecutorService acceptWorker = Executors.newSingleThreadExecutor();
    private final List<LocalEndpoint> localEndpoints = new ArrayList<>();

    private Spinner endpointSpinner;
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

    private boolean nativeReady;
    private boolean decisionInProgress;
    private boolean destroyed;
    private int serviceGeneration;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        bindViews();
        configureEndpoints();
        initializeNativeContext();

        startButton.setOnClickListener(view -> startReceiver());
        stopButton.setOnClickListener(view -> stopReceiver());
        waitButton.setOnClickListener(view -> waitForSender());
        approveButton.setOnClickListener(view -> approveRequest());
        rejectButton.setOnClickListener(view -> rejectRequest());
        closeChannelButton.setOnClickListener(view -> closeChannel());
        refreshQrButton.setOnClickListener(view -> refreshInvitation());
        grantView.setOnCheckedChangeListener((button, checked) -> renderState());
        grantSend.setOnCheckedChangeListener((button, checked) -> renderState());
        grantAnnotate.setOnCheckedChangeListener((button, checked) -> renderState());
        renderState();
        startButton.requestFocus();
    }

    private void bindViews() {
        endpointSpinner = findViewById(R.id.endpoint_spinner);
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
            Enumeration<NetworkInterface> interfaces = NetworkInterface.getNetworkInterfaces();
            if (interfaces != null) {
                while (interfaces.hasMoreElements()) {
                    NetworkInterface networkInterface = interfaces.nextElement();
                    if (!networkInterface.isUp() || networkInterface.isLoopback()) {
                        continue;
                    }
                    Enumeration<InetAddress> addresses = networkInterface.getInetAddresses();
                    while (addresses.hasMoreElements()) {
                        InetAddress address = addresses.nextElement();
                        if (!(address instanceof Inet4Address)
                                || address.isAnyLocalAddress()
                                || address.isLoopbackAddress()
                                || address.isLinkLocalAddress()) {
                            continue;
                        }
                        String numericAddress = address.getHostAddress();
                        localEndpoints.add(new LocalEndpoint(
                                networkInterface.getDisplayName(), numericAddress));
                    }
                }
            }
        } catch (SocketException error) {
            serviceState.setText("The TV network addresses could not be listed: " + error.getMessage());
        }
        Collections.sort(localEndpoints, (left, right) ->
                left.address.compareTo(right.address));

        ArrayAdapter<LocalEndpoint> adapter = new ArrayAdapter<>(
                this, android.R.layout.simple_spinner_item, localEndpoints);
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        endpointSpinner.setAdapter(adapter);
        if (localEndpoints.isEmpty()) {
            serviceState.setText(R.string.pairing_no_address);
        }
    }

    private void startReceiver() {
        if (!nativeReady || !pairingState.canStart() || localEndpoints.isEmpty()) {
            return;
        }
        LocalEndpoint endpoint = (LocalEndpoint) endpointSpinner.getSelectedItem();
        if (endpoint == null) {
            serviceState.setText(R.string.pairing_no_address);
            return;
        }
        startButton.setEnabled(false);
        serviceState.setText("Starting the authenticated receiver on " + endpoint.address + "...");
        int startGeneration = serviceGeneration;
        controlWorker.execute(() -> {
            try {
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
                    invitationText.setVisibility(View.VISIBLE);
                    pairingState.receiverStarted();
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
                        serviceState.setText("Receiver start failed: "
                                + (error.getMessage() == null ? "Unknown error" : error.getMessage()));
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
        requestDetails.setText("Waiting for a verified WSS pairing request on port "
                + PAIRING_PORT + ".");
        serviceState.setText("Pairing is active. No screen video is connected.");
        renderState();
        acceptWorker.execute(() -> {
            try {
                JSONObject request = requireSuccess(NativePairingBridge.waitForPairingRequest());
                runOnUiThread(() -> showPendingRequest(request));
            } catch (Exception | LinkageError error) {
                runOnUiThread(() -> {
                    if (destroyed || pairingState.phase() != TvPairingState.Phase.WAITING_FOR_REQUEST) {
                        return;
                    }
                    String message = error.getMessage() == null ? "Pairing wait failed" : error.getMessage();
                    if (message.contains("stopped")) {
                        pairingState.receiverStopped();
                        clearInvitation();
                        serviceState.setText(R.string.pairing_stopped);
                    } else {
                        pairingState.waitFailed();
                        serviceState.setText(message);
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
        long requestId = pairingState.requestId();
        String name = request.optString("display_name", "Unknown sender");
        String sourceIp = request.optString("source_ip", "Unknown address");
        String fingerprint = request.optString("fingerprint", "Unavailable");
        JSONObject grants = request.optJSONObject("requested_grants");
        boolean requestedView = grants != null && grants.optBoolean("view");
        boolean requestedSend = grants != null && grants.optBoolean("send");
        boolean requestedAnnotate = grants != null && grants.optBoolean("annotate");

        grantView.setChecked(false);
        grantSend.setChecked(false);
        grantAnnotate.setChecked(false);
        grantView.setVisibility(requestedView ? View.VISIBLE : View.GONE);
        grantSend.setVisibility(requestedSend ? View.VISIBLE : View.GONE);
        grantAnnotate.setVisibility(requestedAnnotate ? View.VISIBLE : View.GONE);
        invitationQr.setVisibility(View.GONE);
        invitationText.setVisibility(View.GONE);
        grantView.setEnabled(requestedView);
        grantSend.setEnabled(requestedSend);
        grantAnnotate.setEnabled(requestedAnnotate);
        requestDetails.setText("Sender: " + name
                + "\nSource address: " + sourceIp
                + "\nKey fingerprint: " + fingerprint
                + "\nRequest ID: " + requestId);
        serviceState.setText(R.string.pairing_request_pending);
        helpText.setText("Permission boxes are unchecked. Approve only permissions you intend to grant.");
        renderState();
        if (requestedView) {
            grantView.requestFocus();
        } else {
            rejectButton.requestFocus();
        }
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
        renderState();
        long requestId = pairingState.requestId();
        controlWorker.execute(() -> {
            try {
                JSONObject result = requireSuccess(NativePairingBridge.approvePairing(
                        requestId, view, send, annotate));
                long connectionId = result.getLong("connection_id");
                String peerId = result.optString("peer_id", "paired peer");
                runOnUiThread(() -> {
                    if (destroyed) {
                        return;
                    }
                    decisionInProgress = false;
                    if (!pairingState.requestApproved(connectionId)) {
                        return;
                    }
                    requestDetails.setText("Paired sender: " + peerId
                            + "\nAuthenticated control channel is open.\nVideo streaming is not connected yet.");
                    invitationQr.setVisibility(View.GONE);
                    invitationText.setVisibility(View.GONE);
                    serviceState.setText(R.string.pairing_paired);
                    helpText.setText("Close channel ends this connection. The approved peer remains trusted.");
                    clearGrantSelection();
                    renderState();
                    closeChannelButton.requestFocus();
                });
            } catch (Exception | LinkageError error) {
                runOnUiThread(() -> {
                    decisionInProgress = false;
                    if (pairingState.decisionFailed()) {
                        serviceState.setText(error.getMessage() == null
                                ? "Approval failed" : error.getMessage());
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
                        serviceState.setText(error.getMessage() == null
                                ? "Rejection failed" : error.getMessage());
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
        renderState();
        long connectionId = pairingState.connectionId();
        controlWorker.execute(() -> {
            try {
                if (!NativePairingBridge.closeControlChannel(connectionId)) {
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

    private void refreshInvitation() {
        TvPairingState.Phase phase = pairingState.phase();
        boolean cancelWait = phase == TvPairingState.Phase.WAITING_FOR_REQUEST;
        if (decisionInProgress || (!cancelWait && phase != TvPairingState.Phase.READY)) {
            return;
        }
        if (cancelWait) {
            pairingState.waitFailed();
        }
        decisionInProgress = true;
        serviceState.setText("Refreshing the one-time pairing code...");
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
                    displayInvitation(freshInvitation);
                    requestDetails.setText("Fresh invitation created. Select Wait for sender to listen.");
                    serviceState.setText(R.string.pairing_ready);
                    renderState();
                    refreshQrButton.requestFocus();
                });
            } catch (Exception | LinkageError error) {
                runOnUiThread(() -> {
                    if (destroyed || refreshGeneration != serviceGeneration) {
                        return;
                    }
                    decisionInProgress = false;
                    serviceState.setText("Pairing code refresh failed: "
                            + (error.getMessage() == null ? "Unknown error" : error.getMessage()));
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
        invitationText.setVisibility(View.VISIBLE);
    }

    private void stopReceiver() {
        boolean wasActive = pairingState.phase() != TvPairingState.Phase.STOPPED;
        serviceGeneration++;
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

        endpointSpinner.setEnabled(stopped && !localEndpoints.isEmpty());
        startButton.setEnabled(nativeReady && stopped && !localEndpoints.isEmpty());
        stopButton.setEnabled(!stopped);
        waitButton.setVisibility(!stopped ? View.VISIBLE : View.GONE);
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
            helpText.setText("This is an authenticated pairing control channel only. No media is being received.");
        } else if (stopped) {
            helpText.setText("Choose an active local address, then start the receiver. A sender request will never be auto-approved.");
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
            serviceState.setText(title + ": "
                    + (error.getMessage() == null ? "Unknown error" : error.getMessage()));
            renderState();
        });
    }

    @Override
    protected void onDestroy() {
        destroyed = true;
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

        @Override
        public String toString() {
            return interfaceName + " - " + address + ":" + PAIRING_PORT;
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
