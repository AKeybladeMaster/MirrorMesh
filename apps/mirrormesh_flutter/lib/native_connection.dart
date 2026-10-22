import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

/// Native metadata only. Capture and rendering remain inside libwebrtc.
Future<Map<String, dynamic>> nativeConnection(
  Map<String, dynamic> command,
) => Isolate.run(() {
  final library = (Platform.isIOS || Platform.isMacOS)
      ? DynamicLibrary.process()
      : DynamicLibrary.open(Platform.isWindows ? 'mm_ffi.dll' : 'libmm_ffi.so');
  final allocate = library
      .lookupFunction<
        Pointer<Uint8> Function(UintPtr),
        Pointer<Uint8> Function(int)
      >('mirrormesh_alloc_command');
  final releaseInput = library
      .lookupFunction<
        Void Function(Pointer<Uint8>, UintPtr),
        void Function(Pointer<Uint8>, int)
      >('mirrormesh_release_command');
  final invoke = library
      .lookupFunction<
        Pointer<Uint8> Function(Pointer<Uint8>, UintPtr),
        Pointer<Uint8> Function(Pointer<Uint8>, int)
      >('mirrormesh_connection_command');
  final releaseOutput = library
      .lookupFunction<
        Void Function(Pointer<Uint8>),
        void Function(Pointer<Uint8>)
      >('mirrormesh_free_json');
  final bytes = utf8.encode(jsonEncode(command));
  final input = allocate(bytes.length);
  if (input == nullptr) throw StateError('Connection command is too large.');
  Pointer<Uint8> output = nullptr;
  try {
    input.asTypedList(bytes.length).setAll(0, bytes);
    output = invoke(input, bytes.length);
    if (output == nullptr) {
      throw StateError('Native connection returned no status.');
    }
    var length = 0;
    while (length < 512 * 1024 && output[length] != 0) {
      length++;
    }
    if (length == 512 * 1024) {
      throw StateError('Native status exceeded its limit.');
    }
    final result = jsonDecode(
      utf8.decode(output.asTypedList(length)),
    ) as Map<String, dynamic>;
    if (result['ok'] != true) {
      throw StateError(
        result['error']?.toString() ?? 'Connection operation failed.',
      );
    }
    return result;
  } finally {
    releaseInput(input, bytes.length);
    if (output != nullptr) releaseOutput(output);
  }
});
