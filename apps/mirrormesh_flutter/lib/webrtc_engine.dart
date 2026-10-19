import 'dart:async';
import 'dart:convert';

import 'package:flutter_webrtc/flutter_webrtc.dart';

const maxSdpBytes = 128 * 1024;
const maxIceCandidateBytes = 4 * 1024;

enum WebRtcDescriptionType { offer, answer }

enum WebRtcConnectionState {
  newConnection,
  connecting,
  connected,
  disconnected,
  failed,
  closed,
}

final class WebRtcIceCandidate {
  const WebRtcIceCandidate({
    required this.candidate,
    this.sdpMid,
    this.sdpMLineIndex,
  });

  final String candidate;
  final String? sdpMid;
  final int? sdpMLineIndex;

  @override
  bool operator ==(Object other) =>
      other is WebRtcIceCandidate &&
      other.candidate == candidate &&
      other.sdpMid == sdpMid &&
      other.sdpMLineIndex == sdpMLineIndex;

  @override
  int get hashCode => Object.hash(candidate, sdpMid, sdpMLineIndex);
}

sealed class WebRtcEngineEvent {
  const WebRtcEngineEvent();
}

final class WebRtcLocalIceCandidateEvent extends WebRtcEngineEvent {
  const WebRtcLocalIceCandidateEvent(this.candidate);

  final WebRtcIceCandidate candidate;

  @override
  bool operator ==(Object other) =>
      other is WebRtcLocalIceCandidateEvent && other.candidate == candidate;

  @override
  int get hashCode => candidate.hashCode;
}

final class WebRtcConnectionStateEvent extends WebRtcEngineEvent {
  const WebRtcConnectionStateEvent(this.state);

  final WebRtcConnectionState state;

  @override
  bool operator ==(Object other) =>
      other is WebRtcConnectionStateEvent && other.state == state;

  @override
  int get hashCode => state.hashCode;
}

final class WebRtcRemoteVideoTrackEvent extends WebRtcEngineEvent {
  const WebRtcRemoteVideoTrackEvent(this.trackId);

  final String trackId;

  @override
  bool operator ==(Object other) =>
      other is WebRtcRemoteVideoTrackEvent && other.trackId == trackId;

  @override
  int get hashCode => trackId.hashCode;
}

abstract interface class WebRtcPeerConnectionFactory {
  Future<WebRtcPeerConnectionPort> create();
  Future<List<WebRtcDesktopCaptureSource>> listDesktopCaptureSources();
}

enum WebRtcDesktopCaptureSourceType { screen, window }

/// Keeps every locally supported H.264 profile while excluding other codecs.
List<RTCRtpCodecCapability> h264VideoCodecPreferences(
  Iterable<RTCRtpCodecCapability> codecs,
) => [
  for (final codec in codecs)
    if (codec.mimeType.toLowerCase() == 'video/h264' &&
        codec.clockRate == 90000)
      codec,
];

final class WebRtcDesktopCaptureSource {
  const WebRtcDesktopCaptureSource({
    required this.id,
    required this.name,
    required this.type,
  });

  final String id;
  final String name;
  final WebRtcDesktopCaptureSourceType type;

  @override
  bool operator ==(Object other) =>
      other is WebRtcDesktopCaptureSource &&
      other.id == id &&
      other.name == name &&
      other.type == type;

  @override
  int get hashCode => Object.hash(id, name, type);
}

abstract interface class WebRtcPeerConnectionPort {
  Stream<WebRtcIceCandidate> get localIceCandidates;
  Stream<WebRtcConnectionState> get connectionStates;
  Stream<String> get remoteVideoTrackIds;

  Future<void> addSendOnlyVideoTransceiver();
  Future<void> startDesktopCapture(String sourceId);
  Future<void> stopDesktopCapture();
  Future<String> createOffer();
  Future<String> createAnswer();
  Future<void> setLocalDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  });
  Future<void> setRemoteDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  });
  Future<void> addIceCandidate(WebRtcIceCandidate candidate);
  Future<void> close();
}

/// Thin control and native track adapter around the Flutter WebRTC plugin.
///
/// Captured pixels and encoded frames stay in the native WebRTC pipeline. This
/// API exposes only source metadata and track lifecycle controls.
final class FlutterWebRtcEngine {
  FlutterWebRtcEngine({WebRtcPeerConnectionFactory? factory})
    : _factory = factory ?? const FlutterWebRtcPeerConnectionFactory();

