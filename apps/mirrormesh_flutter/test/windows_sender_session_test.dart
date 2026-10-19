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
              'startSenderSession' => true,
              'stopSenderSession' => true,
              'sendSenderSessionIce' => true,
              'pollSenderPairing' => <String, Object?>{
                'state': WindowsSenderPairingState.approved.nativeValue,
                'errorCode': 0,
                'receiverFingerprint': 'a' * 64,
              },
              'pollSenderSessionEvent' =>
                events.isEmpty
                    ? jsonEncode(<String, Object?>{'pending': true})
                    : events.removeAt(0),
              'cancelSenderPairing' => true,
              _ => null,
            };
          });

      final peer = _FakePeer();
      const sentStats = WebRtcOutboundVideoStats(
        framesEncoded: 3,
        bytesSent: 2048,
        codecMimeType: 'video/H264',
      );
      peer.outboundVideoStats = sentStats;
      final controller = WindowsSenderSessionController(
        pairing: const WindowsPairingBootstrap(),
        factory: _FakeFactory(peer),
        statsPollInterval: const Duration(milliseconds: 10),
      );
      final statsFuture = controller.videoStats.first;
      final startedSessionId = await controller.start(
        requestId: 41,
        sourceId: 'window-2',
      );
      expect(await statsFuture.timeout(const Duration(seconds: 1)), sentStats);
      peer.emitLocalCandidate(
        const WebRtcIceCandidate(
          candidate: 'candidate:1 1 UDP 1 192.0.2.1 5000 typ host',
          sdpMid: '0',
          sdpMLineIndex: 0,
        ),
      );
      expect(
        calls.where((call) => call.method == 'startSenderSession'),
        isEmpty,
      );
      peer.emitState(WebRtcConnectionState.connected);
      peer.emitState(WebRtcConnectionState.connected);
      await Future<void>.delayed(const Duration(milliseconds: 240));

      expect(startedSessionId, sessionId);
      expect(calls.first.method, 'pollSenderPairing');
      expect(peer.addSendOnlyVideoTransceiverCalls, 0);
      expect(peer.startedCaptureSourceId, 'window-2');
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
      final startCalls = calls
          .where((call) => call.method == 'startSenderSession')
          .toList();
      expect(startCalls, hasLength(1));
      expect(startCalls.single.arguments, <String, Object?>{
        'requestId': 41,
        'sourceId': 'desktop-capture',
      });
      expect(controller.state, WindowsSenderSessionState.streaming);

      await Future.wait(<Future<void>>[
        controller.stop(),
        controller.stop(),
      ]);
      await controller.dispose();
      final statsPollsAfterClose = peer.statsPollCalls;
      await Future<void>.delayed(const Duration(milliseconds: 30));
      expect(peer.statsPollCalls, statsPollsAfterClose);
      expect(peer.stopDesktopCaptureCalls, 1);
      expect(peer.closeCalls, 1);
      final stopCallIndex = calls.indexWhere(
        (call) => call.method == 'stopSenderSession',
      );
      final cancelCallIndex = calls.indexWhere(
        (call) => call.method == 'cancelSenderPairing',
      );
      expect(stopCallIndex, greaterThanOrEqualTo(0));
      expect(cancelCallIndex, greaterThan(stopCallIndex));
      expect(calls[stopCallIndex].arguments, <String, Object?>{
        'requestId': 41,
        'reason': 'user-disconnected',
      });
      expect(
        calls.where((call) => call.method == 'cancelSenderPairing'),
        hasLength(1),
      );
    },
  );

  test('does not request capture before TV pairing approval', () async {
    const channel = MethodChannel(WindowsPairingBootstrap.channelName);
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          return switch (call.method) {
            'pollSenderPairing' => <String, Object?>{
              'state': WindowsSenderPairingState.awaitingApproval.nativeValue,
              'errorCode': 0,
              'receiverFingerprint': '',
            },
            'cancelSenderPairing' => true,
            _ => null,
          };
        });
    final peer = _FakePeer();
    final controller = WindowsSenderSessionController(
      pairing: const WindowsPairingBootstrap(),
      factory: _FakeFactory(peer),
    );

    await expectLater(
      controller.start(requestId: 42, sourceId: 'window-2'),
      throwsStateError,
    );

    expect(peer.startedCaptureSourceId, isNull);
    await controller.dispose();
  });

  test('sends session start only after an encoded video frame exists', () async {
    final calls = <MethodCall>[];
    final startRequested = Completer<void>();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          if (call.method == 'startSenderSession' &&
              !startRequested.isCompleted) {
            startRequested.complete();
          }
          return switch (call.method) {
            'beginSenderSession' => sessionId,
            'startSenderSession' => true,
            'stopSenderSession' => true,
            'sendSenderSessionIce' => true,
            'pollSenderPairing' => <String, Object?>{
              'state': WindowsSenderPairingState.approved.nativeValue,
              'errorCode': 0,
              'receiverFingerprint': 'a' * 64,
            },
            'pollSenderSessionEvent' =>
              jsonEncode(<String, Object?>{'pending': true}),
            'cancelSenderPairing' => true,
            _ => null,
          };
        });

    final peer = _FakePeer();
    peer.outboundVideoStats = const WebRtcOutboundVideoStats(
      framesEncoded: 0,
      bytesSent: 0,
      codecMimeType: 'video/H264',
    );
    final controller = WindowsSenderSessionController(
      pairing: const WindowsPairingBootstrap(),
      factory: _FakeFactory(peer),
      statsPollInterval: const Duration(milliseconds: 10),
      firstEncodedFrameTimeout: const Duration(seconds: 1),
    );

    await controller.start(requestId: 43, sourceId: 'window-2');
    final transportConnected = controller.states.firstWhere(
      (state) => state == WindowsSenderSessionState.transportConnected,
    );
    peer.emitState(WebRtcConnectionState.connected);
    await transportConnected.timeout(const Duration(seconds: 1));

    expect(controller.state, WindowsSenderSessionState.transportConnected);
    expect(
      calls.where((call) => call.method == 'startSenderSession'),
      isEmpty,
    );

    peer.outboundVideoStats = const WebRtcOutboundVideoStats(
      framesEncoded: 1,
      bytesSent: 512,
      codecMimeType: 'video/H264',
    );
    await controller.states
        .firstWhere((state) => state == WindowsSenderSessionState.streaming)
        .timeout(const Duration(seconds: 1));
    await startRequested.future.timeout(const Duration(seconds: 1));

    expect(controller.state, WindowsSenderSessionState.streaming);
    expect(
      calls.where((call) => call.method == 'startSenderSession'),
      hasLength(1),
    );
    await controller.stop();
    await controller.dispose();
  });

  test(
    'fails and releases the session when no frame is encoded in time',
    () async {
      final calls = <MethodCall>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'beginSenderSession' => sessionId,
              'sendSenderSessionIce' => true,
              'pollSenderPairing' => <String, Object?>{
                'state': WindowsSenderPairingState.approved.nativeValue,
                'errorCode': 0,
                'receiverFingerprint': 'a' * 64,
              },
              'pollSenderSessionEvent' =>
                jsonEncode(<String, Object?>{'pending': true}),
              'cancelSenderPairing' => true,
              _ => null,
            };
          });

      final peer = _FakePeer();
      peer.outboundVideoStats = const WebRtcOutboundVideoStats(
        framesEncoded: null,
        bytesSent: null,
        codecMimeType: 'video/H264',
      );
      final controller = WindowsSenderSessionController(
        pairing: const WindowsPairingBootstrap(),
        factory: _FakeFactory(peer),
        statsPollInterval: const Duration(milliseconds: 5),
        firstEncodedFrameTimeout: const Duration(milliseconds: 25),
      );

      await controller.start(requestId: 44, sourceId: 'window-2');
      final failed = controller.states.firstWhere(
        (state) => state == WindowsSenderSessionState.failed,
      );
      peer.emitState(WebRtcConnectionState.connected);
      await failed.timeout(const Duration(seconds: 1));

      expect(controller.state, WindowsSenderSessionState.failed);
      expect(controller.lastError, isA<StateError>());
      expect(peer.stopDesktopCaptureCalls, 1);
      expect(peer.closeCalls, 1);
      expect(
        calls.where((call) => call.method == 'startSenderSession'),
        isEmpty,
      );
      expect(
        calls.where((call) => call.method == 'cancelSenderPairing'),
        hasLength(1),
      );
      await controller.dispose();
    },
  );
}

