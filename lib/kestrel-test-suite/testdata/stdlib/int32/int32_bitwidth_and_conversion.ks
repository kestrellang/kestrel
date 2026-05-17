// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // byteSwapped — 4-byte swap
            // 0x00000001 = 1 -> byte swapped = 0x01000000 = 16777216
            let one: std.numeric.Int32 = 1;
            let oneSwapped = one.byteSwapped;
            let oneSwappedI64 = std.numeric.Int64(from: oneSwapped);
            if oneSwappedI64 != 16777216 { return 1 }

            // byteSwapped — 0x01020304 = 16909060 -> 0x04030201 = 67305985
            let val: std.numeric.Int32 = 16909060;
            let swapped = val.byteSwapped;
            let swappedI64 = std.numeric.Int64(from: swapped);
            if swappedI64 != 67305985 { return 2 }

            // leadingZeros — relative to 32-bit width
            if one.leadingZeros != 31 { return 3 }
            let zero: std.numeric.Int32 = 0;
            if zero.leadingZeros != 32 { return 4 }
            // -1 in Int32 is all 1s, so 0 leading zeros
            let negOne: std.numeric.Int32 = -1;
            if negOne.leadingZeros != 0 { return 5 }
            // 65536 = 2^16, so 15 leading zeros (bit 16 is set, bits 17-31 are 0)
            let val65536: std.numeric.Int32 = 65536;
            if val65536.leadingZeros != 15 { return 6 }

            // rotateLeft — 32-bit rotation
            let expectedRotateLeftOne: std.numeric.Int32 = 2;
            if one.rotateLeft(by: 1) != expectedRotateLeftOne { return 7 }
            let fortyTwo: std.numeric.Int32 = 42;
            if fortyTwo.rotateLeft(by: 0) != fortyTwo { return 8 }

            // rotateRight — 32-bit rotation
            let two: std.numeric.Int32 = 2;
            if two.rotateRight(by: 1) != one { return 9 }
            if fortyTwo.rotateRight(by: 0) != fortyTwo { return 10 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.numeric.Int32 = 123456;
            if testVal.rotateLeft(by: 11).rotateRight(by: 11) != testVal { return 11 }

            // init(from:) — from Int64
            let i64val: std.numeric.Int64 = 100000;
            let fromI64 = std.numeric.Int32(from: i64val);
            let expectedFromI64: std.numeric.Int32 = 100000;
            if fromI64 != expectedFromI64 { return 12 }

            // init(from:) — from Int16
            let i16val: std.numeric.Int16 = -500;
            let fromI16 = std.numeric.Int32(from: i16val);
            let expectedFromI16: std.numeric.Int32 = -500;
            if fromI16 != expectedFromI16 { return 13 }

            // init(from:) — from Int8
            let i8val: std.numeric.Int8 = -100;
            let fromI8 = std.numeric.Int32(from: i8val);
            let expectedFromI8: std.numeric.Int32 = -100;
            if fromI8 != expectedFromI8 { return 14 }

            // parse — valid Int32 value
            let parsed = std.numeric.Int32.parse( "100000");
            if parsed.isNone() { return 15 }
            let expectedParsed: std.numeric.Int32 = 100000;
            if parsed.unwrap() != expectedParsed { return 16 }

            // parse — negative value
            let parsedNeg = std.numeric.Int32.parse( "-2147483648");
            if parsedNeg.isNone() { return 17 }
            if parsedNeg.unwrap() != std.numeric.Int32.minValue { return 18 }

            // parse — maxValue
            let parsedMax = std.numeric.Int32.parse( "2147483647");
            if parsedMax.isNone() { return 19 }
            if parsedMax.unwrap() != std.numeric.Int32.maxValue { return 20 }

            // parse — out-of-range (too large)
            let parsedBig = std.numeric.Int32.parse( "2147483648");
            if parsedBig.isSome() { return 21 }

            // parse — out-of-range (too small)
            let parsedSmall = std.numeric.Int32.parse( "-2147483649");
            if parsedSmall.isSome() { return 22 }

            // parse — invalid string
            let parsedBad = std.numeric.Int32.parse( "xyz");
            if parsedBad.isSome() { return 23 }

            // parse — empty string
            let parsedEmpty = std.numeric.Int32.parse( "");
            if parsedEmpty.isSome() { return 24 }

            0
        }
