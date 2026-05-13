import 'package:flutter/material.dart';

import '../i18n/strings.dart';
import '../widgets/underline_button.dart';

/// 心理援助资源页：温和、克制，不催促"快去打电话"。
class SafetyResourcesPage extends StatelessWidget {
  const SafetyResourcesPage({super.key});

  static const _resources = [
    _Resource(name: '北京心理危机研究与干预中心', detail: '010-82951332', hint: '24 小时'),
    _Resource(name: '希望24热线', detail: '400-161-9995', hint: '24 小时'),
    _Resource(name: '上海市心理援助热线', detail: '021-12320-5', hint: '24 小时'),
    _Resource(name: '北京大学第六医院', detail: '010-82801950', hint: '专业精神卫生机构'),
  ];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(
        leadingWidth: 64,
        leading: Padding(
          padding: const EdgeInsets.only(left: 24),
          child: IconButton(
            icon: const Icon(Icons.arrow_back, size: 18),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ),
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(28, 8, 28, 40),
          children: [
            Text(
              S.safetyResourceTitle.toUpperCase(),
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 18),
            Text(
              S.safetyResourceIntro,
              style: theme.textTheme.bodyLarge?.copyWith(height: 1.9),
            ),
            const SizedBox(height: 24),
            Container(
              height: 0.6,
              width: 24,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.45),
            ),
            const SizedBox(height: 24),
            for (final r in _resources) ...[
              _ResourceTile(resource: r),
              Divider(color: theme.dividerTheme.color, height: 32),
            ],
            const SizedBox(height: 16),
            Text(
              S.safetyResourceFooter,
              style: theme.textTheme.bodySmall?.copyWith(
                fontStyle: FontStyle.italic,
                height: 1.9,
              ),
            ),
            const SizedBox(height: 32),
            Align(
              alignment: Alignment.centerRight,
              child: UnderlineButton(
                label: S.back,
                onTap: () => Navigator.of(context).pop(),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _Resource {
  final String name;
  final String detail;
  final String hint;
  const _Resource({
    required this.name,
    required this.detail,
    required this.hint,
  });
}

class _ResourceTile extends StatelessWidget {
  final _Resource resource;
  const _ResourceTile({required this.resource});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(resource.name, style: theme.textTheme.bodyLarge),
        const SizedBox(height: 6),
        Text(
          resource.detail,
          style: theme.textTheme.headlineMedium?.copyWith(letterSpacing: 2),
        ),
        const SizedBox(height: 4),
        Text(
          resource.hint,
          style: theme.textTheme.bodySmall?.copyWith(
            fontStyle: FontStyle.italic,
            color: theme.colorScheme.onSurface.withValues(alpha: 0.55),
          ),
        ),
      ],
    );
  }
}
