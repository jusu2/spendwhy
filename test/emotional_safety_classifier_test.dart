import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/services/emotional_safety_classifier.dart';

void main() {
  test('empty text -> none', () {
    expect(SafetyClassifier.classify('').level, SafetyLevel.none);
    expect(SafetyClassifier.classify('   ').level, SafetyLevel.none);
  });

  test('neutral text -> none', () {
    expect(SafetyClassifier.classify('今天和朋友吃了顿饭，还行').level, SafetyLevel.none);
  });

  test('gentle phrases trigger gentle level', () {
    final s = SafetyClassifier.classify('最近真的太累了，停不下来');
    expect(s.level, SafetyLevel.gentle);
    expect(s.matched, isNotEmpty);
  });

  test('elevated phrases trigger elevated level', () {
    final s = SafetyClassifier.classify('我有时候真的不想活了');
    expect(s.level, SafetyLevel.elevated);
  });

  test('elevated wins over gentle', () {
    final s = SafetyClassifier.classify('太累了，撑不住，想死');
    expect(s.level, SafetyLevel.elevated);
  });
}
