import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../i18n/strings.dart';
import '../models/enums.dart';
import '../services/app_settings.dart';
import '../services/safety_classifier.dart';
import '../state/fragments_provider.dart';
import '../widgets/intensity_picker.dart';
import '../widgets/tag_picker.dart';
import '../widgets/underline_button.dart';
import 'safety_resources_page.dart';

class RecordPage extends StatefulWidget {
  const RecordPage({super.key});

  @override
  State<RecordPage> createState() => _RecordPageState();
}

class _RecordPageState extends State<RecordPage> {
  final _ctrl = TextEditingController();
  Set<FragmentTag> _tags = {};
  Intensity _intensity = Intensity.hard;
  bool _restoredDraft = false;
  Timer? _draftDebounce;
  static const _draftDebounceDuration = Duration(milliseconds: 500);

  @override
  void initState() {
    super.initState();
    final settings = context.read<AppSettings>();
    final draft = settings.draft;
    if (draft != null && draft.isNotEmpty) {
      _ctrl.text = draft;
      _restoredDraft = true;
    }
    _ctrl.addListener(_onTextChange);
  }

  void _onTextChange() {
    // UI 立即响应（按钮可用态等），但持久化通过 debounce 合并写入。
    setState(() {});
    _draftDebounce?.cancel();
    final text = _ctrl.text;
    _draftDebounce = Timer(_draftDebounceDuration, () {
      if (!mounted) return;
      unawaited(context.read<AppSettings>().setDraft(text));
    });
  }

  @override
  void dispose() {
    _draftDebounce?.cancel();
    _ctrl.removeListener(_onTextChange);
    _ctrl.dispose();
    super.dispose();
  }

  Future<bool> _onWillPop() async {
    if (_ctrl.text.trim().isEmpty) {
      await context.read<AppSettings>().setDraft(null);
      return true;
    }
    final confirm = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: Theme.of(context).scaffoldBackgroundColor,
        shape: const RoundedRectangleBorder(),
        content: Text(
          S.recordDiscardConfirm,
          style: Theme.of(context).textTheme.bodyMedium,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text(S.recordDiscardNo),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text(S.recordDiscardYes),
          ),
        ],
      ),
    );
    if (confirm == true && mounted) {
      // 选择放下不清除草稿：下次进来仍可看见
      // （“下次进来还在”是你设计的温柔点）
    }
    return confirm == true;
  }

  Future<void> _save() async {
    final text = _ctrl.text.trim();
    if (text.isEmpty) return;
    _draftDebounce?.cancel();
    final messenger = ScaffoldMessenger.of(context);
    final navigator = Navigator.of(context);
    final settings = context.read<AppSettings>();
    await context.read<FragmentsProvider>().addFragment(
      content: text,
      tags: _tags.toList(),
      intensity: _intensity,
      fadePeriod: settings.defaultPeriod,
      visibility: settings.visibility,
    );
    await settings.setDraft(null);
    if (!mounted) return;
    messenger.showSnackBar(
      SnackBar(
        content: Text(
          S.recordSaveSuccess,
          style: Theme.of(context).textTheme.bodyMedium,
        ),
        margin: const EdgeInsets.all(28),
        duration: const Duration(seconds: 2),
      ),
    );
    navigator.pop();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) async {
        if (didPop) return;
        final navigator = Navigator.of(context);
        final ok = await _onWillPop();
        if (ok && mounted) navigator.pop();
      },
      child: Scaffold(
        appBar: AppBar(
          leadingWidth: 64,
          leading: Padding(
            padding: const EdgeInsets.only(left: 24),
            child: IconButton(
              icon: const Icon(Icons.close, size: 18),
              onPressed: () async {
                final navigator = Navigator.of(context);
                final ok = await _onWillPop();
                if (ok && mounted) navigator.pop();
              },
            ),
          ),
        ),
        body: SafeArea(
          child: SingleChildScrollView(
            padding: const EdgeInsets.fromLTRB(28, 8, 28, 140),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(S.recordTitle, style: theme.textTheme.headlineLarge),
                const SizedBox(height: 6),
                Text(
                  _restoredDraft ? S.draftRestoredHint : S.recordHint,
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontStyle: FontStyle.italic,
                  ),
                ),
                const SizedBox(height: 28),
                Container(
                  height: 0.6,
                  width: 24,
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.45),
                ),
                const SizedBox(height: 22),
                TextField(
                  controller: _ctrl,
                  maxLines: 12,
                  minLines: 8,
                  autofocus: true,
                  style: theme.textTheme.bodyLarge,
                  decoration: const InputDecoration(hintText: '……'),
                ),
                const SizedBox(height: 36),
                Text(
                  S.recordTagsTitle.toUpperCase(),
                  style: theme.textTheme.titleMedium,
                ),
                const SizedBox(height: 18),
                TagPicker(
                  selected: _tags,
                  onChanged: (s) => setState(() => _tags = s),
                ),
                const SizedBox(height: 36),
                Text(
                  S.recordIntensityTitle.toUpperCase(),
                  style: theme.textTheme.titleMedium,
                ),
                const SizedBox(height: 18),
                IntensityPicker(
                  value: _intensity,
                  onChanged: (i) => setState(() => _intensity = i),
                ),
              ],
            ),
          ),
        ),
        bottomNavigationBar: SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(28, 12, 28, 24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _SafetyHint(text: _ctrl.text),
                Align(
                  alignment: Alignment.centerRight,
                  child: UnderlineButton(
                    label: S.recordSave,
                    onTap: _ctrl.text.trim().isEmpty ? null : _save,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// 记录页底部的温柔提示条。仅在检测到高关注关键词时出现，
/// 以极细补充文本 + 下划线“看一下资源”入口表达，不阻断保存。
class _SafetyHint extends StatelessWidget {
  final String text;
  const _SafetyHint({required this.text});

  @override
  Widget build(BuildContext context) {
    final signal = SafetyClassifier.classify(text);
    if (signal.level == SafetyLevel.none) return const SizedBox.shrink();
    final theme = Theme.of(context);
    final isElevated = signal.level == SafetyLevel.elevated;
    final hint = isElevated ? S.safetyConcern : S.safetyGentle;
    return Padding(
      padding: const EdgeInsets.only(bottom: 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            height: 0.6,
            width: 18,
            color: theme.colorScheme.onSurface.withValues(alpha: 0.55),
          ),
          const SizedBox(height: 10),
          Text(
            hint,
            style: theme.textTheme.bodyMedium?.copyWith(
              fontStyle: FontStyle.italic,
              height: 1.7,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.75),
            ),
          ),
          if (isElevated) ...[
            const SizedBox(height: 8),
            GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (_) => const SafetyResourcesPage(),
                ),
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    S.safetyOpenResources,
                    style: theme.textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 4),
                  Container(
                    height: 0.6,
                    width: 28,
                    color: theme.colorScheme.onSurface,
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}
