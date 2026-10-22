import 'dart:io';

import 'package:flutter/services.dart';

/// Window and process controls only; media stays in native renderers.
abstract final class ReceiverWindow {
  static const channel = MethodChannel('com.mirrormesh/receiver_window');

  static Future<void> setFullscreen(bool enabled) async {
    if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
      await channel.invokeMethod<void>('fullscreen', enabled);
    }
  }
}
