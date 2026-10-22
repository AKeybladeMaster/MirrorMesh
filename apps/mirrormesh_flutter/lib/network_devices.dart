import 'dart:convert';
import 'dart:ffi';
import 'dart:isolate';
import 'dart:io';

/// Reuses the shared Rust DNS-SD adapter; this never scans a subnet or trusts a result.
Future<List<Map<String, dynamic>>> discoverNetworkDevices() => Isolate.run(() {
  final library = Platform.isMacOS
      ? DynamicLibrary.process()
      : DynamicLibrary.open(Platform.isWindows ? 'mm_ffi.dll' : 'libmm_ffi.so');
  final discover = library
      .lookupFunction<Pointer<Uint8> Function(), Pointer<Uint8> Function()>(
        'mirrormesh_discover_devices',
      );
  final release = library
      .lookupFunction<
        Void Function(Pointer<Uint8>),
        void Function(Pointer<Uint8>)
      >('mirrormesh_free_json');
  final pointer = discover();
  if (pointer == nullptr) throw StateError('Device search returned no data.');
  try {
    var length = 0;
    while (length < 128 * 1024 && pointer[length] != 0) {
      length++;
    }
    if (length == 128 * 1024) {
      throw StateError('Device search exceeded its limit.');
    }
    final data = jsonDecode(
      utf8.decode(pointer.asTypedList(length)),
    ) as Map<String, dynamic>;
    if (data['ok'] != true) throw StateError(data['error'] as String);
    return (data['devices'] as List).cast<Map<String, dynamic>>();
  } finally {
    release(pointer);
  }
});
