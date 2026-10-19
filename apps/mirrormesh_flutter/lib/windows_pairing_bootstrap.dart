import 'package:flutter/services.dart';

/// Requests lazy Windows pairing-runtime initialization after explicit user action.
class WindowsPairingBootstrap {
  const WindowsPairingBootstrap() : _channel = const MethodChannel(channelName);

  static const channelName = 'com.mirrormesh/windows_pairing';
  static const _prepareMethod = 'preparePairingRuntime';
  static const _beginSenderPairingMethod = 'beginSenderPairing';
  static const _pollSenderPairingMethod = 'pollSenderPairing';
  static const _cancelSenderPairingMethod = 'cancelSenderPairing';

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

  Future<int> beginSenderPairing(String invitationUri) async {
    final requestId = await _channel.invokeMethod<int>(
      _beginSenderPairingMethod,
      invitationUri,
    );
    if (requestId == null || requestId <= 0) {
      throw PlatformException(
        code: 'pairing_unavailable',
        message: 'Windows did not start sender pairing.',
      );
    }
    return requestId;
  }

  Future<WindowsSenderPairingStatus> pollSenderPairing(int requestId) async {
    final values = await _channel.invokeMapMethod<String, Object?>(
      _pollSenderPairingMethod,
      requestId,
    );
    final state = values?['state'];
    final errorCode = values?['errorCode'];
    final fingerprint = values?['receiverFingerprint'];
    if (state is! int || errorCode is! int || fingerprint is! String) {
      throw PlatformException(
        code: 'pairing_status_invalid',
        message: 'Windows returned invalid sender pairing status.',
      );
    }
    return WindowsSenderPairingStatus(
      state: WindowsSenderPairingState.fromNative(state),
      errorCode: errorCode,
      receiverFingerprint: fingerprint,
    );
  }

  Future<void> cancelSenderPairing(int requestId) async {
    await _channel.invokeMethod<bool>(_cancelSenderPairingMethod, requestId);
  }
}

enum WindowsSenderPairingState {
  connecting(1),
  awaitingApproval(2),
  approved(3),
  rejected(4),
  failed(5),
  cancelling(6),
  cancelled(7);

  const WindowsSenderPairingState(this.nativeValue);

  final int nativeValue;

  static WindowsSenderPairingState fromNative(int value) =>
      WindowsSenderPairingState.values.firstWhere(
        (state) => state.nativeValue == value,
        orElse: () => WindowsSenderPairingState.failed,
      );

  bool get isActive => switch (this) {
    connecting || awaitingApproval || approved || cancelling => true,
    rejected || failed || cancelled => false,
  };
}

class WindowsSenderPairingStatus {
  const WindowsSenderPairingStatus({
    required this.state,
    required this.errorCode,
    required this.receiverFingerprint,
  });

  final WindowsSenderPairingState state;
  final int errorCode;
  final String receiverFingerprint;
}
