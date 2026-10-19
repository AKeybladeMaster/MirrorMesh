import 'package:flutter/services.dart';

/// Requests lazy iOS pairing-runtime initialization after an explicit user action.
class IosPairingBootstrap {
  const IosPairingBootstrap() : _channel = const MethodChannel(channelName);

  static const channelName = 'com.mirrormesh/ios_pairing';
  static const _prepareMethod = 'preparePairingRuntime';

  final MethodChannel _channel;

  Future<String> prepare() async {
    final deviceId = await _channel.invokeMethod<String>(_prepareMethod);
    if (deviceId == null || deviceId.isEmpty) {
      throw PlatformException(
        code: 'pairing_runtime_unavailable',
        message: 'iOS did not return a local device ID.',
      );
    }
    return deviceId;
  }
}
