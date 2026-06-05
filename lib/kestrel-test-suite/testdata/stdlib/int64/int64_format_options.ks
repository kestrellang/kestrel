// test: execution
// stdlib: true

module Test

        func makeOpts(radix: std.numeric.Int64, uppercase: std.core.Bool, alternate: std.core.Bool) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.radix = radix;
            opts.uppercase = uppercase;
            opts.alternate = alternate;
            opts
        }

        func makeWidthOpts(width: std.numeric.Int64, alignment: std.text.Alignment, fill: std.text.Char) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.width = .Some(width);
            opts.alignment = alignment;
            opts.fill = fill;
            opts
        }

        func makeSignOpts(sign: std.text.Sign) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.sign = sign;
            opts
        }

        @main
        func main() -> lang.i64 {
            let val: std.numeric.Int64 = 255;

            // Default format (decimal)
            let dec = val.formatted();
            if dec.isEqual(to: "255") == false { return 1 }

            // Hexadecimal lowercase
            let hex = val.formatted(makeOpts(16, false, false));
            if hex.isEqual(to: "ff") == false { return 2 }

            // Hexadecimal uppercase
            let hexUp = val.formatted(makeOpts(16, true, false));
            if hexUp.isEqual(to: "FF") == false { return 3 }

            // Hex with alternate form (0x prefix)
            let hexAlt = val.formatted(makeOpts(16, false, true));
            if hexAlt.isEqual(to: "0xff") == false { return 4 }

            // Binary
            let fortyTwo: std.numeric.Int64 = 42;
            let bin = fortyTwo.formatted(makeOpts(2, false, false));
            if bin.isEqual(to: "101010") == false { return 5 }

            // Binary with alternate form (0b prefix)
            let binAlt = fortyTwo.formatted(makeOpts(2, false, true));
            if binAlt.isEqual(to: "0b101010") == false { return 6 }

            // Octal
            let oct = val.formatted(makeOpts(8, false, false));
            if oct.isEqual(to: "377") == false { return 7 }

            // Octal with alternate form (0o prefix)
            let octAlt = val.formatted(makeOpts(8, false, true));
            if octAlt.isEqual(to: "0o377") == false { return 8 }

            // Zero formats as "0"
            let zero: std.numeric.Int64 = 0;
            let zeroStr = zero.formatted();
            if zeroStr.isEqual(to: "0") == false { return 9 }

            // Negative value
            let neg: std.numeric.Int64 = -42;
            let negStr = neg.formatted();
            if negStr.isEqual(to: "-42") == false { return 10 }

            // Width and right alignment (default for numbers)
            let padded = fortyTwo.formatted(makeWidthOpts(6, std.text.Alignment.Right, ' '));
            if padded.isEqual(to: "    42") == false { return 11 }

            // Width with zero-fill and right alignment
            let zeroPad = fortyTwo.formatted(makeWidthOpts(6, std.text.Alignment.Right, '0'));
            if zeroPad.isEqual(to: "000042") == false { return 12 }

            // Width with left alignment
            let leftPad = fortyTwo.formatted(makeWidthOpts(6, std.text.Alignment.Left, ' '));
            if leftPad.isEqual(to: "42    ") == false { return 13 }

            // Sign always
            let signAlways = fortyTwo.formatted(makeSignOpts(std.text.Sign.Always));
            if signAlways.isEqual(to: "+42") == false { return 14 }

            // Sign space
            let signSpace = fortyTwo.formatted(makeSignOpts(std.text.Sign.Space));
            if signSpace.isEqual(to: " 42") == false { return 15 }

            // Negative with sign always still shows minus
            let negAlways = neg.formatted(makeSignOpts(std.text.Sign.Always));
            if negAlways.isEqual(to: "-42") == false { return 16 }

            0
        }
