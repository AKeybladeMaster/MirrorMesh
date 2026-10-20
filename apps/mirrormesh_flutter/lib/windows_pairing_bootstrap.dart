import 'dart:convert';

import 'package:flutter/services.dart';

/// Requests lazy Windows pairing-runtime initialization after explicit user action.
class WindowsPairingBootstrap {
  const WindowsPairingBootstrap() : _channel = const MethodChannel(channelName);

  static const channelName = 'com.mirrormesh/windows_pairing';
  static const _prepareMethod = 'preparePairingRuntime';
  static const _beginSenderPairingMethod = 'beginSenderPairing';
  static const _pollSenderPairingMethod = 'pollSenderPairing';
  static const _cancelSenderPairingMethod = 'cancelSenderPairing';
  static const _beginSenderSessionMethod = 'beginSenderSession';
  static const _startSenderSessionMethod = 'startSenderSession';
  static const _stopSenderSessionMethod = 'stopSenderSession';
  static const _sendSenderSessionIceMethod = 'sendSenderSessionIce';
  static const _pollSenderSessionEventMethod = 'pollSenderSessionEvent';

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

  Future<String> beginSenderSession({
    required int requestId,
    required String sdp,
  }) async {
    final sessionId = await _channel.invokeMethod<String>(
      _beginSenderSessionMethod,
      <String, Object?>{'requestId': requestId, 'sdp': sdp},
    );
    if (sessionId == null || sessionId.isEmpty) {
      throw PlatformException(
        code: 'session_unavailable',
        message: 'Windows did not start the sender session.',
      );
    }
    return sessionId;
  }

  Future<void> sendSenderSessionIce({
    required int requestId,
    required String candidate,
    required String mediaId,
    required int mediaLineIndex,
  }) async {
    await _channel.invokeMethod<bool>(
      _sendSenderSessionIceMethod,
      <String, Object?>{
        'requestId': requestId,
        'candidate': candidate,
        'mediaId': mediaId,
        'mediaLineIndex': mediaLineIndex,
      },
    );
  }

  Future<void> startSenderSession({
    required int requestId,
    required String sourceId,
  }) async {
    final started = await _channel.invokeMethod<bool>(
      _startSenderSessionMethod,
      <String, Object?>{'requestId': requestId, 'sourceId': sourceId},
    );
    if (started != true) {
      throw PlatformException(
        code: 'session_start_unavailable',
        message: 'Windows did not start the approved sender source.',
      );
    }
  }

  Future<void> stopSenderSession({
    required int requestId,
    required String reason,
  }) async {
    final stopped = await _channel.invokeMethod<bool>(
      _stopSenderSessionMethod,
      <String, Object?>{'requestId': requestId, 'reason': reason},
    );
    if (stopped != true) {
      throw PlatformException(
        code: 'session_stop_unavailable',
        message: 'Windows could not send the session stop message.',
      );
    }
  }

  Future<WindowsSenderSessionEvent> pollSenderSessionEvent(
    int requestId,
  ) async {
    final encoded = await _channel.invokeMethod<String>(
      _pollSenderSessionEventMethod,
      requestId,
    );
    if (encoded == null) {
      throw PlatformException(
        code: 'session_event_invalid',
        message: 'Windows returned no sender session event.',
      );
    }
    final decoded = jsonDecode(encoded);
    if (decoded is! Map<String, dynamic> || decoded['pending'] is! bool) {
      throw PlatformException(
        code: 'session_event_invalid',
        message: 'Windows returned an invalid sender session event.',
      );
    }
    final pending = decoded['pending']! as bool;
    if (pending) return const WindowsSenderSessionEvent.pending();

    final kind = decoded['kind'];
    final sessionId = decoded['sessionId'];
    if (kind is! String) {
      throw PlatformException(
        code: 'session_event_invalid',
        message: 'Windows returned an invalid sender session event kind.',
      );
    }
    return switch (kind) {
      'answer' when sessionId is String && decoded['sdp'] is String =>
        WindowsSenderSessionEvent.answer(
          sessionId: sessionId,
          sdp: decoded['sdp']! as String,
        ),
      'ice'
          when sessionId is String &&
              decoded['candidate'] is String &&
              decoded['mediaId'] is String &&
              decoded['mediaLineIndex'] is int =>
        WindowsSenderSessionEvent.ice(
          sessionId: sessionId,
          candidate: decoded['candidate']! as String,
          mediaId: decoded['mediaId']! as String,
          mediaLineIndex: decoded['mediaLineIndex']! as int,
        ),
      'stop' when sessionId is String && decoded['reason'] is String =>
        WindowsSenderSessionEvent.stopped(
          sessionId: sessionId,
          reason: decoded['reason']! as String,
        ),
      'revoke' when decoded['peerId'] is String =>
        WindowsSenderSessionEvent.revoked(peerId: decoded['peerId']! as String),
      _ => throw PlatformException(
        code: 'session_event_invalid',
        message: 'Windows returned an invalid sender session event payload.',
      ),
    };
  }
}

