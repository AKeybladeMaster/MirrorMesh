package com.mirrormesh.tv;

import android.app.Activity;
import android.os.Bundle;
import android.widget.Button;
import android.widget.TextView;

/** Local approval-flow demonstration. It does not discover senders or stream media. */
public final class MainActivity extends Activity {
    private final PairingDemoState demoState = new PairingDemoState();
    private TextView sessionState;
    private TextView helpText;
    private Button requestButton;
    private Button acceptButton;
    private Button rejectButton;
    private Button disconnectButton;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        sessionState = findViewById(R.id.session_state);
        TextView serviceState = findViewById(R.id.service_state);
        helpText = findViewById(R.id.help_text);
        requestButton = findViewById(R.id.demo_request_button);
        acceptButton = findViewById(R.id.accept_button);
        rejectButton = findViewById(R.id.reject_button);
        disconnectButton = findViewById(R.id.stop_button);
        Button detailsButton = findViewById(R.id.details_button);

        serviceState.setText(R.string.local_demo_status);
        requestButton.setOnClickListener(view -> {
            demoState.showDemoRequest();
            renderState();
        });
        acceptButton.setOnClickListener(view -> {
            demoState.accept();
            renderState();
        });
        rejectButton.setOnClickListener(view -> {
            demoState.reject();
            renderState();
        });
        disconnectButton.setOnClickListener(view -> {
            demoState.disconnect();
            renderState();
        });
        detailsButton.setOnClickListener(view ->
                helpText.setText(R.string.signaling_details));
        renderState();
        requestButton.requestFocus();
    }

    private void renderState() {
        PairingDemoState.Phase phase = demoState.phase();
        switch (phase) {
            case REQUEST_PENDING:
                sessionState.setText(R.string.session_request_pending);
                helpText.setText(R.string.demo_request_help);
                break;
            case CONNECTED:
                sessionState.setText(R.string.session_demo_connected);
                helpText.setText(R.string.demo_connected_help);
                break;
            case REJECTED:
                sessionState.setText(R.string.session_request_rejected);
                helpText.setText(R.string.demo_rejected_help);
                break;
            case STANDBY:
            default:
                sessionState.setText(R.string.session_standby);
                helpText.setText(R.string.demo_standby_help);
                break;
        }

        boolean requestPending = phase == PairingDemoState.Phase.REQUEST_PENDING;
        requestButton.setEnabled(phase == PairingDemoState.Phase.STANDBY
                || phase == PairingDemoState.Phase.REJECTED);
        acceptButton.setEnabled(requestPending);
        rejectButton.setEnabled(requestPending);
        disconnectButton.setEnabled(demoState.isConnected());
    }
}