final class _FakeFactory implements WebRtcPeerConnectionFactory {
  const _FakeFactory(this.peer);

  final _FakePeer peer;

  @override
  Future<WebRtcPeerConnectionPort> create() async => peer;

  @override
  Future<List<WebRtcDesktopCaptureSource>> listDesktopCaptureSources() async =>
      const [
        WebRtcDesktopCaptureSource(
          id: 'window-2',
          name: 'MirrorMesh test pattern',
          type: WebRtcDesktopCaptureSourceType.window,
        ),
      ];
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
  int stopDesktopCaptureCalls = 0;
  String? startedCaptureSourceId;
  String? localDescription;
  String? remoteDescription;
  WebRtcOutboundVideoStats? outboundVideoStats;
  int statsPollCalls = 0;

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
  Future<void> startDesktopCapture(String sourceId) async {
    startedCaptureSourceId = sourceId;
  }

  @override
  Future<void> stopDesktopCapture() async {
    stopDesktopCaptureCalls++;
  }

  @override
  Future<WebRtcOutboundVideoStats?> getOutboundVideoStats() async =>
      _readOutboundVideoStats();

  Future<WebRtcOutboundVideoStats?> _readOutboundVideoStats() async {
    statsPollCalls++;
    return outboundVideoStats;
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
