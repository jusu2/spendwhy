import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../i18n/strings.dart';
import '../models/fragment.dart';
import '../state/fragments_provider.dart';

/// 添加恢复（"今天好一点了"）的浮层
class RecoveryDialog extends StatefulWidget {
  final Fragment? prefilledFragment;
  const RecoveryDialog({super.key, this.prefilledFragment});

  @override
  State<RecoveryDialog> createState() => _RecoveryDialogState();
}

class _RecoveryDialogState extends State<RecoveryDialog> {
  final _ctrl = TextEditingController();
  int _intensity = 3;
  late Set<String> _related;

  @override
  void initState() {
    super.initState();
    _related = widget.prefilledFragment != null
        ? {widget.prefilledFragment!.id}
        : <String>{};
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final provider = context.watch<FragmentsProvider>();
    return Dialog(
      shape: const RoundedRectangleBorder(),
      backgroundColor: theme.scaffoldBackgroundColor,
      insetPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 80),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(28, 32, 28, 20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(S.recoveryTitle, style: theme.textTheme.headlineSmall),
            const SizedBox(height: 4),
            Text(S.recoveryHint, style: theme.textTheme.bodySmall),
            const SizedBox(height: 22),
            TextField(
              controller: _ctrl,
              maxLines: 3,
              minLines: 2,
              autofocus: true,
              style: theme.textTheme.bodyLarge,
              decoration: const InputDecoration(hintText: '……'),
              onChanged: (_) => setState(() {}),
            ),
            const Divider(height: 24),
            Text(
              S.recoveryIntensityTitle.toUpperCase(),
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 14),
            // 复用 IntensityPicker 的视觉，但内部数值不同
            Row(
              children: List.generate(5, (i) {
                final v = i + 1;
                final picked = v <= _intensity;
                final size = 8.0 + v * 1.6;
                return Padding(
                  padding: const EdgeInsets.only(right: 14),
                  child: GestureDetector(
                    onTap: () => setState(() => _intensity = v),
                    behavior: HitTestBehavior.opaque,
                    child: Container(
                      width: 28,
                      height: 28,
                      alignment: Alignment.center,
                      child: AnimatedContainer(
                        duration: const Duration(milliseconds: 220),
                        width: size,
                        height: size,
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          color: picked
                              ? const Color(0xFFD4B896)
                              : theme.colorScheme.onSurface.withValues(
                                  alpha: 0.18,
                                ),
                        ),
                      ),
                    ),
                  ),
                );
              }),
            ),
            if (provider.fragments.isNotEmpty) ...[
              const Divider(height: 32),
              Text(
                S.recoveryRelatedTitle.toUpperCase(),
                style: theme.textTheme.titleMedium,
              ),
              const SizedBox(height: 10),
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 160),
                child: SingleChildScrollView(
                  child: Column(
                    children: provider.fragments.take(8).map((f) {
                      final picked = _related.contains(f.id);
                      return GestureDetector(
                        behavior: HitTestBehavior.opaque,
                        onTap: () => setState(() {
                          if (picked) {
                            _related.remove(f.id);
                          } else {
                            _related.add(f.id);
                          }
                        }),
                        child: Padding(
                          padding: const EdgeInsets.symmetric(vertical: 8),
                          child: Row(
                            children: [
                              Container(
                                width: 6,
                                height: 6,
                                decoration: BoxDecoration(
                                  shape: BoxShape.circle,
                                  color: picked
                                      ? theme.colorScheme.onSurface
                                      : theme.colorScheme.onSurface.withValues(
                                          alpha: 0.25,
                                        ),
                                ),
                              ),
                              const SizedBox(width: 12),
                              Expanded(
                                child: Text(
                                  f.content,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: theme.textTheme.bodyMedium?.copyWith(
                                    color: picked
                                        ? theme.colorScheme.onSurface
                                        : theme.colorScheme.onSurface
                                              .withValues(alpha: 0.5),
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      );
                    }).toList(),
                  ),
                ),
              ),
            ],
            const SizedBox(height: 24),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(false),
                  child: const Text(S.cancel),
                ),
                const SizedBox(width: 14),
                _UnderlineAction(
                  label: S.recoverySave,
                  enabled: _ctrl.text.trim().isNotEmpty,
                  onTap: () async {
                    await context.read<FragmentsProvider>().addRecovery(
                      description: _ctrl.text.trim(),
                      intensity: _intensity,
                      relatedFragmentIds: _related.toList(),
                    );
                    if (context.mounted) {
                      Navigator.of(context).pop(true);
                    }
                  },
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _UnderlineAction extends StatelessWidget {
  final String label;
  final bool enabled;
  final VoidCallback onTap;
  const _UnderlineAction({
    required this.label,
    required this.enabled,
    required this.onTap,
  });
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return GestureDetector(
      onTap: enabled ? onTap : null,
      behavior: HitTestBehavior.opaque,
      child: Opacity(
        opacity: enabled ? 1 : 0.35,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              label,
              style: theme.textTheme.bodyMedium?.copyWith(letterSpacing: 2),
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
    );
  }
}
