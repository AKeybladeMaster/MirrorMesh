import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';

import 'native_connection.dart';
import 'device_context.dart';
import 'connection_popups.dart';
import 'ios_pairing_bootstrap.dart';
import 'receive_screen_page.dart';
import 'webrtc_engine.dart';

class MobileConnectionHome extends StatelessWidget {
  const MobileConnectionHome({super.key});
  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('MirrorMesh')),
    body: ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(
          'Choose a direction',
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        const SizedBox(height: 24),
        FilledButton.icon(
          onPressed: () => Navigator.push<void>(
            context,
            MaterialPageRoute(builder: (_) => const MobileSharePage()),
          ),
          icon: const Icon(Icons.screen_share),
          label: const Text('Share this device'),
        ),
        const SizedBox(height: 16),
        OutlinedButton.icon(
          onPressed: () => Navigator.push<void>(
            context,
            MaterialPageRoute(builder: (_) => const ReceiveScreenPage()),
          ),
          icon: const Icon(Icons.qr_code_2),
          label: const Text('Receive another screen / show QR'),
        ),
        const SizedBox(height: 16),
        const Text(
          'Connect devices running MirrorMesh on the same network. Each connection requires identity verification and receiver approval.',
        ),
      ],
    ),
  );
}

class MobileSharePage extends StatefulWidget {
  const MobileSharePage({super.key});
  @override
  State<MobileSharePage> createState() => _MobileSharePageState();
}