enum WindowsSenderSessionEventKind { pending, answer, ice, stopped, revoked }

class WindowsSenderSessionEvent {
  const WindowsSenderSessionEvent.pending()
    : kind = WindowsSenderSessionEventKind.pending,
      sessionId = null,
      sdp = null,
      candidate = null,
      mediaId = null,
      mediaLineIndex = null,
      reason = null,
      peerId = null;

  const WindowsSenderSessionEvent.answer({
    required this.sessionId,
    required this.sdp,
  }) : kind = WindowsSenderSessionEventKind.answer,
       candidate = null,
       mediaId = null,
       mediaLineIndex = null,
       reason = null,
       peerId = null;

  const WindowsSenderSessionEvent.ice({
    required this.sessionId,
    required this.candidate,
    required this.mediaId,
    required this.mediaLineIndex,
  }) : kind = WindowsSenderSessionEventKind.ice,
       sdp = null,
       reason = null,
       peerId = null;

  const WindowsSenderSessionEvent.stopped({
    required this.sessionId,
    required this.reason,
  }) : kind = WindowsSenderSessionEventKind.stopped,
       sdp = null,
       candidate = null,
       mediaId = null,
       mediaLineIndex = null,
       peerId = null;

  const WindowsSenderSessionEvent.revoked({required this.peerId})
    : kind = WindowsSenderSessionEventKind.revoked,
      sessionId = null,
      sdp = null,
      candidate = null,
      mediaId = null,
      mediaLineIndex = null,
      reason = null;

  final WindowsSenderSessionEventKind kind;
  final String? sessionId;
  final String? sdp;
  final String? candidate;
  final String? mediaId;
  final int? mediaLineIndex;
  final String? reason;
  final String? peerId;
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

  String? get failureDescription => switch (errorCode) {
    0 => null,
    1 => 'Windows could not open its protected device identity or trust store.',
    2 =>
      'The receiver rejected pairing. Refresh its invitation before retrying.',
    3 => 'The authenticated signaling connection failed. Check the receiver status.',
    4 => 'The receiver closed the connection before approval. Check its status and refresh the invitation.',
    5 => 'The receiver did not approve screen viewing. Select Allow view and approve the new request.',
    6 => 'Windows could not complete a local pairing operation.',
    7 => 'The approved signaling channel failed.',
    10 => 'The invitation format is invalid. Copy the entire mirrormesh://pair link, not a browser search or sharing-page URL.',
    11 => 'This invitation has expired. Refresh the code on Android, then immediately copy and paste the new link.',
    12 => 'The invitation expiry is ahead of the allowed window. Enable automatic date and time on both devices, then refresh the code.',
    13 => 'The receiver refused the connection. Start the receiver and check its selected Wi-Fi or Ethernet address.',
    14 => 'The receiver did not answer in time. Check that both devices are on the same LAN and the receiver is listening.',
    15 => 'Windows could not reach the receiver. Check the network, selected address and app-specific firewall access.',
    16 => 'The secure TLS/WebSocket handshake failed. Use a fresh invitation from this receiver and check its status. Identity checks remain enforced.',
    17 => 'Windows could not sign its pairing request with the local device identity.',
    18 => 'The receiver identity did not match the invitation. Pairing was blocked.',
    19 => 'The receiver returned an invalid pairing response. Update both apps to the same build.',
    20 => 'The pinned TLS connection could not be initialized.',
    21 => 'The invitation expired while waiting for approval. Refresh the receiver code and approve the next request promptly.',
    _ => 'Pairing failed with an unrecognized native status. Update both apps to the same build.',
  };
}
