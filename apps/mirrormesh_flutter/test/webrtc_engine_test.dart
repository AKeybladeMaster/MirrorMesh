import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/webrtc_engine.dart';

void main() {
  test(
    'engine creates a peer only when initialized and forwards metadata',
    () async {
      final factory = _FakeFactory();
      final engine = FlutterWebRtcEngine(factory: factory);
      final events = <WebRtcEngineEvent>[];
      final subscription = engine.events.listen(events.add);

      expect(factory.createCalls, 0);
      await engine.initialize();
      expect(factory.createCalls, 1);
      expect(await engine.createOffer(), 'offer-sdp');
      await engine.setRemoteDescription(
        type: WebRtcDescriptionType.answer,
        sdp: 'answer-sdp',
      );
      await engine.addRemoteIceCandidate(
        const WebRtcIceCandidate(
          candidate: 'candidate:1 1 UDP 1 192.0.2.1 5000 typ host',
          sdpMid: '0',
          sdpMLineIndex: 0,
        ),
      );

      const candidate = WebRtcIceCandidate(
        candidate: 'candidate:2 1 UDP 1 192.0.2.2 5001 typ host',
        sdpMid: '0',
        sdpMLineIndex: 0,
      );
      factory.peer.emitLocalCandidate(candidate);
      factory.peer.emitState(WebRtcConnectionState.connected);
      factory.peer.emitRemoteTrack('video-track-1');
      await Future<void>.delayed(Duration.zero);

      expect(factory.peer.remoteDescription, 'answer-sdp');
      expect(factory.peer.addedCandidates, [
        const WebRtcIceCandidate(
          candidate: 'candidate:1 1 UDP 1 192.0.2.1 5000 typ host',
          sdpMid: '0',
          sdpMLineIndex: 0,
        ),
      ]);
      expect(events, hasLength(3));
      expect(events[0], isA<WebRtcLocalIceCandidateEvent>());
      expect((events[0] as WebRtcLocalIceCandidateEvent).candidate, candidate);
      expect(
        events[1],
        const WebRtcConnectionStateEvent(WebRtcConnectionState.connected),
      );
      expect(events[2], const WebRtcRemoteVideoTrackEvent('video-track-1'));

      await subscription.cancel();
      await engine.close();
    },
  );

  test(
    'operations before initialization fail without creating an engine',
    () async {
      final factory = _FakeFactory();
      final engine = FlutterWebRtcEngine(factory: factory);

      await expectLater(engine.createOffer(), throwsStateError);
      expect(factory.createCalls, 0);
      await engine.close();
      expect(factory.peer.closeCalls, 0);
    },
  );

  test(
    'configures a send-only video transceiver through the native adapter',
    () async {
      final factory = _FakeFactory();
      final engine = FlutterWebRtcEngine(factory: factory);

      await expectLater(engine.addSendOnlyVideoTransceiver(), throwsStateError);
      expect(factory.peer.addSendOnlyVideoTransceiverCalls, 0);
      await engine.initialize();
      await engine.addSendOnlyVideoTransceiver();

      expect(factory.peer.addSendOnlyVideoTransceiverCalls, 1);
      await engine.close();
    },
  );

  test('close tears down the peer once and ends the event stream', () async {
    final factory = _FakeFactory();
    final engine = FlutterWebRtcEngine(factory: factory);
    final done = expectLater(engine.events, emitsDone);
    await engine.initialize();

    await engine.close();
    await engine.close();
    await done;

    expect(factory.peer.closeCalls, 1);
    await expectLater(engine.createOffer(), throwsStateError);
  });

  test('close ends engine events even when native cleanup fails', () async {
    final factory = _FakeFactory();
    final engine = FlutterWebRtcEngine(factory: factory);
    final done = Completer<void>();
    engine.events.listen((_) {}, onDone: done.complete);
    await engine.initialize();
    factory.peer.closeFailure = StateError('native close failed');

    await expectLater(engine.close(), throwsStateError);
    expect(done.isCompleted, isTrue);
    expect(factory.peer.closeCalls, 1);
  });
  test(
    'rejects oversized or control-bearing descriptions before delegation',
    () async {
      final factory = _FakeFactory();
      final engine = FlutterWebRtcEngine(factory: factory);
      await engine.initialize();

      await expectLater(
        engine.setRemoteDescription(
          type: WebRtcDescriptionType.offer,
          sdp: 'x' * (maxSdpBytes + 1),
        ),
        throwsFormatException,
      );
      await expectLater(
        engine.setRemoteDescription(
          type: WebRtcDescriptionType.offer,
          sdp: 'v=0\u0000',
        ),
        throwsFormatException,
      );
      expect(factory.peer.remoteDescription, isNull);
      await engine.close();
    },
  );

  test(
    'rejects empty and oversized ICE candidates before delegation',
    () async {
      final factory = _FakeFactory();
      final engine = FlutterWebRtcEngine(factory: factory);
      await engine.initialize();

      await expectLater(
        engine.addRemoteIceCandidate(const WebRtcIceCandidate(candidate: '')),
        throwsFormatException,
      );
      await expectLater(
        engine.addRemoteIceCandidate(
          WebRtcIceCandidate(candidate: 'x' * (maxIceCandidateBytes + 1)),
        ),
        throwsFormatException,
      );
      expect(factory.peer.addedCandidates, isEmpty);
      await engine.close();
    },
  );
}

final class _FakeFactory implements WebRtcPeerConnectionFactory {
  final _FakePeer peer = _FakePeer();
  int createCalls = 0;

  @override
  Future<WebRtcPeerConnectionPort> create() async {
    createCalls++;
    return peer;
  }
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
  String? remoteDescription;
  Object? closeFailure;

  @override
  Stream<WebRtcIceCandidate> get localIceCandidates => _localIce.stream;

  @override
  Stream<WebRtcConnectionState> get connectionStates => _states.stream;

  @override
  Stream<String> get remoteVideoTrackIds => _remoteTracks.stream;

  void emitLocalCandidate(WebRtcIceCandidate candidate) =>
      _localIce.add(candidate);

  void emitState(WebRtcConnectionState state) => _states.add(state);

  void emitRemoteTrack(String id) => _remoteTracks.add(id);

  @override
  Future<void> addSendOnlyVideoTransceiver() async {
    addSendOnlyVideoTransceiverCalls++;
  }

  @override
  Future<String> createOffer() async => 'offer-sdp';

  @override
  Future<String> createAnswer() async => 'answer-sdp';

  @override
  Future<void> setLocalDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  }) async {}

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
    if (closeFailure != null) throw closeFailure!;
  }
}
