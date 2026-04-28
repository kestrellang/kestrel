// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // byteSwapped — 2-byte swap
            // 0x0102 = 258 -> byte swapped = 0x0201 = 513
            let val: std.numeric.Int16 = 258;
            let swapped = val.byteSwapped;
            let swappedAsI64 = std.numeric.Int64(from: swapped);
            if swappedAsI64 != 513 { return 1 }

            // byteSwapped — 1 in Int16 is 0x0001, swapped is 0x0100 = 256
            let one: std.numeric.Int16 = 1;
            let oneSwapped = one.byteSwapped;
            let oneSwappedI64 = std.numeric.Int64(from: oneSwapped);
            if oneSwappedI64 != 256 { return 2 }

            // leadingZeros — relative to 16-bit width
            if one.leadingZeros != 15 { return 3 }
            let zero: std.numeric.Int16 = 0;
            if zero.leadingZeros != 16 { return 4 }
            // -1 in Int16 is all 1s, so 0 leading zeros
            let negOne: std.numeric.Int16 = -1;
            if negOne.leadingZeros != 0 { return 5 }
            // 256 = 0x0100 = 0b0000000100000000 -> 7 leading zeros
            let val256: std.numeric.Int16 = 256;
            if val256.leadingZeros != 7 { return 6 }

            // rotateLeft — 16-bit rotation
            let expectedRotateLeft: std.numeric.Int16 = 2;
            if one.rotateLeft(by: 1) != expectedRotateLeft { return 7 }
            let fortyTwo: std.numeric.Int16 = 42;
            if fortyTwo.rotateLeft(by: 0) != fortyTwo { return 8 }

            // rotateRight — 16-bit rotation
            let two: std.numeric.Int16 = 2;
            if two.rotateRight(by: 1) != one { return 9 }
            if fortyTwo.rotateRight(by: 0) != fortyTwo { return 10 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.numeric.Int16 = 12345;
            if testVal.rotateLeft(by: 5).rotateRight(by: 5) != testVal { return 11 }

            // init(from:) — from Int64
            let i64val: std.numeric.Int64 = 1000;
            let fromI64 = std.numeric.Int16(from: i64val);
            let expectedFromI64: std.numeric.Int16 = 1000;
            if fromI64 != expectedFromI64 { return 12 }

            // init(from:) — from Int8
            let i8val: std.numeric.Int8 = -50;
            let fromI8 = std.numeric.Int16(from: i8val);
            let expectedFromI8: std.numeric.Int16 = -50;
            if fromI8 != expectedFromI8 { return 13 }

            // parse — valid Int16 value
            let parsed = std.numeric.Int16.parse( "1000");
            if parsed.isNone() { return 14 }
            let expectedParsed: std.numeric.Int16 = 1000;
            if parsed.unwrap() != expectedParsed { return 15 }

            // parse — negative value
            let parsedNeg = std.numeric.Int16.parse( "-32768");
            if parsedNeg.isNone() { return 16 }
            if parsedNeg.unwrap() != std.numeric.Int16.minValue { return 17 }

            // parse — maxValue
            let parsedMax = std.numeric.Int16.parse( "32767");
            if parsedMax.isNone() { return 18 }
            if parsedMax.unwrap() != std.numeric.Int16.maxValue { return 19 }

            // parse — out-of-range (too large)
            let parsedBig = std.numeric.Int16.parse( "32768");
            if parsedBig.isSome() { return 20 }

            // parse — out-of-range (too small)
            let parsedSmall = std.numeric.Int16.parse( "-32769");
            if parsedSmall.isSome() { return 21 }

            // parse — invalid string
            let parsedBad = std.numeric.Int16.parse( "xyz");
            if parsedBad.isSome() { return 22 }

            // parse — empty string
            let parsedEmpty = std.numeric.Int16.parse( "");
            if parsedEmpty.isSome() { return 23 }

            0
        }
