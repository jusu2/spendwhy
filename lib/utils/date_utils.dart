import 'package:intl/intl.dart';

import '../i18n/strings.dart';

class DateText {
  DateText._();

  static String relative(DateTime then, {DateTime? now}) {
    final n = now ?? DateTime.now();
    final diff = n.difference(then);
    if (diff.inMinutes < 1) return S.justNow;
    if (diff.inMinutes < 60) return '${diff.inMinutes} ${S.minutesAgo}';
    if (diff.inHours < 24) return '${diff.inHours} ${S.hoursAgo}';
    if (diff.inDays < 30) return '${diff.inDays} ${S.daysAgo}';
    return DateFormat('yyyy.MM.dd').format(then);
  }

  static String monthDay(DateTime t) => DateFormat('M月d日').format(t);
  static String yearMonth(DateTime t) => DateFormat('yyyy.MM').format(t);
  static String full(DateTime t) => DateFormat('yyyy.MM.dd HH:mm').format(t);
}
