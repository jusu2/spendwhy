import 'package:flutter/material.dart';

import 'app.dart';
import 'services/app_settings.dart';
import 'services/rust_backend.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustBackend.init();
  final settings = await AppSettings.load();
  runApp(FragmentsApp(settings: settings));
}
