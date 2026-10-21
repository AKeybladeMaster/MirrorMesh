package com.mirrormesh.tv;

import android.content.Context;
import android.net.nsd.NsdManager;
import android.net.nsd.NsdServiceInfo;

/** Public discovery hints only. Never publish invitations, fingerprints, nonces or approval. */
final class ReceiverAdvertisement {
    private final NsdManager manager;
    private NsdManager.RegistrationListener active;

    ReceiverAdvertisement(Context context) {
        manager = (NsdManager) context.getSystemService(Context.NSD_SERVICE);
    }

    void start(String id, String name) {
        stop();
        if (manager == null) return;
        NsdServiceInfo info = new NsdServiceInfo();
        info.setServiceName(name);
        info.setServiceType("_mirrormesh._tcp.");
        info.setPort(8765);
        info.setAttribute("v", "1"); info.setAttribute("id", id);
        info.setAttribute("roles", "recv"); info.setAttribute("pair", "qr");
        info.setAttribute("tls", "1"); info.setAttribute("video", "h264");
        info.setAttribute("audio", "none"); info.setAttribute("max", "1920x1080@60");
        NsdManager.RegistrationListener listener = new NsdManager.RegistrationListener() {
            @Override public void onServiceRegistered(NsdServiceInfo service) { }
            @Override public void onRegistrationFailed(NsdServiceInfo service, int error) { }
            @Override public void onServiceUnregistered(NsdServiceInfo service) { }
            @Override public void onUnregistrationFailed(NsdServiceInfo service, int error) { }
        };
        active = listener;
        try { manager.registerService(info, NsdManager.PROTOCOL_DNS_SD, listener); }
        catch (RuntimeException failure) { active = null; }
    }

    void stop() {
        NsdManager.RegistrationListener listener = active;
        active = null;
        if (manager != null && listener != null) {
            try { manager.unregisterService(listener); } catch (RuntimeException ignored) { }
        }
    }
}
