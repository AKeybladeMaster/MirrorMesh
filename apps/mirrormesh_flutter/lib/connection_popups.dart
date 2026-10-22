import 'dart:async';
import 'dart:io';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

/// Owns only its own route, so completion cannot dismiss an incoming approval.
class ConnectionPopup<T> {
  ConnectionPopup(BuildContext context, WidgetBuilder builder) {
    _navigator = Navigator.of(context, rootNavigator: true);
    _route = Platform.isIOS
        ? CupertinoDialogRoute<T>(
            context: context,
            builder: builder,
            barrierDismissible: false,
          )
        : DialogRoute<T>(
            context: context,
            builder: builder,
            barrierDismissible: false,
          );
    result = _navigator.push<T>(_route);
  }
  late final NavigatorState _navigator;
  late final Route<T> _route;
  late final Future<T?> result;
  void close([T? value]) {
    if (_route.isActive) _navigator.removeRoute<T>(_route, value);
  }
}

Widget popupAction(
  BuildContext context,
  String label,
  VoidCallback? action, {
  bool primary = false,
  bool destructive = false,
}) {
  if (Platform.isIOS) {
    return CupertinoDialogAction(
      onPressed: action,
      isDefaultAction: primary,
      isDestructiveAction: destructive,
      child: Text(label),
    );
  }
  return primary
      ? FilledButton(onPressed: action, child: Text(label))
      : TextButton(onPressed: action, child: Text(label));
}

Future<T> withConnectionProgress<T>(
  BuildContext context,
  String title,
  Future<T> Function() operation,
) async {
  final popup = ConnectionPopup<void>(
    context,
    (context) => PopScope(
      canPop: false,
      child: AlertDialog.adaptive(
        title: Text(title),
        content: const Padding(
          padding: EdgeInsets.symmetric(vertical: 20),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [CircularProgressIndicator.adaptive()],
          ),
        ),
      ),
    ),
  );
  try {
    return await operation();
  } finally {
    popup.close();
  }
}

Future<void> connectionNotice(
  BuildContext context,
  String title,
  String message, {
  bool brief = false,
}) async {
  late ConnectionPopup<void> popup;
  popup = ConnectionPopup<void>(
    context,
    (context) => AlertDialog.adaptive(
      title: Text(title),
      content: Text(message),
      actions: [
        popupAction(context, 'Done', () => popup.close(), primary: true),
      ],
    ),
  );
  final timer = brief
      ? Timer(const Duration(seconds: 2), () => popup.close())
      : null;
  try {
    await popup.result;
  } finally {
    timer?.cancel();
  }
}

String deviceText(Object? value, [String fallback = 'Not supplied']) {
  if (value is! String) return fallback;
  final text = String.fromCharCodes(
    value.runes
        .where(
          (rune) =>
              rune >= 32 &&
              !(rune >= 127 && rune <= 159) &&
              rune != 0x061c &&
              rune != 0x200e &&
              rune != 0x200f &&
              !(rune >= 0x202a && rune <= 0x202e) &&
              !(rune >= 0x2066 && rune <= 0x2069),
        )
        .take(160),
  ).trim();
  return text.isEmpty ? fallback : text;
}

Widget deviceFacts(Map<String, dynamic> request) {
  final details = request['device_details'] is Map
      ? request['device_details'] as Map
      : const {};
  final rows = <(String, String)>[
    ('Device name', deviceText(request['display_name'], 'Unknown device')),
    ('Model', deviceText(details['model'])),
    ('Manufacturer', deviceText(details['manufacturer'])),
    (
      'System',
      deviceText(details['os_version'], deviceText(details['platform'])),
    ),
    if (request['source_ip'] is String)
      ('Address', deviceText(request['source_ip'])),
  ];
  return Column(
    mainAxisSize: MainAxisSize.min,
    children: [
      for (final row in rows)
        Padding(
          padding: const EdgeInsets.only(bottom: 10),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(width: 104, child: Text(row.$1)),
              Expanded(child: Text(row.$2)),
            ],
          ),
        ),
      const SizedBox(height: 8),
      const Text(
        'Device details are reported by the sender. Approve only the device you selected.',
      ),
    ],
  );
}

Future<bool> confirmIncomingDevice(
  BuildContext context,
  Map<String, dynamic> request, {
  void Function(ConnectionPopup<bool>)? onOpened,
}) async {
  var permission = false;
  late ConnectionPopup<bool> popup;
  popup = ConnectionPopup<bool>(
    context,
    (context) => StatefulBuilder(
      builder: (context, update) => PopScope(
        canPop: false,
        child: AlertDialog.adaptive(
          title: Text(
            'Receive from ${deviceText(request['display_name'], 'this device')}?',
          ),
          content: ConstrainedBox(
            constraints: BoxConstraints(
              maxHeight: MediaQuery.sizeOf(context).height * 0.55,
            ),
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  deviceFacts(request),
                  if (request['fingerprint'] case final String fingerprint) ...[
                    const SizedBox(height: 12),
                    const Text('Identity fingerprint'),
                    SelectableText(deviceText(fingerprint)),
                  ],
                  if (request['verification_code'] case final String code) ...[
                    const SizedBox(height: 16),
                    Text(
                      'Connection code',
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                    SelectableText(
                      code,
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const Text('Compare this code on both devices.'),
                  ],
                  const SizedBox(height: 16),
                  Row(
                    children: [
                      Checkbox.adaptive(
                        value: permission,
                        onChanged: (value) =>
                            update(() => permission = value == true),
                      ),
                      Expanded(
                        child: Text(
                          request['verification_code'] is String
                              ? 'The codes match. Allow this device to send its screen'
                              : 'Allow this device to send its screen',
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          actions: [
            popupAction(
              context,
              'Reject',
              () => popup.close(false),
              destructive: true,
            ),
            popupAction(
              context,
              'Approve',
              permission &&
                      request['requested_grants'] is Map &&
                      request['requested_grants']['view'] == true
                  ? () => popup.close(true)
                  : null,
              primary: true,
            ),
          ],
        ),
      ),
    ),
  );
  onOpened?.call(popup);
  return await popup.result == true;
}