  final WebRtcPeerConnectionFactory _factory;
  final StreamController<WebRtcEngineEvent> _events =
      StreamController<WebRtcEngineEvent>.broadcast();
  Future<void>? _initialization;
  Future<void>? _closing;
  WebRtcPeerConnectionPort? _peer;
  StreamSubscription<WebRtcIceCandidate>? _iceSubscription;
  StreamSubscription<WebRtcConnectionState>? _stateSubscription;
  StreamSubscription<String>? _trackSubscription;
  bool _closed = false;

  Stream<WebRtcEngineEvent> get events => _events.stream;

  Future<List<WebRtcDesktopCaptureSource>> listDesktopCaptureSources() {
    if (_closed) {
      return Future<List<WebRtcDesktopCaptureSource>>.error(
        StateError('WebRTC engine is closed.'),
      );
    }
    return _factory.listDesktopCaptureSources();
  }

  Future<void> initialize() {
    if (_closed) {
      return Future<void>.error(StateError('WebRTC engine is closed.'));
    }
    if (_peer != null) return Future<void>.value();
    return _initialization ??= _initializePeer();
  }

  Future<void> _initializePeer() async {
    final peer = await _factory.create();
    if (_closed) {
      await peer.close();
      throw StateError('WebRTC engine closed during initialization.');
    }
    _peer = peer;
    _iceSubscription = peer.localIceCandidates.listen((candidate) {
      _emit(WebRtcLocalIceCandidateEvent(candidate));
    }, onError: _forwardError);
    _stateSubscription = peer.connectionStates.listen((state) {
      _emit(WebRtcConnectionStateEvent(state));
    }, onError: _forwardError);
    _trackSubscription = peer.remoteVideoTrackIds.listen((trackId) {
      if (trackId.isNotEmpty) _emit(WebRtcRemoteVideoTrackEvent(trackId));
    }, onError: _forwardError);
  }

  Future<String> createOffer() async => _peerOrThrow().createOffer();

  Future<void> addSendOnlyVideoTransceiver() async =>
      _peerOrThrow().addSendOnlyVideoTransceiver();

  Future<void> startDesktopCapture({required String sourceId}) async {
    if (sourceId.isEmpty || sourceId.length > 256) {
      throw ArgumentError.value(
        sourceId,
        'sourceId',
        'Invalid desktop source.',
      );
    }
    await _peerOrThrow().startDesktopCapture(sourceId);
  }

  Future<void> stopDesktopCapture() async {
    await _peerOrThrow().stopDesktopCapture();
  }

  Future<String> createAnswer() async => _peerOrThrow().createAnswer();

  Future<void> setLocalDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  }) async {
    _validateSdp(sdp);
    await _peerOrThrow().setLocalDescription(type: type, sdp: sdp);
  }

  Future<void> setRemoteDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  }) async {
    _validateSdp(sdp);
    await _peerOrThrow().setRemoteDescription(type: type, sdp: sdp);
  }

  Future<void> addRemoteIceCandidate(WebRtcIceCandidate candidate) async {
    _validateCandidate(candidate.candidate);
    await _peerOrThrow().addIceCandidate(candidate);
  }

  Future<void> close() => _closing ??= _closePeer();

  Future<void> _closePeer() async {
    if (_closed) return;
    _closed = true;
    try {
      await _initialization;
    } on Object {
      // A failed or concurrently closed initialization still needs cleanup.
    }
    await _iceSubscription?.cancel();
    await _stateSubscription?.cancel();
    await _trackSubscription?.cancel();
    final peer = _peer;
    _peer = null;
    try {
      if (peer != null) {
        try {
          await peer.stopDesktopCapture();
        } finally {
          await peer.close();
        }
      }
    } finally {
      await _events.close();
    }
  }

  WebRtcPeerConnectionPort _peerOrThrow() {
    final peer = _peer;
    if (_closed || peer == null) {
      throw StateError('WebRTC engine has not been initialized.');
    }
    return peer;
  }

  void _forwardError(Object error, StackTrace stackTrace) {
    if (!_closed && !_events.isClosed) _events.addError(error, stackTrace);
  }

  void _emit(WebRtcEngineEvent event) {
    if (!_closed && !_events.isClosed) _events.add(event);
  }
}

