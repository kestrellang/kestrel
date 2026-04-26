// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // byteSwapped — 8-byte swap
            // 1 as u64 (0x0000000000000001) -> 0x0100000000000000 (72057594037927936)
            let one: std.num.UInt64 = 1;
            let lit72057594037927936: std.num.UInt64 = 72057594037927936;
            if one.byteSwapped.equals(lit72057594037927936) == false { return 1 }

            // byteSwapped — double swap is identity
            let val: std.num.UInt64 = 123456789;
            if val.byteSwapped.byteSwapped.equals(val) == false { return 2 }

            // leadingZeros — relative to 64-bit width
            if one.leadingZeros != 63 { return 3 }

            let zero: std.num.UInt64 = 0;
            if zero.leadingZeros != 64 { return 4 }

            // Value with high bit set: 2^63 = 9223372036854775808
            let highBit: std.num.UInt64 = 9223372036854775808;
            if highBit.leadingZeros != 0 { return 5 }

            // rotateLeft — 64-bit rotation
            // rotateLeft(1, by: 1) = 2
            let rotTwo: std.num.UInt64 = 2;
            if one.rotateLeft(by: 1).equals(rotTwo) == false { return 6 }
            // rotateLeft(highBit, by: 1) = 1 (wraps around from bit 63 to bit 0)
            if highBit.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 64-bit rotation
            // rotateRight(2, by: 1) = 1
            let two: std.num.UInt64 = 2;
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = highBit (wraps from bit 0 to bit 63)
            if one.rotateRight(by: 1).equals(highBit) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.num.UInt64 = 123456789;
            if testVal.rotateLeft(by: 17).rotateRight(by: 17).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64Val: std.num.Int64 = 1000000;
            let fromI64 = std.num.UInt64(from: fromI64Val);
            let lit1000000: std.num.UInt64 = 1000000;
            if fromI64.equals(lit1000000) == false { return 11 }

            // init(from:) — from UInt8
            let fromU8Val: std.num.UInt8 = 255;
            let fromU8 = std.num.UInt64(from: fromU8Val);
            let lit255: std.num.UInt64 = 255;
            if fromU8.equals(lit255) == false { return 12 }

            // parse — valid large value
            let parsed = std.num.UInt64.parse( "18446744073709551615");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.num.UInt64.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.num.UInt64.parse( "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.num.UInt64.zero) == false { return 16 }

            // parse — out of range (18446744073709551616 > max)
            let parsedOver = std.num.UInt64.parse( "18446744073709551616");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.num.UInt64.parse( "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.num.UInt64.parse( "");
            if parsedEmpty.isSome() { return 19 }

            0
        }
