// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let t: std.core.Bool = true;
            let f: std.core.Bool = false;

            // Test matches (`matches` is sugar-only: reached through patterns)
            match t { true => {}, _ => { return 1 } }
            match t { false => { return 2 }, _ => {} }
            match f { false => {}, _ => { return 3 } }
            match f { true => { return 4 }, _ => {} }

            // Test format - default formatting
            let trueStr = t.formatted();
            if trueStr != "true" { return 5 }

            let falseStr = f.formatted();
            if falseStr != "false" { return 6 }

            0
        }
