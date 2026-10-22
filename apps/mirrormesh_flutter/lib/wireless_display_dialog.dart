import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'receiver_window.dart';

/// Windows owns Miracast approval, media and Stop. This only observes its status.
class WirelessDisplayDialog extends StatefulWidget {
  const WirelessDisplayDialog({super.key});

  @override
  State<WirelessDisplayDialog> createState() => _WirelessDisplayDialogState();
}

class _WirelessDisplayDialogState extends State<WirelessDisplayDialog> {
  Timer? _timer;
  bool _polling = false;
  String _wifi = 'unknown';
  String _listening = 'checking';
  String? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_poll());
    _timer = Timer.periodic(
      const Duration(seconds: 2),
      (_) => unawaited(_poll()),
    );
  }

  Future<void> _poll() async {
    if (_polling) return;
    _polling = true;
    try {
      final status = await ReceiverWindow.channel
          .invokeMapMethod<String, dynamic>('wireless_display_status');
      if (!mounted || status == null || status['pending'] == true) return;
      setState(() {
        _wifi = status['wifi'] as String? ?? 'unknown';
        _listening = status['listening'] as String? ?? 'unknown';
        _error = (status['error'] as String?)?.isNotEmpty == true
            ? status['error'] as String
            : null;
      });
    } on PlatformException catch (error) {
      if (mounted) {
        setState(
          () => _error = error.message ?? 'Windows status is unavailable.',
        );
      }
    } finally {
      _polling = false;
    }
  }

  Future<void> _settings() async {
    try {
      await ReceiverWindow.channel.invokeMethod<void>(
        'wireless_display_settings',
      );
    } on PlatformException catch (error) {
      if (mounted) setState(() => _error = error.message);
    }
  }

  String get _message => switch (_listening) {
    'listening' => 'Windows is listening. Select this PC in Android Wireless display, then approve in the Windows receiver window.',
    'pending' => 'Windows reports a pending connection. Check the Wireless Display window for approval or a PIN.',
    'connected' => 'Windows reports a Miracast connection. View and disconnect in the Wireless Display window.',
    'policy' => 'Windows policy has disabled projection. Contact the administrator or use MirrorMesh on both devices.',
    'disabled' => 'Windows has temporarily disabled projection. Check the Wireless Display window and projection settings.',
    'not_listening' => 'Windows is not listening. Keep the Wireless Display window open and check Projecting to this PC in Settings.',
    _ => 'Checking the Windows receiver. Wireless Display handles approval, rendering and disconnect in its own window.',
  };

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Windows Wireless Display'),
    content: SizedBox(
      width: 460,
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(_error ?? _message, semanticsLabel: _error ?? _message),
            const SizedBox(height: 16),
            Text('Wi-Fi Miracast support: $_wifi'),
            const SizedBox(height: 12),
            const Text(
              'If the PC appears but connection fails, discovery has succeeded. Check the Windows receiver message and Wi-Fi/display drivers. A visible PC does not confirm a working media connection.',
            ),
            const SizedBox(height: 12),
            const Text(
              'Google Cast uses a different protocol. Use Android Wireless display / Miracast, or the MirrorMesh Android app.',
            ),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: _settings,
        child: const Text('Projection settings'),
      ),
      FilledButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('Done'),
      ),
    ],
  );

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }
}
