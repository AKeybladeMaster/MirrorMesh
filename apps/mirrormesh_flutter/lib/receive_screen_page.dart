import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';
import 'package:qr_flutter/qr_flutter.dart';

import 'ios_pairing_bootstrap.dart';
import 'native_connection.dart';
import 'webrtc_engine.dart';

/// The same Rust listener and approval flow used by the native Android receiver.
class ReceiveScreenPage extends StatefulWidget {
  const ReceiveScreenPage({super.key});
  @override
  State<ReceiveScreenPage> createState() => _ReceiveScreenPageState();
}

class _ReceiveScreenPageState extends State<ReceiveScreenPage> {
  final _renderer = RTCVideoRenderer();
  List<String> _addresses = [];
  String? _address, _invitation, _error;
  String _status = 'Start receiving to create a QR invitation.';
  RTCPeerConnection? _peer;
  MediaStream? _remote;
  int? _connection;
  bool _active = false, _busy = false, _renderReady = false;
  int _generation = 0;
  final List<RTCIceCandidate> _pendingIce = [];
  bool _remoteDescriptionSet = false;
  bool _started = false;
  bool _ownsReceiver = false, _disposing = false;
  Future<void>? _stopping;
  Future<Map<String, dynamic>>? _starting;

  @override
  void initState() {
    super.initState();
    unawaited(_initialize());
  }

  Future<void> _initialize() async {
    try {
      await _renderer.initialize();
      final interfaces = await NetworkInterface.list(
        type: InternetAddressType.IPv4,
      );
      final addresses = interfaces
          .expand((item) => item.addresses)
          .where((ip) => !ip.isLoopback && !ip.isLinkLocal)
          .map((ip) => ip.address)
          .toList();
      if (mounted) {
        setState(() {
          _addresses = addresses;
          _address = addresses.firstOrNull;
          _renderReady = true;
        });
      }
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    }
  }

