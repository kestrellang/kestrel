// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- bytes.count ----
            if s.bytes.count != 5 { return 1 }

            // ---- bytes.isEmpty() ----
            if s.bytes.isEmpty() { return 2 }

            // Empty string bytes view
            let empty = std.text.String();
            if empty.bytes.isEmpty() == false { return 3 }
            if empty.bytes.count != 0 { return 4 }

            // ---- bytes.byteAt() ----
            // 'h' = 104
            let b0 = s.bytes.byteAt(0);
            if b0.isNone() { return 5 }
            let byteH: std.num.UInt8 = 104;
            if b0.unwrap() != byteH { return 6 }

            // 'e' = 101
            let b1 = s.bytes.byteAt(1);
            if b1.isNone() { return 7 }
            let byteE: std.num.UInt8 = 101;
            if b1.unwrap() != byteE { return 8 }

            // Out of bounds returns None
            let bOob = s.bytes.byteAt(100);
            if bOob.isSome() { return 9 }

            // Negative index returns None
            let bNeg = s.bytes.byteAt(-1);
            if bNeg.isSome() { return 10 }

            // ---- bytes.byteAtUnchecked() ----
            // 'o' = 111
            let bu = s.bytes.byteAtUnchecked(4);
            let byteO: std.num.UInt8 = 111;
            if bu != byteO { return 11 }

            0
        }
