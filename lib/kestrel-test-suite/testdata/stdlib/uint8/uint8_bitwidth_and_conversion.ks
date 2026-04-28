// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // byteSwapped — identity for single-byte type
            let val: std.numeric.UInt8 = 42;
            if val.byteSwapped.equals(val) == false { return 1 }
            let maxVal = std.numeric.UInt8.maxValue;
            if maxVal.byteSwapped.equals(maxVal) == false { return 2 }

            // leadingZeros — relative to 8-bit width
            let one: std.numeric.UInt8 = 1;
            if one.leadingZeros != 7 { return 3 }

            let zero: std.numeric.UInt8 = 0;
            if zero.leadingZeros != 8 { return 4 }

            let v128: std.numeric.UInt8 = 128;
            if v128.leadingZeros != 0 { return 5 }

            // rotateLeft — 8-bit rotation
            // rotateLeft(1, by: 1) = 2
            let rotTwo: std.numeric.UInt8 = 2;
            if one.rotateLeft(by: 1).equals(rotTwo) == false { return 6 }
            // rotateLeft(128, by: 1) = 1 (wraps around)
            if v128.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 8-bit rotation
            // rotateRight(2, by: 1) = 1
            let two: std.numeric.UInt8 = 2;
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = 128 (wraps around)
            if one.rotateRight(by: 1).equals(v128) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.numeric.UInt8 = 42;
            if testVal.rotateLeft(by: 3).rotateRight(by: 3).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64Val: std.numeric.Int64 = 200;
            let fromI64 = std.numeric.UInt8(from: fromI64Val);
            let lit200: std.numeric.UInt8 = 200;
            if fromI64.equals(lit200) == false { return 11 }

            // init(from:) — from UInt64
            let fromU64Val: std.numeric.UInt64 = 100;
            let fromU64 = std.numeric.UInt8(from: fromU64Val);
            let lit100: std.numeric.UInt8 = 100;
            if fromU64.equals(lit100) == false { return 12 }

            // parse — valid value
            let parsed = std.numeric.UInt8.parse( "255");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.numeric.UInt8.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.numeric.UInt8.parse( "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.numeric.UInt8.zero) == false { return 16 }

            // parse — out of range (256 > 255)
            let parsedOver = std.numeric.UInt8.parse( "256");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.numeric.UInt8.parse( "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.numeric.UInt8.parse( "");
            if parsedEmpty.isSome() { return 19 }

            // parse — non-numeric
            let parsedBad = std.numeric.UInt8.parse( "abc");
            if parsedBad.isSome() { return 20 }

            0
        }
