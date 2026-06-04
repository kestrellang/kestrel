// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // ---- half-open Range over LF-only buffer ----
            let s: std.text.String = "alpha\nbeta\ngamma";
            let sub = s.lines(std.core.Range[std.numeric.Int64](0, 2));
            if sub.count != 2 { return 1 }
            // Sub-view re-iterates as the same lines
            var it = sub.iter();
            if it.next().unwrap().isEqual(to: "alpha") == false { return 2 }
            if it.next().unwrap().isEqual(to: "beta") == false { return 3 }
            if it.next().isSome() { return 4 }
            // Underlying byte range includes the terminator after "beta"
            if sub.toString().isEqual(to: "alpha\nbeta\n") == false { return 5 }

            // ---- ClosedRange matches half-open with +1 end ----
            let subClosed = s.lines(std.core.ClosedRange[std.numeric.Int64](0, 1));
            if subClosed.toString().isEqual(to: "alpha\nbeta\n") == false { return 6 }

            // ---- whole buffer round-trip ----
            let whole = s.lines(std.core.Range[std.numeric.Int64](0, 3));
            if whole.toString().isEqual(to: "alpha\nbeta\ngamma") == false { return 7 }

            // ---- CRLF preservation ----
            let crlf: std.text.String = "a\r\nb\r\nc";
            let crlfSub = crlf.lines(std.core.Range[std.numeric.Int64](0, 1));
            if crlfSub.toString().isEqual(to: "a\r\n") == false { return 8 }

            // ---- lone \r preservation ----
            let cr: std.text.String = "a\rb\rc";
            let crSub = cr.lines(std.core.Range[std.numeric.Int64](1, 3));
            if crSub.count != 2 { return 9 }
            if crSub.toString().isEqual(to: "b\rc") == false { return 10 }

            // ---- trailing line without terminator ----
            let trail: std.text.String = "a\nb";
            if trail.lines.count != 2 { return 11 }
            let trailHead = trail.lines(std.core.Range[std.numeric.Int64](0, 1));
            if trailHead.toString().isEqual(to: "a\n") == false { return 12 }
            let trailTail = trail.lines(std.core.Range[std.numeric.Int64](1, 2));
            if trailTail.toString().isEqual(to: "b") == false { return 13 }

            // ---- empty range ----
            let emptyRange = s.lines(std.core.Range[std.numeric.Int64](0, 0));
            if emptyRange.count != 0 { return 14 }
            if emptyRange.toString().isEmpty == false { return 15 }

            // ---- one-past-end as endpoint ----
            let last = s.lines(std.core.Range[std.numeric.Int64](2, 3));
            if last.toString().isEqual(to: "gamma") == false { return 16 }

            // ---- past-end: checked returns None ----
            let oob = s.lines(checked: std.core.Range[std.numeric.Int64](0, 999));
            if oob.isSome() { return 17 }

            // ---- past-end: clamped saturates to count ----
            let clamp = s.lines(clamped: std.core.Range[std.numeric.Int64](-5, 999));
            if clamp.toString().isEqual(to: "alpha\nbeta\ngamma") == false { return 18 }

            // ---- negative start, checked ----
            let negCheck = s.lines(checked: std.core.Range[std.numeric.Int64](-1, 2));
            if negCheck.isSome() { return 19 }

            // ---- mixed terminators round-trip ----
            let mixed: std.text.String = "a\nb\r\nc\rd";
            let midSlice = mixed.lines(std.core.Range[std.numeric.Int64](1, 3));
            if midSlice.count != 2 { return 20 }
            // bytes covered: from start of "b" (offset 2) to start of "d" (offset 7)
            if midSlice.toString().isEqual(to: "b\r\nc\r") == false { return 21 }

            // ---- empty source ----
            let emptyStr = std.text.String();
            let emptyLines = emptyStr.lines(std.core.Range[std.numeric.Int64](0, 0));
            if emptyLines.count != 0 { return 22 }

            // ---- lines.substring (Range and ClosedRange) ----
            if s.lines.substring(std.core.Range[std.numeric.Int64](0, 2)).isEqual(to: "alpha\nbeta\n") == false { return 23 }
            if s.lines.substring(std.core.ClosedRange[std.numeric.Int64](0, 1)).isEqual(to: "alpha\nbeta\n") == false { return 24 }

            0
        }
