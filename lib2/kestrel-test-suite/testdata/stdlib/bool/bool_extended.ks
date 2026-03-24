// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let t: std.core.Bool = true;
            let f: std.core.Bool = false;

            // Test matches
            if t.matches(t) == false { return 1 }
            if t.matches(f) { return 2 }
            if f.matches(f) == false { return 3 }
            if f.matches(t) { return 4 }

            // Test format - default formatting
            let trueStr = t.format();
            if trueStr != "true" { return 5 }

            let falseStr = f.format();
            if falseStr != "false" { return 6 }

            0
        }
