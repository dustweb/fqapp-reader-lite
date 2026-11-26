part of 'library_page.dart';

/// 书架按小说和听书筛选。
class _ShelfKindFilter extends StatelessWidget {
  const _ShelfKindFilter({required this.selected, required this.onChanged});

  final String? selected;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      const kinds = [(null, '全部'), ('book', '小说'), ('audio', '听书')];
      const gap = 6.0;
      const padding = 9.0;
      final labelWidth = MediaQuery.textScalerOf(context).scale(13) * 2;
      final minimum = (labelWidth + padding * 2).clamp(
        HomeChip.minTapTarget,
        double.infinity,
      );
      final available = constraints.maxWidth;
      final capacity = ((available + gap) / (minimum + gap)).floor().clamp(
        1,
        kinds.length,
      );
      final rows = (kinds.length / capacity).ceil();
      final columns = (kinds.length / rows).ceil();
      final width = ((available - gap * (columns - 1)) / columns)
          .floorToDouble()
          .clamp(0.0, rows == 1 ? minimum + 12 : available);
      return Wrap(
        spacing: gap,
        runSpacing: gap,
        children: [
          for (final (value, label) in kinds)
            SizedBox(
              width: width,
              child: HomeChip(
                key: Key('library-shelf-kind-${value ?? 'all'}'),
                label: label,
                horizontalPadding: padding,
                selected: selected == value,
                onTap: () => onChanged(value),
              ),
            ),
        ],
      );
    },
  );
}

/// 收藏 tab 的 video/manju 类型筛选行。
///
/// 类型筛选是 fqapp 专有行为（官方收藏页按类型分频道），Key 与文案保持
/// 不变；视觉换成上游 `GlassTextButton` 选中态。
class _CollectionTypeFilter extends StatelessWidget {
  const _CollectionTypeFilter({
    required this.selected,
    required this.onChanged,
  });

  final String? selected;
  final ValueChanged<String?> onChanged;

  static const _options = <({String key, String label, String? value})>[
    (key: 'all', label: '全部', value: null),
    (key: 'video', label: '短剧', value: 'video'),
    (key: 'manju', label: '漫剧', value: 'manju'),
  ];

  @override
  Widget build(BuildContext context) => Row(
    children: [
      for (final option in _options)
        Padding(
          padding: const EdgeInsets.only(right: 8),
          child: Semantics(
            key: Key('library-collection-filter-${option.key}'),
            selected: selected == option.value,
            inMutuallyExclusiveGroup: true,
            child: GlassTextButton(
              compact: true,
              selected: selected == option.value,
              onPressed: () => onChanged(option.value),
              child: AppText(option.label),
            ),
          ),
        ),
    ],
  );
}
