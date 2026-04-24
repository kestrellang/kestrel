// test: execution
// stdlib: true

module Test

        func makeOpts(radix: std.num.Int64, uppercase: std.core.Bool, alternate: std.core.Bool) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.radix = radix;
            opts.uppercase = uppercase;
            opts.alternate = alternate;
            opts
        }

        func makeWidthOpts(width: std.num.Int64, alignment: std.text.Alignment, fill: std.text.Char) -> std.text.FormatOptions {
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

        func main() -> lang.i64 {
            let val: std.num.Int64 = 255;

            // Default format (decimal)
            let dec = val.format();
            if dec.equals("255") == false { return 1 }

            // Hexadecimal lowercase
            let hex = val.format(makeOpts(16, false, false));
            if hex.equals("ff") == false { return 2 }

            // Hexadecimal uppercase
            let hexUp = val.format(makeOpts(16, true, false));
            if hexUp.equals("FF") == false { return 3 }

            // Hex with alternate form (0x prefix)
            let hexAlt = val.format(makeOpts(16, false, true));
            if hexAlt.equals("0xff") == false { return 4 }

            // Binary
            let fortyTwo: std.num.Int64 = 42;
            let bin = fortyTwo.format(makeOpts(2, false, false));
            if bin.equals("101010") == false { return 5 }

            // Binary with alternate form (0b prefix)
            let binAlt = fortyTwo.format(makeOpts(2, false, true));
            if binAlt.equals("0b101010") == false { return 6 }

            // Octal
            let oct = val.format(makeOpts(8, false, false));
            if oct.equals("377") == false { return 7 }

            // Octal with alternate form (0o prefix)
            let octAlt = val.format(makeOpts(8, false, true));
            if octAlt.equals("0o377") == false { return 8 }

            // Zero formats as "0"
            let zero: std.num.Int64 = 0;
            let zeroStr = zero.format();
            if zeroStr.equals("0") == false { return 9 }

            // Negative value
            let neg: std.num.Int64 = -42;
            let negStr = neg.format();
            if negStr.equals("-42") == false { return 10 }

            // Width and right alignment (default for numbers)
            let padded = fortyTwo.format(makeWidthOpts(6, std.text.Alignment.Right, ' '));
            if padded.equals("    42") == false { return 11 }

            // Width with zero-fill and right alignment
            let zeroPad = fortyTwo.format(makeWidthOpts(6, std.text.Alignment.Right, '0'));
            if zeroPad.equals("000042") == false { return 12 }

            // Width with left alignment
            let leftPad = fortyTwo.format(makeWidthOpts(6, std.text.Alignment.Left, ' '));
            if leftPad.equals("42    ") == false { return 13 }

            // Sign always
            let signAlways = fortyTwo.format(makeSignOpts(std.text.Sign.Always));
            if signAlways.equals("+42") == false { return 14 }

            // Sign space
            let signSpace = fortyTwo.format(makeSignOpts(std.text.Sign.Space));
            if signSpace.equals(" 42") == false { return 15 }

            // Negative with sign always still shows minus
            let negAlways = neg.format(makeSignOpts(std.text.Sign.Always));
            if negAlways.equals("-42") == false { return 16 }

            0
        }