class _MobileSharePageState extends State<MobileSharePage>
    with WidgetsBindingObserver {
  final _invitation = TextEditingController();
  String _status =
      'Scan the receiver QR with the Camera app, or paste its invitation here.';
  String? _error;
  int? _id;
  bool _approved = false,
      _busy = false,
      _offered = false,
      _answered = false,
      _started = false;
  int _generation = 0;
  RTCPeerConnection? _peer;
  MediaStream? _capture;
  final _localIce = <RTCIceCandidate>[];
  final _remoteIce = <RTCIceCandidate>[];
  Timer? _deadline;
  bool _broadcastAvailable = false, _wholeDevice = false;
  bool _broadcastChecked = false;
  ConnectionPopup<void>? _pairingPopup;
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    unawaited(_readInvitation());
  }

  Future<void> _readInvitation() async {
    if (_id != null) return;
    try {
      final value = await const MethodChannel('com.mirrormesh/ios_pairing')
          .invokeMethod<String>('takeInvitation');
      if (mounted && value != null) _invitation.text = value;
      final available =
          await const MethodChannel('com.mirrormesh/ios_pairing')
              .invokeMethod<bool>('hasBroadcastExtension') ??
          false;
      if (mounted) {
        setState(() {
          _broadcastAvailable = available;
          if (!_broadcastChecked) _wholeDevice = available;
          _broadcastChecked = true;
        });
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = 'Could not read the invitation: $error');
      }
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) unawaited(_readInvitation());
  }

  Future<void> _pair() async {
    if (_busy || _id != null) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    final generation = ++_generation;
    _pairingPopup = ConnectionPopup<void>(
      context,
      (context) => PopScope(
        canPop: false,
        child: AlertDialog.adaptive(
          title: const Text('Connecting to receiver'),
          content: const Text(
            'Approve this device on the receiver. Screen capture still requires your permission.',
          ),
          actions: [popupAction(context, 'Cancel', _stop)],
        ),
      ),
    );
    try {
      await const IosPairingBootstrap().prepare();
      await configureDeviceProfile();
      final result = await nativeConnection({
        'op': 'send_pair',
        'invitation': _invitation.text.trim(),
      });
      final id = result['id'] as int;
      if (!mounted || generation != _generation) {
        await nativeConnection({'op': 'send_cancel', 'id': id});
        return;
      }
      _id = id;
      while (mounted && generation == _generation) {
        final status = await nativeConnection({'op': 'send_poll', 'id': id});
        if (!mounted || generation != _generation) return;
        if (status['state'] == 3) {
          _pairingPopup?.close();
          _pairingPopup = null;
          setState(() {
            _approved = true;
            _status = 'Approved. Start sharing only when you are ready.';
          });
          unawaited(
            connectionNotice(
              context,
              'Connection approved',
              'Choose a screen using the system capture dialog when you are ready.',
              brief: true,
            ),
          );
          break;
        }
        if ((status['state'] as int) >= 4) {
          throw StateError(
            'Pairing ended (MM-PAIR-${status['error']}). Create a fresh receiver invitation.',
          );
        }
        setState(() => _status = 'Waiting for receiver approval…');
        await Future<void>.delayed(const Duration(milliseconds: 300));
      }
    } catch (error) {
      if (mounted) {
        setState(() => _error = error.toString());
        await _stop();
        if (mounted) {
          await connectionNotice(
            context,
            'Connection failed',
            'Refresh the receiver invitation and try again.',
          );
        }
      }
    } finally {
      _pairingPopup?.close();
      _pairingPopup = null;
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _share() async {
    if (!_approved || _busy || _peer != null) return;
    setState(() => _busy = true);
    final generation = _generation;
    try {
      final capabilities = await getRtpSenderCapabilities('video');
      final codecs = h264VideoCodecPreferences(capabilities.codecs ?? []);
      if (codecs.isEmpty) {
        throw StateError('H.264 screen encoding is unavailable.');
      }
      final peer = await createPeerConnection({
        'iceServers': <Object>[],
        'sdpSemantics': 'unified-plan',
      });
      _peer = peer;
      // Only the system broadcast picker can approve full-device ReplayKit capture.
      final stream = await navigator.mediaDevices.getDisplayMedia({
        'video': _wholeDevice
            ? <String, dynamic>{'deviceId': 'broadcast'}
            : true,
        'audio': false,
      });
      _capture = stream;
      if (!mounted || generation != _generation) {
        await _stop();
        return;
      }
      final tracks = stream.getVideoTracks();
      if (tracks.length != 1) {
        throw StateError('A screen capture track is required.');
      }
      tracks.single.onEnded = () => unawaited(_stop());
      await peer.addTransceiver(
        track: tracks.single,
        init: RTCRtpTransceiverInit(
          direction: TransceiverDirection.SendOnly,
          streams: [stream],
        ),
      );
      for (final transceiver in await peer.getTransceivers()) {
        await transceiver.setCodecPreferences(codecs);
      }
      peer.onIceCandidate = (candidate) {
        if (generation != _generation ||
            candidate.candidate?.isNotEmpty != true) {
          return;
        }
        if (!_offered) {
          if (_localIce.length < 64) _localIce.add(candidate);
        } else {
          unawaited(_sendIce(candidate));
        }
      };
      peer.onConnectionState = (state) {
        if (generation != _generation) return;
        if (state == RTCPeerConnectionState.RTCPeerConnectionStateConnected &&
            !_started) {
          _started = true;
          _deadline?.cancel();
          unawaited(
            nativeConnection({
                  'op': 'send_start',
                  'id': _id,
                  'source': _wholeDevice ? 'ios-screen' : 'ios-app-screen',
                })
                .then((_) {
                  if (mounted) {
                    setState(
                      () => _status = _wholeDevice
                          ? 'Sharing the device screen'
                          : 'Sharing this app’s screen',
                    );
                  }
                })
                .catchError((Object error) {
                  if (mounted) setState(() => _error = error.toString());
                  unawaited(_stop());
                }),
          );
        }
        if (state == RTCPeerConnectionState.RTCPeerConnectionStateFailed) {
          unawaited(_stop());
        }
      };
      final offer = await peer.createOffer();
      await peer.setLocalDescription(offer);
      await nativeConnection({'op': 'send_offer', 'id': _id, 'sdp': offer.sdp});
      _offered = true;
      for (final candidate in _localIce) {
        await _sendIce(candidate);
      }
      _localIce.clear();
      _deadline = Timer(const Duration(seconds: 30), () {
        if (mounted) setState(() => _error = 'Video connection timed out.');
        unawaited(_stop());
      });
      unawaited(_pollEvents(generation));
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
      await _stop();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _sendIce(RTCIceCandidate ice) => nativeConnection({
    'op': 'send_ice',
    'id': _id,
    'candidate': ice.candidate,
    'mid': ice.sdpMid ?? '0',
    'line': ice.sdpMLineIndex ?? 0,
  }).then((_) {});
  Future<void> _pollEvents(int generation) async {
    try {
      while (mounted && generation == _generation && _id != null) {
        final event = await nativeConnection({'op': 'send_event', 'id': _id});
        if (generation != _generation) return;
        switch (event['kind']) {
          case 'answer':
            await _peer?.setRemoteDescription(
              RTCSessionDescription(event['sdp'] as String, 'answer'),
            );
            _answered = true;
            for (final ice in _remoteIce) {
              await _peer?.addCandidate(ice);
            }
            _remoteIce.clear();
          case 'ice':
            final ice = RTCIceCandidate(
              event['candidate'] as String,
              event['mid'] as String,
              event['line'] as int,
            );
            if (_answered) {
              await _peer?.addCandidate(ice);
            } else if (_remoteIce.length < 64) {
              _remoteIce.add(ice);
            }
          case 'stop' || 'revoke' || 'ended':
            await _stop();
            return;
        }
        await Future<void>.delayed(const Duration(milliseconds: 100));
      }
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
      await _stop();
    }
  }

  Future<void> _stop() async {
    ++_generation;
    _pairingPopup?.close();
    _pairingPopup = null;
    _deadline?.cancel();
    final id = _id;
    _id = null;
    final peer = _peer;
    _peer = null;
    final capture = _capture;
    _capture = null;
    for (final track in capture?.getTracks() ?? <MediaStreamTrack>[]) {
      await track.stop();
    }
    await capture?.dispose();
    await peer?.close();
    await peer?.dispose();
    if (id != null) {
      try {
        await nativeConnection({'op': 'send_cancel', 'id': id});
      } catch (_) {}
    }
    _approved = false;
    _offered = false;
    _answered = false;
    _started = false;
    _localIce.clear();
    _remoteIce.clear();
    if (mounted) {
      setState(
        () => _status = 'Sharing stopped. Pair again with a new invitation.',
      );
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('Share this device')),
    body: ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(_status),
        if (_error != null)
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        TextField(
          controller: _invitation,
          enabled: _id == null,
          decoration: const InputDecoration(labelText: 'Receiver invitation'),
          maxLines: 3,
          maxLength: 1024,
        ),
        OutlinedButton(
          onPressed: _id == null
              ? () async {
                  final data = await Clipboard.getData(Clipboard.kTextPlain);
                  if (mounted) _invitation.text = data?.text?.trim() ?? '';
                }
              : null,
          child: const Text('Paste invitation'),
        ),
        SwitchListTile(
          title: const Text('Share the whole device screen'),
          subtitle: Text(
            _broadcastAvailable
                ? 'iOS will ask you to start a screen broadcast.'
                : 'Requires the signed MirrorMesh Broadcast extension.',
          ),
          value: _wholeDevice,
          onChanged: _broadcastAvailable && _peer == null && !_busy
              ? (value) => setState(() => _wholeDevice = value)
              : null,
        ),
        FilledButton(
          onPressed: _busy || _id != null ? null : _pair,
          child: const Text('Pair with receiver'),
        ),
        FilledButton(
          onPressed: _approved && !_busy && _peer == null ? _share : null,
          child: Text(
            _wholeDevice ? 'Start screen broadcast' : 'Start sharing this app',
          ),
        ),
        OutlinedButton(
          onPressed: _id != null ? _stop : null,
          child: const Text('Stop / disconnect'),
        ),
        const Text(
          'Screen capture starts only after your explicit action and the iOS permission dialog. Use Stop or the system broadcast control to end it.',
        ),
      ],
    ),
  );
  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    unawaited(_stop());
    _invitation.dispose();
    super.dispose();
  }
}