  Future<void> _start() async {
    if (_busy ||
        _active ||
        _address == null ||
        _stopping != null ||
        _disposing) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    final generation = ++_generation;
    try {
      if (Platform.isIOS) await const IosPairingBootstrap().prepare();
      if (!mounted || generation != _generation || _disposing) return;
      _starting = nativeConnection({
        'op': 'receive_start',
        'address': '$_address:8765',
      });
      final result = await _starting!;
      _ownsReceiver = true;
      if (!mounted || generation != _generation) {
        return;
      }
      setState(() {
        _active = true;
        _invitation = result['invitation'] as String;
        _status = 'Scan or copy this invitation on the device whose screen you want to view.';
      });
      unawaited(_waitForRequest(generation));
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      _starting = null;
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _waitForRequest(int generation) async {
    try {
      final request = await nativeConnection({'op': 'receive_wait'});
      if (!mounted || generation != _generation) return;
      var permission = false;
      final approved = await showDialog<bool>(
        context: context,
        barrierDismissible: false,
        builder: (context) => StatefulBuilder(
          builder: (context, update) => AlertDialog(
            title: Text('Receive from ${request['display_name']}?'),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  SelectableText(
                    'Address: ${request['source_ip']}\nFingerprint: ${request['fingerprint']}',
                  ),
                  if (request['verification_code'] case final String code) ...[
                    const SizedBox(height: 16),
                    const Text(
                      'Connection code — compare every group on the sender:',
                    ),
                    SelectableText(
                      code,
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const Text(
                      'Reject unexpected requests or different codes.',
                    ),
                  ],
                  CheckboxListTile(
                    value: permission,
                    onChanged: (value) =>
                        update(() => permission = value ?? false),
                    title: Text(
                      request['verification_code'] is String
                          ? 'The codes match. Allow this device to send its screen'
                          : 'Allow this device to send its screen',
                    ),
                  ),
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: const Text('Reject'),
              ),
              FilledButton(
                onPressed:
                    permission && request['requested_grants']['view'] == true
                    ? () => Navigator.pop(context, true)
                    : null,
                child: const Text('Approve'),
              ),
            ],
          ),
        ),
      );
      if (!mounted || generation != _generation) return;
      final decision = await nativeConnection({
        'op': 'receive_decide',
        'id': request['request_id'],
        'approve': approved == true,
      });
      if (!mounted || generation != _generation || _disposing) {
        return;
      }
      if (approved != true) {
        await _stop();
        return;
      }
      _connection = decision['connection_id'] as int;
      if (mounted) {
        setState(() {
          _invitation = null;
          _status = 'Approved. Waiting for the sender to choose a screen.';
        });
      }
      await _poll(generation);
    } catch (error) {
      if (mounted && generation == _generation) {
        setState(() => _error = error.toString());
        await _stop();
      }
    }
  }

  Future<void> _poll(int generation) async {
    while (mounted && generation == _generation && _connection != null) {
      final event = await nativeConnection({
        'op': 'receive_poll',
        'id': _connection,
      });
      if (!mounted || generation != _generation) return;
      if (event['pending'] == true) continue;
      switch (event['kind']) {
        case 'offer':
          await _answer(event['sdp'] as String, generation);
        case 'ice':
          final ice = RTCIceCandidate(
            event['candidate'] as String,
            event['media_id'] as String,
            event['media_line_index'] as int,
          );
          if (_remoteDescriptionSet) {
            await _peer?.addCandidate(ice);
          } else {
            _pendingIce.add(ice);
          }
        case 'start':
          _started = true;
          _renderer.srcObject = _remote;
          if (mounted) setState(() => _status = 'Receiving the shared screen');
        case 'stop' || 'revoked' || 'revoke':
          await _stop();
      }
    }
  }

  Future<void> _answer(String sdp, int generation) async {
    if (_peer != null) throw StateError('A video session is already active.');
    final peer = await createPeerConnection({
      'iceServers': <Object>[],
      'sdpSemantics': 'unified-plan',
    });
    if (!mounted || generation != _generation) {
      await peer.dispose();
      return;
    }
    _peer = peer;
    final connection = _connection;
    peer.onTrack = (event) async {
      if (!mounted || generation != _generation) return;
      if (event.track.kind != 'video') {
        event.track.enabled = false;
        return;
      }
      if (event.streams.isNotEmpty) {
        _remote = event.streams.first;
      } else {
        final remote = _remote ?? await createLocalMediaStream('received');
        if (!mounted || generation != _generation) {
          await remote.dispose();
          return;
        }
        _remote = remote;
        await remote.addTrack(event.track);
      }
      if (mounted && generation == _generation && _started) {
        _renderer.srcObject = _remote;
      }
    };
    peer.onIceCandidate = (ice) {
      if (generation == _generation &&
          _connection != null &&
          ice.candidate?.isNotEmpty == true) {
        unawaited(
          nativeConnection({
            'op': 'receive_ice',
            'id': _connection,
            'candidate': ice.candidate,
            'mid': ice.sdpMid ?? '0',
            'line': ice.sdpMLineIndex ?? 0,
          }).catchError((Object error) {
            if (mounted) setState(() => _error = error.toString());
            return <String, dynamic>{};
          }),
        );
      }
    };
    peer.onConnectionState = (state) {
      if (state == RTCPeerConnectionState.RTCPeerConnectionStateFailed &&
          generation == _generation) {
        unawaited(_stop());
      }
    };
    await peer.setRemoteDescription(RTCSessionDescription(sdp, 'offer'));
    if (generation != _generation || _disposing) return;
    final capabilities = await getRtpReceiverCapabilities('video');
    final codecs = h264VideoCodecPreferences(capabilities.codecs ?? []);
    if (codecs.isEmpty) {
      throw StateError('H.264 decoding is unavailable.');
    }
    for (final transceiver in await peer.getTransceivers()) {
      await transceiver.setCodecPreferences(codecs);
    }
    if (generation != _generation || _disposing) return;
    _remoteDescriptionSet = true;
    for (final ice in _pendingIce) {
      await peer.addCandidate(ice);
    }
    _pendingIce.clear();
    final answer = await peer.createAnswer();
    if (generation != _generation || _disposing) return;
    await peer.setLocalDescription(answer);
    if (generation != _generation || _disposing) return;
    await nativeConnection({
      'op': 'receive_answer',
      'id': connection,
      'sdp': answer.sdp,
    });
  }

  Future<void> _stop() =>
      _stopping ??= _performStop().whenComplete(() => _stopping = null);

  Future<void> _performStop() async {
    ++_generation;
    if (mounted && !_disposing) setState(() => _busy = true);
    _active = false;
    _connection = null;
    _renderer.srcObject = null;
    final peer = _peer;
    _peer = null;
    final remote = _remote;
    _remote = null;
    _remoteDescriptionSet = false;
    _started = false;
    _pendingIce.clear();
    try {
      final starting = _starting;
      if (starting != null) {
        try {
          await starting;
          _ownsReceiver = true;
        } catch (_) {}
      }
      if (_ownsReceiver) {
        _ownsReceiver = false;
        await nativeConnection({'op': 'receive_stop'});
      }
    } catch (error) {
      if (mounted && !_disposing) setState(() => _error = error.toString());
    } finally {
      await peer?.close();
      await peer?.dispose();
      await remote?.dispose();
    }
    if (mounted && !_disposing) {
      setState(() {
        _busy = false;
        _invitation = null;
        _status = 'Receiver stopped. Start again for a new invitation.';
      });
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('Receive another device’s screen')),
    body: ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(_status, style: Theme.of(context).textTheme.titleLarge),
        if (_error != null)
          SelectableText(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        if (!_active) ...[
          DropdownButtonFormField<String>(
            initialValue: _address,
            decoration: const InputDecoration(
              labelText: 'This device’s network address',
            ),
            items: _addresses
                .map(
                  (address) =>
                      DropdownMenuItem(value: address, child: Text(address)),
                )
                .toList(),
            onChanged: _busy
                ? null
                : (value) => setState(() => _address = value),
          ),
          FilledButton(
            onPressed: _busy || !_renderReady || _address == null
                ? null
                : _start,
            child: const Text('Start receiving / show QR'),
          ),
        ],
        if (_invitation case final invitation?) ...[
          Center(
            child: Semantics(
              label: 'Receiver QR invitation. Expires after two minutes.',
              child: QrImageView(
                data: invitation,
                size: 280,
                backgroundColor: Colors.white,
              ),
            ),
          ),
          SelectableText(invitation),
          OutlinedButton.icon(
            onPressed: () => Clipboard.setData(ClipboardData(text: invitation)),
            icon: const Icon(Icons.copy),
            label: const Text('Copy invitation'),
          ),
          const Text(
            'Use MirrorMesh on the sending device. QR pairing does not grant screen-capture permission.',
          ),
        ],
        if (_renderReady && _connection != null)
          AspectRatio(
            aspectRatio: 16 / 9,
            child: RTCVideoView(
              _renderer,
              objectFit: RTCVideoViewObjectFit.RTCVideoViewObjectFitContain,
            ),
          ),
        if (_active)
          FilledButton.tonal(
            onPressed: _stop,
            child: const Text('Stop receiving'),
          ),
      ],
    ),
  );

  @override
  void dispose() {
    _disposing = true;
    unawaited(_stop().whenComplete(_renderer.dispose));
    super.dispose();
  }
}
