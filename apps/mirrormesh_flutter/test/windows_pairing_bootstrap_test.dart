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

  test(
    'starts, polls, and cancels Windows sender pairing over the method channel',
    () async {
      final calls = <MethodCall>[];
      final fingerprint = List.filled(64, 'a').join();
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'beginSenderPairing' => 41,
              'pollSenderPairing' => <String, Object?>{
                'state': 3,
                'errorCode': 0,
                'receiverFingerprint': fingerprint,
              },
              'cancelSenderPairing' => true,
              _ => null,
            };
          });

      const bootstrap = WindowsPairingBootstrap();
      final requestId = await bootstrap.beginSenderPairing(
        'mirrormesh://pair?v=1',
      );
      final status = await bootstrap.pollSenderPairing(requestId);
      await bootstrap.cancelSenderPairing(requestId);

      expect(calls.map((call) => call.method), [
        'beginSenderPairing',
        'pollSenderPairing',
        'cancelSenderPairing',
      ]);
      expect(calls.first.arguments, 'mirrormesh://pair?v=1');
      expect(status.state, WindowsSenderPairingState.approved);
      expect(status.receiverFingerprint, fingerprint);
    },
  );

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

  testWidgets(
    'Windows Devices action pairs from a pasted TV invitation and exposes disconnect',
    (tester) async {
      const invitation = 'mirrormesh://pair?v=1';
      final calls = <MethodCall>[];
      final fingerprint = List.filled(64, 'a').join();
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'preparePairingRuntime' => '9a73e6e5-5c95-8c02-9dd1-fc456e920215',
              'beginSenderPairing' => 41,
              'pollSenderPairing' => <String, Object?>{
                'state': 3,
                'errorCode': 0,
                'receiverFingerprint': fingerprint,
              },
              'cancelSenderPairing' => true,
              _ => null,
            };
          });

      await tester.pumpWidget(const MirrorMeshApp());
      await tester.tap(find.text('Devices').first);
      await tester.pumpAndSettle();
      final prepareButton = find.byKey(
        const Key('prepare-windows-pairing-button'),
      );
      await tester.ensureVisible(prepareButton);
      await tester.tap(prepareButton);
      await tester.pumpAndSettle();

      final invitationField = find.byKey(
        const Key('windows-pairing-invitation-field'),
      );
      await tester.ensureVisible(invitationField);
      await tester.enterText(invitationField, invitation);
      final pairButton = find.byKey(
        const Key('begin-windows-sender-pairing-button'),
      );
      await tester.ensureVisible(pairButton);
      await tester.tap(pairButton);
      await tester.pumpAndSettle();

      expect(calls.any((call) => call.method == 'beginSenderPairing'), isTrue);
      expect(find.textContaining('video is not streaming'), findsOneWidget);
      expect(find.textContaining('Verified TV fingerprint:'), findsOneWidget);

      final disconnectButton = find.byKey(
        const Key('cancel-windows-sender-pairing-button'),
      );
      await tester.tap(disconnectButton);
      await tester.pumpAndSettle();
      expect(calls.any((call) => call.method == 'cancelSenderPairing'), isTrue);
    },
    skip: !Platform.isWindows,
  );
}
