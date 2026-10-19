import 'package:flutter/services.dart';

/// Requests lazy Linux pairing-runtime initialization after an explicit action.
class LinuxPairingBootstrap {
  const LinuxPairingBootstrap() : _channel = const MethodChannel(channelName);

  static const channelName = 'com.mirrormesh/linux_pairing';
  static const _prepareMethod = 'preparePairingRuntime';

  final MethodChannel _channel;

  Future<String> prepare() async {
    final deviceId = await _channel.invokeMethod<String>(_prepareMethod);
    if (deviceId == null || deviceId.isEmpty) {
      throw PlatformException(
        code: 'pairing_runtime_unavailable',
        message: 'Linux did not return a local device ID.',
      );
    }
    return deviceId;
  }
}
