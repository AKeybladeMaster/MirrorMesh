package com.mirrormesh.tv;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.LinkAddress;
import android.net.LinkProperties;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.os.Build;
import android.provider.Settings;
import java.net.Inet4Address;
import java.net.InetAddress;
import org.json.JSONObject;

/** Public, sender-reported OS facts. No serial, account or hardware identifiers. */
final class DeviceContext {
    static String clean(String value) {
        if (value == null) return "";
        StringBuilder output = new StringBuilder();
        int scalars = 0;
        for (int index = 0; index < value.length() && scalars < 80;) {
            int c = value.codePointAt(index);
            index += Character.charCount(c);
            if (Character.isISOControl(c) || c == 0x061c || c == 0x200e || c == 0x200f
                    || (c >= 0x202a && c <= 0x202e) || (c >= 0x2066 && c <= 0x2069)
                    || (c >= 0xd800 && c <= 0xdfff)) continue;
            output.appendCodePoint(c);
            scalars++;
        }
        return output.toString().trim();
    }

    static String name(Context context) {
        String value = null;
        try { value = Settings.Global.getString(context.getContentResolver(), "device_name"); }
        catch (SecurityException ignored) { }
        value = clean(value);
        return value.isEmpty() ? clean(Build.MANUFACTURER + " " + Build.MODEL) : value;
    }

    static void configure(Context context) throws Exception {
        SenderActivity.command(new JSONObject().put("op", "configure_device")
                .put("name", name(context)).put("platform", context.getPackageName().endsWith(".tv") ? "Android TV" : "Android")
                .put("manufacturer", clean(Build.MANUFACTURER)).put("model", clean(Build.MODEL))
                .put("os_version", clean("Android " + Build.VERSION.RELEASE)));
    }

    static String lanAddress(Context context) {
        ConnectivityManager manager = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        if (manager == null) return null;
        Network network = null;
        if (Build.VERSION.SDK_INT >= 23) {
            network = manager.getActiveNetwork();
        } else {
            android.net.NetworkInfo active = manager.getActiveNetworkInfo();
            if (active == null || !active.isConnected()) return null;
            for (Network candidate : manager.getAllNetworks()) {
                android.net.NetworkInfo info = manager.getNetworkInfo(candidate);
                if (info != null && info.isConnected() && info.getType() == active.getType()) {
                    network = candidate; break;
                }
            }
        }
        if (network == null) return null;
        NetworkCapabilities capabilities = manager.getNetworkCapabilities(network);
        LinkProperties properties = manager.getLinkProperties(network);
        if (capabilities == null || properties == null || capabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
                || (!capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)
                    && !capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET))) return null;
        for (LinkAddress link : properties.getLinkAddresses()) {
            InetAddress address = link.getAddress();
            if (address instanceof Inet4Address && !address.isLoopbackAddress() && !address.isLinkLocalAddress()
                    && !address.isAnyLocalAddress() && !address.isMulticastAddress()) return address.getHostAddress();
        }
        return null;
    }

    private DeviceContext() { }
}
