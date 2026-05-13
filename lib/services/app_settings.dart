import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/enums.dart';

/// 全局应用设置：持久化到 shared_preferences。
///
/// 在 main 中通过 [AppSettings.load] 一次加载完成后注入 Provider 树。
class AppSettings extends ChangeNotifier {
  static const _kOnboarded = 'onboarded';
  static const _kDefaultPeriodDays = 'default_fade_days';
  static const _kBio = 'bio_lock';
  static const _kNotify = 'notifications';
  static const _kVisibility = 'share_visibility';
  static const _kDraft = 'record_draft';

  final SharedPreferences _prefs;

  AppSettings._(this._prefs);

  static Future<AppSettings> load() async {
    final prefs = await SharedPreferences.getInstance();
    return AppSettings._(prefs);
  }

  // === Onboarding ===
  bool get onboarded => _prefs.getBool(_kOnboarded) ?? false;
  Future<void> markOnboarded() async {
    await _prefs.setBool(_kOnboarded, true);
    notifyListeners();
  }

  // === Default fade period ===
  FadePeriod get defaultPeriod {
    final days = _prefs.getInt(_kDefaultPeriodDays) ?? 270;
    return FadePeriod.values.firstWhere(
      (p) => p.days == days,
      orElse: () => FadePeriod.nineMonths,
    );
  }

  Future<void> setDefaultPeriod(FadePeriod p) async {
    await _prefs.setInt(_kDefaultPeriodDays, p.days);
    notifyListeners();
  }

  // === Biometric lock ===
  bool get biometricLock => _prefs.getBool(_kBio) ?? false;
  Future<void> setBiometricLock(bool v) async {
    await _prefs.setBool(_kBio, v);
    notifyListeners();
  }

  // === Notifications ===
  bool get notifications => _prefs.getBool(_kNotify) ?? false;
  Future<void> setNotifications(bool v) async {
    await _prefs.setBool(_kNotify, v);
    notifyListeners();
  }

  // === Visibility ===
  ShareVisibility get visibility {
    final i = _prefs.getInt(_kVisibility) ?? 0;
    return ShareVisibility.values[i.clamp(
      0,
      ShareVisibility.values.length - 1,
    )];
  }

  Future<void> setVisibility(ShareVisibility v) async {
    await _prefs.setInt(_kVisibility, v.index);
    notifyListeners();
  }

  // === Draft (无 notify，输入时频繁调用) ===
  String? get draft => _prefs.getString(_kDraft);
  Future<void> setDraft(String? text) async {
    if (text == null || text.isEmpty) {
      await _prefs.remove(_kDraft);
    } else {
      await _prefs.setString(_kDraft, text);
    }
  }
}
