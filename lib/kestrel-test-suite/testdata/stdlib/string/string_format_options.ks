// test: execution
// stdlib: true

module Test

        func makeOpts(width: std.numeric.Int64, alignment: std.text.Alignment, fill: std.text.Char) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.width = .Some(width);
            opts.alignment = alignment;
            opts.fill = fill;
            opts
        }

        @main
        func main() -> lang.i64 {
            let s: std.text.String = "test";

            // format() with no options returns the string itself
            let plain = s.formatted();
            if plain.isEqual(to: "test") == false { return 1 }

            // format with width and left alignment
            let leftPadded = s.formatted(makeOpts(10, std.text.Alignment.Left, ' '));
            if leftPadded.isEqual(to: "test      ") == false { return 2 }
            if leftPadded.chars.count != 10 { return 3 }

            // format with width and right alignment
            let rightPadded = s.formatted(makeOpts(10, std.text.Alignment.Right, ' '));
            if rightPadded.isEqual(to: "      test") == false { return 4 }
            if rightPadded.chars.count != 10 { return 5 }

            // format with width and center alignment
            let centerPadded = s.formatted(makeOpts(10, std.text.Alignment.Center, ' '));
            if centerPadded.isEqual(to: "   test   ") == false { return 6 }
            if centerPadded.chars.count != 10 { return 7 }

            // format when string is already wider than width
            let noChange = s.formatted(makeOpts(2, std.text.Alignment.Left, ' '));
            if noChange.isEqual(to: "test") == false { return 8 }

            // format with custom fill character
            let customFill = s.formatted(makeOpts(8, std.text.Alignment.Right, '-'));
            if customFill.isEqual(to: "----test") == false { return 9 }

            0
        }