final class FlutterWebRtcPeerConnectionFactory
    implements WebRtcPeerConnectionFactory {
  const FlutterWebRtcPeerConnectionFactory({
    this.configuration = const <String, dynamic>{},
  });

  final Map<String, dynamic> configuration;

  @override
  Future<WebRtcPeerConnectionPort> create() async {
    final peer = await createPeerConnection(configuration);
    return _FlutterWebRtcPeerConnection(peer);
  }

  @override
  Future<List<WebRtcDesktopCaptureSource>> listDesktopCaptureSources() async {
    final sources = await desktopCapturer.getSources(
      types: const [SourceType.Screen, SourceType.Window],
    );
    return [
      for (final source in sources.take(64))
        if (source.id.isNotEmpty &&
            source.id.length <= 256 &&
            source.name.isNotEmpty &&
            source.name.length <= 256 &&
            !source.name.contains(RegExp(r'[\u0000-\u001f\u007f]')))
          WebRtcDesktopCaptureSource(
            id: source.id,
            name: source.name,
            type: switch (source.type) {
              SourceType.Screen => WebRtcDesktopCaptureSourceType.screen,
              SourceType.Window => WebRtcDesktopCaptureSourceType.window,
            },
          ),
    ];
  }
}

final class _FlutterWebRtcPeerConnection implements WebRtcPeerConnectionPort {
  _FlutterWebRtcPeerConnection(this._peer) {
    _peer.onIceCandidate = _onIceCandidate;
    _peer.onConnectionState = _onConnectionState;
    _peer.onTrack = _onTrack;
  }

  final RTCPeerConnection _peer;
  final StreamController<WebRtcIceCandidate> _iceCandidates =
      StreamController<WebRtcIceCandidate>.broadcast();
  final StreamController<WebRtcConnectionState> _connectionStates =
      StreamController<WebRtcConnectionState>.broadcast();
  final StreamController<String> _remoteVideoTrackIds =
      StreamController<String>.broadcast();
  MediaStream? _localDesktopStream;
  bool _closed = false;

  @override
  Stream<WebRtcIceCandidate> get localIceCandidates => _iceCandidates.stream;

  @override
  Stream<WebRtcConnectionState> get connectionStates =>
      _connectionStates.stream;

  @override
  Stream<String> get remoteVideoTrackIds => _remoteVideoTrackIds.stream;

  @override
  Future<void> addSendOnlyVideoTransceiver() async {
    await _peer.addTransceiver(
      kind: RTCRtpMediaType.RTCRtpMediaTypeVideo,
      init: RTCRtpTransceiverInit(direction: TransceiverDirection.SendOnly),
    );
  }

  @override
  Future<void> startDesktopCapture(String sourceId) async {
    if (_closed || _localDesktopStream != null) {
      throw StateError(
        'Desktop capture is already active or the peer is closed.',
      );
    }
    final codecCapabilities = await getRtpSenderCapabilities('video');
    final h264Codecs = h264VideoCodecPreferences(
      codecCapabilities.codecs ?? const <RTCRtpCodecCapability>[],
    );
    if (h264Codecs.isEmpty) {
      throw UnsupportedError(
        'This Windows WebRTC build does not advertise H.264 video encoding.',
      );
    }

    final stream = await navigator.mediaDevices.getDisplayMedia(
      <String, dynamic>{
        'audio': false,
        'video': <String, dynamic>{
          'deviceId': <String, String>{'exact': sourceId},
          'mandatory': <String, int>{'frameRate': 30},
          'cursor': 'always',
        },
      },
    );
    try {
      final videoTracks = stream.getVideoTracks();
      if (videoTracks.length != 1 || stream.getAudioTracks().isNotEmpty) {
        throw StateError(
          'The selected source must provide one silent video track.',
        );
      }
      final sender = await _peer.addTrack(videoTracks.single, stream);
      RTCRtpTransceiver? senderTransceiver;
      for (final transceiver in await _peer.getTransceivers()) {
        if (transceiver.sender.senderId == sender.senderId) {
          senderTransceiver = transceiver;
          break;
        }
      }
      if (senderTransceiver == null) {
        throw StateError('The native WebRTC video sender has no transceiver.');
      }
      await senderTransceiver.setCodecPreferences(h264Codecs);
      _localDesktopStream = stream;
    } catch (_) {
      await _stopStream(stream);
      rethrow;
    }
  }

