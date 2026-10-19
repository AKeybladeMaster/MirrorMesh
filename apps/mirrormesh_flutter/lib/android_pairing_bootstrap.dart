import 'package:flutter/services.dart';

/// Requests lazy Android pairing-runtime initialization after an explicit user action.
class AndroidPairingBootstrap {
  const AndroidPairingBootstrap() : _channel = const MethodChannel(channelName);

  static const channelName = 'com.mirrormesh/android_pairing';
  static const _prepareMethod = 'preparePairingRuntime';

  final MethodChannel _channel;

  Future<String> prepare() async {
    final deviceId = await _channel.invokeMethod<String>(_prepareMethod);
    if (deviceId == null || deviceId.isEmpty) {
      throw PlatformException(
        code: 'pairing_runtime_unavailable',
        message: 'Android did not return a local device ID.',
      );
    }
    return deviceId;
  }
}
