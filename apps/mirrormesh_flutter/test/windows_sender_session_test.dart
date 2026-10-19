import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/webrtc_engine.dart';
import 'package:mirrormesh_flutter/windows_pairing_bootstrap.dart';
import 'package:mirrormesh_flutter/windows_sender_session.dart';

void main() {
  const channel = MethodChannel(WindowsPairingBootstrap.channelName);
  const sessionId = '8b3d8788-1476-4aae-a287-334a428b5b6f';

  TestWidgetsFlutterBinding.ensureInitialized();

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test(
    'negotiates a receive-ready peer and closes on stop or revoke',
    () async {
      final calls = <MethodCall>[];
      final events = <String>[
        jsonEncode(<String, Object?>{
          'pending': false,
          'kind': 'answer',
          'sessionId': sessionId,
          'sdp': 'v=0\r\n',
        }),
        jsonEncode(<String, Object?>{
          'pending': false,
          'kind': 'ice',
          'sessionId': sessionId,
          'candidate': 'candidate:2 1 UDP 1 192.0.2.2 5001 typ host',
          'mediaId': '0',
          'mediaLineIndex': 0,
        }),
      ];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'beginSenderSession' => sessionId,
              'sendSenderSessionIce' => true,
              'pollSenderSessionEvent' =>
                events.isEmpty
                    ? jsonEncode(<String, Object?>{'pending': true})
                    : events.removeAt(0),
              'cancelSenderPairing' => true,
              _ => null,
            };
          });

      final peer = _FakePeer();
      final controller = WindowsSenderSessionController(
        pairing: const WindowsPairingBootstrap(),
        factory: _FakeFactory(peer),
      );
      final startedSessionId = await controller.start(requestId: 41);
      peer.emitLocalCandidate(
        const WebRtcIceCandidate(
          candidate: 'candidate:1 1 UDP 1 192.0.2.1 5000 typ host',
          sdpMid: '0',
          sdpMLineIndex: 0,
        ),
      );
      peer.emitState(WebRtcConnectionState.connected);
      await Future<void>.delayed(const Duration(milliseconds: 240));

      expect(startedSessionId, sessionId);
      expect(peer.addSendOnlyVideoTransceiverCalls, 1);
      expect(peer.localDescription, 'v=0\r\n');
      expect(peer.remoteDescription, 'v=0\r\n');
      expect(peer.addedCandidates, [
        const WebRtcIceCandidate(
          candidate: 'candidate:2 1 UDP 1 192.0.2.2 5001 typ host',
          sdpMid: '0',
          sdpMLineIndex: 0,
        ),
      ]);
      expect(
        calls.where((call) => call.method == 'sendSenderSessionIce'),
        hasLength(1),
      );
      expect(controller.state, WindowsSenderSessionState.transportConnected);

      await controller.dispose();
      expect(peer.closeCalls, 1);
      expect(
        calls.where((call) => call.method == 'cancelSenderPairing'),
        hasLength(1),
      );
    },
  );
}

final class _FakeFactory implements WebRtcPeerConnectionFactory {
  const _FakeFactory(this.peer);

  final _FakePeer peer;

  @override
  Future<WebRtcPeerConnectionPort> create() async => peer;
}

final class _FakePeer implements WebRtcPeerConnectionPort {
  final StreamController<WebRtcIceCandidate> _localIce =
      StreamController<WebRtcIceCandidate>.broadcast();
  final StreamController<WebRtcConnectionState> _states =
      StreamController<WebRtcConnectionState>.broadcast();
  final StreamController<String> _remoteTracks =
      StreamController<String>.broadcast();
  final List<WebRtcIceCandidate> addedCandidates = [];
  int addSendOnlyVideoTransceiverCalls = 0;
  int closeCalls = 0;
  String? localDescription;
  String? remoteDescription;

  @override
  Stream<WebRtcIceCandidate> get localIceCandidates => _localIce.stream;

  @override
  Stream<WebRtcConnectionState> get connectionStates => _states.stream;

  @override
  Stream<String> get remoteVideoTrackIds => _remoteTracks.stream;

  void emitLocalCandidate(WebRtcIceCandidate candidate) =>
      _localIce.add(candidate);

  void emitState(WebRtcConnectionState state) => _states.add(state);

  @override
  Future<void> addSendOnlyVideoTransceiver() async {
    addSendOnlyVideoTransceiverCalls++;
  }

  @override
  Future<String> createOffer() async => 'v=0\r\n';

  @override
  Future<String> createAnswer() async => 'v=0\r\n';

  @override
  Future<void> setLocalDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  }) async {
    localDescription = sdp;
  }

  @override
  Future<void> setRemoteDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  }) async {
    remoteDescription = sdp;
  }

  @override
  Future<void> addIceCandidate(WebRtcIceCandidate candidate) async {
    addedCandidates.add(candidate);
  }

  @override
  Future<void> close() async {
    closeCalls++;
    await _localIce.close();
    await _states.close();
    await _remoteTracks.close();
  }
}
