import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/app.dart';
import 'package:mirrormesh_flutter/windows_pairing_bootstrap.dart';

void main() {
  const channel = MethodChannel(WindowsPairingBootstrap.channelName);

  TestWidgetsFlutterBinding.ensureInitialized();

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test(
    'asks Windows to prepare pairing support and returns its public ID',
    () async {
      MethodCall? receivedCall;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            receivedCall = call;
            return '9a73e6e5-5c95-8c02-9dd1-fc456e920215';
          });

      final deviceId = await const WindowsPairingBootstrap().prepare();

      expect(receivedCall?.method, 'preparePairingRuntime');
      expect(receivedCall?.arguments, isNull);
      expect(deviceId, '9a73e6e5-5c95-8c02-9dd1-fc456e920215');
    },
  );

  test('rejects a native response without a device ID', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (_) async => null);

    await expectLater(
      const WindowsPairingBootstrap().prepare(),
      throwsA(isA<PlatformException>()),
    );
  });

  testWidgets(
    'Windows Devices action prepares pairing only after the user taps it',
    (tester) async {
      const deviceId = '9a73e6e5-5c95-8c02-9dd1-fc456e920215';
      MethodCall? receivedCall;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            receivedCall = call;
            return deviceId;
          });

      await tester.pumpWidget(const MirrorMeshApp());
      await tester.tap(find.text('Devices').first);
      await tester.pumpAndSettle();
      expect(receivedCall, isNull);
      final prepareButton = find.byKey(
        const Key('prepare-windows-pairing-button'),
      );
      await tester.ensureVisible(prepareButton);
      await tester.tap(prepareButton);
      await tester.pumpAndSettle();

      expect(receivedCall?.method, 'preparePairingRuntime');
      expect(find.text('Local device ID: $deviceId'), findsOneWidget);
    },
    skip: !Platform.isWindows,
  );
}
