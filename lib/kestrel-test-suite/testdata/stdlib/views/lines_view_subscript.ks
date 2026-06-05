// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let s: std.text.String = "alpha\nbeta\ngamma";

            // ---- count ----
            if s.lines.count != 3 { return 1 }

            // ---- view(i) ----
            let l0 = s.lines(0);
            if l0.isEqual(to: "alpha") == false { return 2 }
            let l1 = s.lines(1);
            if l1.isEqual(to: "beta") == false { return 3 }
            let l2 = s.lines(2);
            if l2.isEqual(to: "gamma") == false { return 4 }

            // ---- view(checked: i) ----
            let lc = s.lines(checked: 1);
            if lc.isNone() { return 5 }
            if lc.unwrap().isEqual(to: "beta") == false { return 6 }

            let lOob = s.lines(checked: 100);
            if lOob.isSome() { return 7 }

            let lNeg = s.lines(checked: -1);
            if lNeg.isSome() { return 8 }

            // ---- view(clamped: i) ----
            let lcl = s.lines(clamped: 1);
            if lcl.unwrap().isEqual(to: "beta") == false { return 9 }

            let lNegClamp = s.lines(clamped: -10);
            if lNegClamp.unwrap().isEqual(to: "alpha") == false { return 10 }

            let lOverClamp = s.lines(clamped: 100);
            if lOverClamp.unwrap().isEqual(to: "gamma") == false { return 11 }

            // ---- empty view returns None on clamp ----
            let empty = std.text.String();
            let emptyClamp = empty.lines(clamped: 0);
            if emptyClamp.isSome() { return 12 }

            // ---- mixed terminators ----
            let mixed: std.text.String = "a\nb\r\nc\rd";
            if mixed.lines.count != 4 { return 13 }
            if mixed.lines(0).isEqual(to: "a") == false { return 14 }
            if mixed.lines(1).isEqual(to: "b") == false { return 15 }
            if mixed.lines(2).isEqual(to: "c") == false { return 16 }
            if mixed.lines(3).isEqual(to: "d") == false { return 17 }

            0
        }
