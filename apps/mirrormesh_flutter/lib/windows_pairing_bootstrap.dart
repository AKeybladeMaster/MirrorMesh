import 'package:flutter/services.dart';

/// Requests lazy Windows pairing-runtime initialization after explicit user action.
class WindowsPairingBootstrap {
  const WindowsPairingBootstrap() : _channel = const MethodChannel(channelName);

  static const channelName = 'com.mirrormesh/windows_pairing';
  static const _prepareMethod = 'preparePairingRuntime';

  final MethodChannel _channel;

  Future<String> prepare() async {
    final deviceId = await _channel.invokeMethod<String>(_prepareMethod);
    if (deviceId == null || deviceId.isEmpty) {
      throw PlatformException(
        code: 'pairing_runtime_unavailable',
        message: 'Windows did not return a local device ID.',
      );
    }
    return deviceId;
  }
}
