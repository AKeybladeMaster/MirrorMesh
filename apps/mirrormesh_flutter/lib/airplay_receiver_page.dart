import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'receiver_window.dart';
import 'connection_popups.dart';

/// Native media stays in the isolated receiver; Dart carries consent and status.
class AirPlayReceiverPage extends StatefulWidget {
  const AirPlayReceiverPage({super.key});
  @override
  State<AirPlayReceiverPage> createState() => _AirPlayReceiverPageState();
}

class _AirPlayReceiverPageState extends State<AirPlayReceiverPage> {
  final _stage = GlobalKey();
  Timer? _timer;
  bool _running = false, _ready = false, _video = false, _busy = false;
  bool _polling = false, _fullscreen = false, _closing = false;
  int _request = 0, _generation = 0;
  int _overlayCount = 0;
  String _pin = '', _error = '';
  ConnectionPopup<void>? _startupPopup, _pinPopup;
  ConnectionPopup<bool>? _approvalPopup;
  static const _channel = ReceiverWindow.channel;

  Future<void> _start() async {
    if (_busy || _running || _closing) return;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _error = '';
    });
    _startupPopup = ConnectionPopup<void>(
      context,
      (context) => PopScope(
        canPop: false,
        child: AlertDialog(
          title: const Text('Starting iPhone / iPad receiver'),
          content: const Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              CircularProgressIndicator.adaptive(),
              SizedBox(height: 16),
              Text('Preparing AirPlay on your local network.'),
            ],
          ),
          actions: [TextButton(onPressed: _stop, child: const Text('Cancel'))],
        ),
      ),
    );
    try {
      await _channel.invokeMethod<void>('airplay_start');
      if (_closing || !mounted || generation != _generation) return;
      setState(() => _running = true);
      _timer = Timer.periodic(
        const Duration(milliseconds: 250),
        (_) => unawaited(_poll()),
      );
      await _poll();
    } catch (error) {
      _startupPopup?.close();
      _startupPopup = null;
      if (mounted && generation == _generation) {
        await connectionNotice(
          context,
          'AirPlay unavailable',
          _describe(error),
        );
      }
    } finally {
      if (mounted && generation == _generation) setState(() => _busy = false);
    }
  }

  Future<void> _poll() async {
    if (_polling || _closing || !_running) return;
    _polling = true;
    final generation = _generation;
    try {
      final state = await _channel.invokeMapMethod<String, dynamic>(
        'airplay_poll',
      );
      if (!mounted || _closing || generation != _generation || state == null) {
        return;
      }
      final previousPin = _pin,
          previousRequest = _request,
          previousError = _error;
      setState(() {
        _running = state['running'] == true;
        _ready = state['ready'] == true;
        _video = state['video'] == true;
        _fullscreen = state['fullscreen'] == true;
        _pin = state['pin'] as String? ?? '';
        _request = state['request'] as int? ?? 0;
        _error = state['error'] as String? ?? '';
      });
      if (_ready || !_running) {
        _startupPopup?.close();
        _startupPopup = null;
      }
      if (_pin.isEmpty || _request > 0 || !_running) {
        _pinPopup?.close();
        _pinPopup = null;
      }
      if (_request == 0 || !_running) {
        _approvalPopup?.close(false);
        _approvalPopup = null;
      }
      if (_pin.isNotEmpty && _pin != previousPin && _request == 0 && !_video) {
        _pinPopup?.close();
        _pinPopup = ConnectionPopup<void>(
          context,
          (context) => PopScope(
            canPop: false,
            child: AlertDialog(
              title: const Text('Enter this code on your iPhone or iPad'),
              content: Semantics(
                liveRegion: true,
                child: SelectableText(
                  _pin,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.displaySmall
                      ?.copyWith(letterSpacing: 8),
                ),
              ),
              actions: [
                TextButton(
                  onPressed: _stop,
                  child: const Text('Stop receiver'),
                ),
              ],
            ),
          ),
        );
      }
      if (_request > 0 && _request != previousRequest) {
        _approvalPopup?.close(false);
        final details = Map<String, dynamic>.from(
          state['device'] as Map? ?? {},
        );
        unawaited(_approveDevice(_request, generation, details));
      }
      if (!_running) _timer?.cancel();
      if (_error.isNotEmpty && _error != previousError) {
        unawaited(connectionNotice(context, 'AirPlay ended', _error));
      }
    } catch (error) {
      if (generation == _generation) {
        await _stop();
        if (mounted) {
          await connectionNotice(context, 'AirPlay ended', _describe(error));
        }
      }
    } finally {
      _polling = false;
    }
  }

  Future<void> _approveDevice(
    int request,
    int generation,
    Map<String, dynamic> details,
  ) async {
    ++_overlayCount;
    try {
      // A native child HWND otherwise covers Flutter modal routes during video.
      await _channel.invokeMethod<void>('airplay_overlay', true);
      if (!mounted ||
          _closing ||
          generation != _generation ||
          request != _request) {
        return;
      }
      await _approvalFlow(request, generation, details);
    } catch (error) {
      if (mounted && !_closing && generation == _generation) {
        await connectionNotice(
          context,
          'Approval unavailable',
          _describe(error),
        );
      }
    } finally {
      --_overlayCount;
      try {
        await _channel.invokeMethod<void>('airplay_overlay', _overlayCount > 0);
      } catch (_) {}
    }
  }

  Future<void> _approvalFlow(
    int request,
    int generation,
    Map<String, dynamic> details,
  ) async {
    final allow = await confirmIncomingDevice(
      context,
      details,
      onOpened: (popup) => _approvalPopup = popup,
    );
    if (!mounted ||
        _closing ||
        generation != _generation ||
        request != _request) {
      return;
    }
    _approvalPopup = null;
    try {
      await withConnectionProgress(
        context,
        allow ? 'Approving connection' : 'Rejecting connection',
        () => _channel.invokeMethod<void>('airplay_decide', {
          'request': request,
          'allow': allow,
        }),
      );
      if (!mounted || generation != _generation) return;
      setState(() => _request = 0);
      await connectionNotice(
        context,
        allow ? 'Connection approved' : 'Connection rejected',
        allow
            ? 'Your iPhone or iPad may now share its screen.'
            : 'No screen access was granted.',
        brief: true,
      );
    } catch (error) {
      if (mounted) {
        await connectionNotice(context, 'Connection ended', _describe(error));
      }
    }
  }

  Future<void> _stop() async {
    ++_generation;
    _timer?.cancel();
    _startupPopup?.close();
    _startupPopup = null;
    _pinPopup?.close();
    _pinPopup = null;
    _approvalPopup?.close(false);
    _approvalPopup = null;
    try {
      await _channel.invokeMethod<void>('airplay_stop');
    } catch (error) {
      if (mounted && !_closing) {
        await connectionNotice(
          context,
          'Receiver cleanup failed',
          _describe(error),
        );
      }
    }
    if (mounted) {
      setState(() {
        _running = _ready = _video = _fullscreen = _busy = false;
        _pin = '';
        _request = 0;
      });
    }
  }

  Future<void> _toggleFullscreen() async {
    try {
      final next = !_fullscreen;
      await ReceiverWindow.setFullscreen(next);
      if (mounted) setState(() => _fullscreen = next);
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    }
  }

  void _layoutStage() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _closing || !_running) return;
      final box = _stage.currentContext?.findRenderObject() as RenderBox?;
      if (box == null || !box.hasSize || box.size.isEmpty) return;
      final offset = box.localToGlobal(Offset.zero);
      final scale = View.of(context).devicePixelRatio;
      unawaited(
        _channel
            .invokeMethod<void>('airplay_bounds', {
              'x': offset.dx * scale,
              'y': offset.dy * scale,
              'width': box.size.width * scale,
              'height': box.size.height * scale,
            })
            .catchError((Object error) {
              if (mounted && !_closing) {
                setState(() => _error = _describe(error));
              }
            }),
      );
    });
  }

  String _describe(Object error) => error is PlatformException
      ? error.message ?? 'AirPlay is unavailable.'
      : 'The AirPlay connection was interrupted.';

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(
      title: const Text('Receive iPhone / iPad'),
      actions: [
        IconButton(
          tooltip: _fullscreen ? 'Exit fullscreen' : 'Fullscreen',
          onPressed: _toggleFullscreen,
          icon: const Icon(Icons.fullscreen),
        ),
      ],
    ),
    body: CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.f11): _toggleFullscreen,
        const SingleActivator(LogicalKeyboardKey.escape): () {
          if (_fullscreen) unawaited(_toggleFullscreen());
        },
      },
      child: Focus(
        autofocus: true,
        child: SafeArea(
          child: Column(
            children: [
              Flexible(
                flex: _video ? 1 : 3,
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(24),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        _video
                            ? 'Receiving your screen'
                            : _ready
                            ? 'Ready for your iPhone or iPad'
                            : 'Receive with AirPlay',
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                      const SizedBox(height: 12),
                      if (!_video)
                        const Text(
                          'Connect to the same network. Open Control Center on iPhone or iPad, choose Screen Mirroring, then MirrorMesh PC. Enter the code shown in the popup and approve this device here.',
                        ),
                      const SizedBox(height: 20),
                      if (_running)
                        FilledButton.tonalIcon(
                          onPressed: _stop,
                          icon: const Icon(Icons.stop),
                          label: const Text('Stop receiving'),
                        )
                      else
                        FilledButton.icon(
                          onPressed: _busy ? null : _start,
                          icon: const Icon(Icons.airplay),
                          label: const Text('Start iPhone / iPad receiver'),
                        ),
                    ],
                  ),
                ),
              ),
              Expanded(
                flex: _video ? 5 : 1,
                child: LayoutBuilder(
                  builder: (context, constraints) {
                    _layoutStage();
                    return ColoredBox(
                      key: _stage,
                      color: Colors.black,
                      child: const SizedBox.expand(),
                    );
                  },
                ),
              ),
            ],
          ),
        ),
      ),
    ),
  );

  @override
  void dispose() {
    _closing = true;
    ++_generation;
    _timer?.cancel();
    _startupPopup?.close();
    _pinPopup?.close();
    _approvalPopup?.close(false);
    unawaited(
      _channel.invokeMethod<void>('airplay_stop').catchError((Object _) {}),
    );
    super.dispose();
  }
}
