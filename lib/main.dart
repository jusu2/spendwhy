import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'app.dart';
import 'services/app_settings.dart';
import 'services/rust_backend.dart';

void main() {
  // 在同一个 Zone 里捕获 Flutter framework 异常 + Dart 异步异常 +
  // 平台分发线程异常，避免任何未处理错误把 App 留在黑屏状态。
  // 返回值是 Future<void>?，与 sync main 的语义匹配，无需 await。
  // ignore: discarded_futures
  runZonedGuarded<Future<void>>(
    () async {
      WidgetsFlutterBinding.ensureInitialized();

      FlutterError.onError = (FlutterErrorDetails details) {
        FlutterError.presentError(details);
        if (kDebugMode) {
          debugPrint('[FlutterError] ${details.exceptionAsString()}');
        }
      };

      PlatformDispatcher.instance.onError = (Object error, StackTrace stack) {
        debugPrint('[PlatformDispatcher] $error\n$stack');
        return true;
      };

      await RustBackend.init();
      final settings = await AppSettings.load();
      runApp(FragmentsApp(settings: settings));
    },
    (Object error, StackTrace stack) {
      debugPrint('[Zone] uncaught: $error\n$stack');
    },
  );
}