  @override
  Future<void> stopDesktopCapture() async {
    final stream = _localDesktopStream;
    _localDesktopStream = null;
    if (stream != null) await _stopStream(stream);
  }

  @override
  Future<String> createOffer() async =>
      _readDescription(await _peer.createOffer());

  @override
  Future<String> createAnswer() async =>
      _readDescription(await _peer.createAnswer());

  @override
  Future<void> setLocalDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  }) => _peer.setLocalDescription(RTCSessionDescription(sdp, _sdpType(type)));

  @override
  Future<void> setRemoteDescription({
    required WebRtcDescriptionType type,
    required String sdp,
  }) => _peer.setRemoteDescription(RTCSessionDescription(sdp, _sdpType(type)));

  @override
  Future<void> addIceCandidate(WebRtcIceCandidate candidate) =>
      _peer.addCandidate(
        RTCIceCandidate(
          candidate.candidate,
          candidate.sdpMid,
          candidate.sdpMLineIndex,
        ),
      );

  @override
  Future<void> close() async {
    if (_closed) return;
    _closed = true;
    try {
      await stopDesktopCapture();
    } finally {
      try {
        await _peer.close();
      } finally {
        try {
          await _peer.dispose();
        } finally {
          await _iceCandidates.close();
          await _connectionStates.close();
          await _remoteVideoTrackIds.close();
        }
      }
    }
  }

  Future<void> _stopStream(MediaStream stream) async {
    try {
      for (final track in stream.getTracks()) {
        await track.stop();
      }
    } finally {
      await stream.dispose();
    }
  }

  void _onIceCandidate(RTCIceCandidate candidate) {
    final value = candidate.candidate;
    if (value == null || value.isEmpty) return;
    try {
      _validateCandidate(value);
      _iceCandidates.add(
        WebRtcIceCandidate(
          candidate: value,
          sdpMid: candidate.sdpMid,
          sdpMLineIndex: candidate.sdpMLineIndex,
        ),
      );
    } on FormatException catch (error, stackTrace) {
      _iceCandidates.addError(error, stackTrace);
    }
  }

  void _onConnectionState(RTCPeerConnectionState state) {
    _connectionStates.add(switch (state) {
      RTCPeerConnectionState.RTCPeerConnectionStateNew =>
        WebRtcConnectionState.newConnection,
      RTCPeerConnectionState.RTCPeerConnectionStateConnecting =>
        WebRtcConnectionState.connecting,
      RTCPeerConnectionState.RTCPeerConnectionStateConnected =>
        WebRtcConnectionState.connected,
      RTCPeerConnectionState.RTCPeerConnectionStateDisconnected =>
        WebRtcConnectionState.disconnected,
      RTCPeerConnectionState.RTCPeerConnectionStateFailed =>
        WebRtcConnectionState.failed,
      RTCPeerConnectionState.RTCPeerConnectionStateClosed =>
        WebRtcConnectionState.closed,
    });
  }

  void _onTrack(RTCTrackEvent event) {
    final trackId = event.track.id;
    if (event.track.kind == 'video' && trackId != null && trackId.isNotEmpty) {
      _remoteVideoTrackIds.add(trackId);
    }
  }
}

String _readDescription(RTCSessionDescription description) {
  final sdp = description.sdp;
  if (sdp == null) throw const FormatException('WebRTC returned no SDP.');
  _validateSdp(sdp);
  return sdp;
}

String _sdpType(WebRtcDescriptionType type) => switch (type) {
  WebRtcDescriptionType.offer => 'offer',
  WebRtcDescriptionType.answer => 'answer',
};

void _validateSdp(String sdp) {
  if (utf8.encode(sdp).length > maxSdpBytes) {
    throw const FormatException('SDP exceeds the 128 KiB limit.');
  }
  if (sdp.codeUnits.any(
    (unit) => (unit < 0x20 && unit != 0x0a && unit != 0x0d) || unit == 0x7f,
  )) {
    throw const FormatException('SDP contains a disallowed control character.');
  }
}

void _validateCandidate(String candidate) {
  if (candidate.isEmpty ||
      utf8.encode(candidate).length > maxIceCandidateBytes) {
    throw const FormatException('ICE candidate is empty or exceeds 4 KiB.');
  }
  if (candidate.codeUnits.any((unit) => unit < 0x20 || unit == 0x7f)) {
    throw const FormatException('ICE candidate contains a control character.');
  }
}
