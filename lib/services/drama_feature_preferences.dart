import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

class DramaFeaturePreferences {
  DramaFeaturePreferences._();

  static final instance = DramaFeaturePreferences._();
  static const _key = 'drama_feature_enabled';

  // Android Reader Lite hides video entry points even after an app upgrade.
  static bool get _available =>
      kIsWeb || defaultTargetPlatform != TargetPlatform.android;

  final ValueNotifier<bool> enabled = ValueNotifier(_available);
  Future<void>? _loading;

  Future<void> load() => _loading ??= _load();

  Future<void> _load() async {
    if (!_available) return;
    try {
      final preferences = await SharedPreferences.getInstance();
      enabled.value = preferences.getBool(_key) ?? true;
    } catch (_) {
      _loading = null;
      rethrow;
    }
  }

  Future<void> setEnabled(bool value) async {
    if (!_available) return;
    await load();
    final preferences = await SharedPreferences.getInstance();
    if (!await preferences.setBool(_key, value)) {
      throw StateError('Could not save short drama preference');
    }
    enabled.value = value;
  }
}
