// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- chars(Range) - default subscript with range ----
            let sub = s.chars(std.core.Range[std.num.Int64](0, 5));
            if sub.equals("hello") == false { return 1 }

            let sub2 = s.chars(std.core.Range[std.num.Int64](6, 11));
            if sub2.equals("world") == false { return 2 }

            // ---- chars(checked: Range) ----
            let checked = s.chars(checked: std.core.Range[std.num.Int64](0, 5));
            if checked.isNone() { return 3 }
            if checked.unwrap().equals("hello") == false { return 4 }

            // Out of bounds returns None
            let oob = s.chars(checked: std.core.Range[std.num.Int64](0, 100));
            if oob.isSome() { return 5 }

            // Empty range
            let empty = s.chars(checked: std.core.Range[std.num.Int64](3, 3));
            if empty.isNone() { return 6 }
            if empty.unwrap().isEmpty == false { return 7 }

            0
        }
