import 'package:flutter/material.dart';

import 'app.dart';
import 'rust_session_status.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final rustSessionStatus = await RustSessionStatus.initialize();
  runApp(MirrorMeshApp(rustSessionStatus: rustSessionStatus));
}
