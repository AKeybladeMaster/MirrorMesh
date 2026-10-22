import 'dart:io';

import 'native_connection.dart';
import 'receiver_window.dart';

/// Public OS facts and one local routed interface; never displayed as setup addresses.
Future<Map<String, String>> readDeviceContext() async {
  if (!Platform.isWindows &&
      !Platform.isIOS &&
      !Platform.isMacOS &&
      !Platform.isLinux) {
    throw UnsupportedError(
      'This platform does not provide device information.',
    );
  }
  final values = await ReceiverWindow.channel.invokeMapMethod<String, dynamic>(
    'device_context',
  );
  if (values == null) throw StateError('Device information is unavailable.');
  return values.map(
    (key, value) => MapEntry(key, value is String ? value : ''),
  );
}

Future<Map<String, String>> configureDeviceProfile() async {
  final device = await readDeviceContext();
  await nativeConnection({
    'op': 'configure_device',
    for (final key in [
      'name',
      'platform',
      'manufacturer',
      'model',
      'os_version',
    ])
      key: device[key] ?? '',
  });
  return device;
}

Future<String> preferredLanAddress() async {
  final device = await configureDeviceProfile();
  final value = device['lan_address'] ?? '';
  final address = InternetAddress.tryParse(value);
  if (address == null ||
      address.isLoopback ||
      address.isLinkLocal ||
      address.type != InternetAddressType.IPv4) {
    throw StateError('Connect to a Wi-Fi or Ethernet network, then try again.');
  }
  return address.address;
}
