// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // removeSubrange with RangeFrom: remove from index 3 onward
            var a = [10, 20, 30, 40, 50];
            a.removeSubrange(3..);
            if a.count != 3 { return 1 }
            if a(0) != 10 { return 2 }
            if a(2) != 30 { return 3 }

            // removeSubrange with RangeUpTo: remove first 2
            var b = [10, 20, 30, 40, 50];
            b.removeSubrange(..<2);
            if b.count != 3 { return 4 }
            if b(0) != 30 { return 5 }

            // removeSubrange with RangeThrough: remove through index 2
            var c = [10, 20, 30, 40, 50];
            c.removeSubrange(..=2);
            if c.count != 2 { return 6 }
            if c(0) != 40 { return 7 }

            // replaceSubrange with RangeFrom: replace tail
            var d = [10, 20, 30, 40, 50];
            d.replaceSubrange(3.., with: [99]);
            if d.count != 4 { return 8 }
            if d(3) != 99 { return 9 }

            // replaceSubrange with RangeUpTo: replace head
            var e = [10, 20, 30, 40, 50];
            e.replaceSubrange(..<2, with: [1, 2, 3]);
            if e.count != 6 { return 10 }
            if e(0) != 1 { return 11 }
            if e(2) != 3 { return 12 }
            if e(3) != 30 { return 13 }

            // existing Range still works
            var f = [10, 20, 30, 40, 50];
            f.removeSubrange(1..<3);
            if f.count != 3 { return 14 }
            if f(1) != 40 { return 15 }

            0
        }
