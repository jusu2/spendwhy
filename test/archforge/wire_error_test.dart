import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/archforge/archforge.dart';

void main() {
  group('WireErrorKind.fromWire', () {
    test('maps every known wire value', () {
      for (final kind in WireErrorKind.values) {
        // unknown 的 wire value 也是 'unknown'; 验证 round-trip 一致。
        expect(WireErrorKind.fromWire(kind.wireValue), kind);
      }
    });

    test('returns unknown for null', () {
      expect(WireErrorKind.fromWire(null), WireErrorKind.unknown);
    });

    test('returns unknown for unrecognised string', () {
      expect(
        WireErrorKind.fromWire('future_kind_we_havent_invented'),
        WireErrorKind.unknown,
      );
    });
  });

  group('WireError.fromJson', () {
    test('parses a complete payload', () {
      final wire = WireError.fromJson({
        'kind': 'conflict',
        'message': 'dup email',
        'is_panic': false,
      });
      expect(wire.kind, WireErrorKind.conflict);
      expect(wire.message, 'dup email');
      expect(wire.isPanic, isFalse);
      expect(wire.rawKind, isNull);
    });

    test('keeps raw kind when unknown', () {
      final wire = WireError.fromJson({
        'kind': 'future_variant',
        'message': 'something',
      });
      expect(wire.kind, WireErrorKind.unknown);
      expect(wire.rawKind, 'future_variant');
    });

    test('defaults is_panic to false when missing', () {
      final wire = WireError.fromJson({
        'kind': 'internal',
        'message': 'whatever',
      });
      expect(wire.isPanic, isFalse);
    });

    test('flags is_panic from Rust-side panic marker', () {
      final wire = WireError.fromJson({
        'kind': 'internal',
        'message': 'panic: boom in worker',
        'is_panic': true,
      });
      expect(wire.kind, WireErrorKind.internal);
      expect(wire.isPanic, isTrue);
      expect(wire.shouldReportAsCrash, isTrue);
    });

    test('never throws on missing fields', () {
      // Empty map: must still produce a usable error.
      final empty = WireError.fromJson({});
      expect(empty.kind, WireErrorKind.unknown);
      expect(empty.message, '');
      expect(empty.isPanic, isFalse);
    });

    test('never throws on wrong types', () {
      // Wire data has been corrupted; we should still degrade gracefully.
      final junk = WireError.fromJson({
        'kind': 42,
        'message': null,
        'is_panic': 'yes',
      });
      expect(junk.kind, WireErrorKind.unknown);
      expect(junk.message, '');
      expect(junk.isPanic, isFalse);
    });
  });

  group('WireError semantics', () {
    test('isRetriable is true only for unavailable / deadline', () {
      const retriable = {
        WireErrorKind.unavailable,
        WireErrorKind.deadlineExceeded,
      };
      for (final kind in WireErrorKind.values) {
        final wire = WireError(kind: kind, message: '');
        expect(
          wire.isRetriable,
          retriable.contains(kind),
          reason: 'isRetriable mismatch for $kind',
        );
      }
    });

    test('isUserInput is true only for invalid', () {
      for (final kind in WireErrorKind.values) {
        final wire = WireError(kind: kind, message: '');
        expect(wire.isUserInput, kind == WireErrorKind.invalid);
      }
    });

    test('shouldReportAsCrash mirrors isPanic, regardless of kind', () {
      final crash = WireError(
        kind: WireErrorKind.internal,
        message: 'boom',
        isPanic: true,
      );
      expect(crash.shouldReportAsCrash, isTrue);

      final business = WireError(
        kind: WireErrorKind.internal,
        message: 'dbpool drained',
      );
      expect(business.shouldReportAsCrash, isFalse);
    });

    test('toString includes panic marker when flagged', () {
      final s = WireError(
        kind: WireErrorKind.internal,
        message: 'boom',
        isPanic: true,
      ).toString();
      expect(s, contains('internal|panic'));
      expect(s, contains('boom'));
    });

    test('equality + hashCode follow value semantics', () {
      const a = WireError(kind: WireErrorKind.notFound, message: 'x');
      const b = WireError(kind: WireErrorKind.notFound, message: 'x');
      const c = WireError(kind: WireErrorKind.conflict, message: 'x');
      expect(a, b);
      expect(a.hashCode, b.hashCode);
      expect(a == c, isFalse);
    });
  });

  group('WireException', () {
    test('exposes the underlying wire fields', () {
      final wire = WireError(
        kind: WireErrorKind.forbidden,
        message: 'no token',
      );
      final ex = WireException(wire);
      expect(ex.kind, WireErrorKind.forbidden);
      expect(ex.message, 'no token');
      expect(ex.isPanic, isFalse);
      expect(ex.isRetriable, isFalse);
      expect(ex.toString(), contains('forbidden'));
    });

    test('is throwable and catchable as Exception', () {
      WireException? caught;
      try {
        throw WireException(
          WireError(kind: WireErrorKind.unavailable, message: 'down'),
        );
      } on WireException catch (e) {
        caught = e;
      }
      expect(caught, isNotNull);
      expect(caught.kind, WireErrorKind.unavailable);
      expect(caught.isRetriable, isTrue);
    });
  });
}
