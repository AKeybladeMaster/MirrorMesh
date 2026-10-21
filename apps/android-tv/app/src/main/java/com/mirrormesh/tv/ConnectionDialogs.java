package com.mirrormesh.tv;

import android.app.Activity;
import android.app.AlertDialog;
import java.util.ArrayList;
import java.util.List;

/** Activity-owned dialogs; cleanup cannot dismiss another operation's window. */
final class ConnectionDialogs {
    private final Activity activity;
    private final List<AlertDialog> open = new ArrayList<>();
    private AlertDialog progress;
    ConnectionDialogs(Activity activity) { this.activity = activity; }
    AlertDialog show(AlertDialog dialog) {
        open.add(dialog);
        dialog.setOnDismissListener(ignored -> open.remove(dialog));
        dialog.show();
        return dialog;
    }
    void progress(String title, String message) {
        progress(title, message, null);
    }
    void progress(String title, String message, Runnable cancel) {
        endProgress();
        if (activity.isFinishing() || activity.isDestroyed()) return;
        AlertDialog.Builder builder = new AlertDialog.Builder(activity).setTitle(title).setMessage(message).setCancelable(false);
        if (cancel != null) builder.setNegativeButton("Cancel", (dialog, which) -> cancel.run());
        progress = builder.create();
        show(progress);
    }
    void endProgress() { if (progress != null) { progress.dismiss(); progress = null; } }
    void notice(String title, String message) {
        endProgress();
        if (activity.isFinishing() || activity.isDestroyed()) return;
        show(new AlertDialog.Builder(activity).setTitle(title).setMessage(message).setPositiveButton("Done", null).create());
    }
    void close() {
        for (AlertDialog dialog : new ArrayList<>(open)) dialog.dismiss();
        progress = null;
    }
}
