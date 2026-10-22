# Install MirrorMesh on an Android TV

Use **`releases/android-tv/MirrorMesh-tv-arm64-v8a-debug.apk`** for the installable preview.
Keep the matching Windows 0.4.0 or newer app. The optimized `release-unsigned.apk`
cannot be installed until it is signed. The TV build has compiled, but the owner has
not yet verified it on a physical TV.

This APK targets **Android TV / Google TV with an ARM64 Android OS**. Samsung Tizen,
LG webOS and other non-Android TVs cannot install it. A 64-bit processor does not
guarantee a 64-bit TV OS; `INSTALL_FAILED_NO_MATCHING_ABIS` means a different build is needed.

## USB flash drive (simple option)

1. Copy the TV debug APK above onto a USB flash drive using your PC or phone plus an OTG adapter.
2. Plug the drive into the TV. Open a TV file manager and select the APK.
3. If asked, allow **Install unknown apps** for that file manager, then install.
4. Open MirrorMesh from the TV apps list. If it is absent, open it from Settings → Apps.

A phone USB cable works only if the TV's file manager supports browsing an MTP phone.
Select **File transfer** on the phone if supported. Most TV USB ports are intended for
USB storage; a cable alone does not automatically install a phone's APK on the TV.
Copying from the phone to a USB drive is the more predictable cable-based route.

## Wireless ADB from the PC

Use only the debugging options offered by the TV, on your trusted LAN. The SDK here has
ADB at `.toolchains/android-sdk/platform-tools/adb.exe`.

On Android TV 13 or later, the TV may provide **Developer options → Wireless debugging**.
Choose **Pair device with pairing code**, then use the addresses and ports shown on the TV:

```powershell
$adb = '.\.toolchains\android-sdk\platform-tools\adb.exe'
& $adb pair TV_IP:PAIRING_PORT
# Enter the code shown by the TV when prompted.
& $adb connect TV_IP:DEBUG_PORT
& $adb -s TV_IP:DEBUG_PORT install -r '.\releases\android-tv\MirrorMesh-tv-arm64-v8a-debug.apk'
```

Pairing and debugging ports are usually different. Replace the placeholders with the
TV's values; do not paste a literal `TV_IP`. Some older TVs expose a separate network
debugging switch and port 5555. Use `adb connect TV_IP:5555` only when that TV explicitly
offers network debugging. Otherwise use USB storage or its supported transfer method.

Direct PC-to-TV USB debugging needs a TV port that supports USB device/debug mode.
Common TV USB-A storage ports do not provide this. **Do not use a USB-A-to-USB-A cable.**

See [Android's official ADB guide](https://developer.android.com/tools/adb) for pairing
and APK installation. Turn off debugging after installation if you no longer need it.

## Phone transfer over Wi-Fi

A TV file manager with an authenticated LAN transfer function can receive the APK from
your phone. Put both on the same LAN, transfer the **TV** APK, then open it on the TV
and follow the installation prompts. Use the transfer tool's documented procedure.
Do not expose a temporary transfer server to the Internet.

## First PC → TV session

1. Connect PC and TV to the same trusted LAN; guest/client isolation can prevent sharing.
2. In TV MirrorMesh, start receiving.
3. On PC, choose **Find devices** and select the TV, or copy the TV's current invitation
   through your phone into the PC app.
4. Compare the connection code when requested. On the TV, check the reported device and
   select **Allow view** before **Approve**.
5. Choose a PC screen or window to share. Use Stop/Disconnect to end the session.

TV → PC additionally needs the TV manufacturer's support for Android's screen-capture
permission. It cannot capture protected content. A successful APK build does not establish
that a particular TV permits capture or decodes this stream.
