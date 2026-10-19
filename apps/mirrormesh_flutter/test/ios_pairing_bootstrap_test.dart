import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/ios_pairing_bootstrap.dart';

void main() {
  const channel = MethodChannel(IosPairingBootstrap.channelName);

  TestWidgetsFlutterBinding.ensureInitialized();

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test(
    'asks iOS to prepare pairing support and returns its public ID',
    () async {
      MethodCall? receivedCall;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            receivedCall = call;
            return '9a73e6e5-5c95-8c02-9dd1-fc456e920215';
          });

      final deviceId = await const IosPairingBootstrap().prepare();

      expect(receivedCall?.method, 'preparePairingRuntime');
      expect(receivedCall?.arguments, isNull);
      expect(deviceId, '9a73e6e5-5c95-8c02-9dd1-fc456e920215');
    },
  );

  test('rejects a native response without a device ID', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (_) async => null);

    await expectLater(
      const IosPairingBootstrap().prepare(),
      throwsA(isA<PlatformException>()),
    );
  });
}
