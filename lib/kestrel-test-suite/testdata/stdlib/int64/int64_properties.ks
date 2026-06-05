// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // sign
            let pos: std.numeric.Int64 = 42;
            let neg: std.numeric.Int64 = -42;
            let zero: std.numeric.Int64 = 0;

            if pos.sign != 1 { return 1 }
            if neg.sign != -1 { return 2 }
            if zero.sign != 0 { return 3 }

            // isPositive
            if pos.isPositive == false { return 4 }
            if zero.isPositive { return 5 }
            if neg.isPositive { return 6 }

            // isNegative
            if neg.isNegative == false { return 7 }
            if zero.isNegative { return 8 }
            if pos.isNegative { return 9 }

            // isZero
            if zero.isZero == false { return 10 }
            if pos.isZero { return 11 }

            // isPowerOfTwo
            let one: std.numeric.Int64 = 1;
            let two: std.numeric.Int64 = 2;
            let four: std.numeric.Int64 = 4;
            let three: std.numeric.Int64 = 3;
            if one.isPowerOfTwo == false { return 12 }
            if two.isPowerOfTwo == false { return 13 }
            if four.isPowerOfTwo == false { return 14 }
            if three.isPowerOfTwo { return 15 }
            if zero.isPowerOfTwo { return 16 }
            if neg.isPowerOfTwo { return 17 }

            // countOnes
            let ten: std.numeric.Int64 = 10;  // 0b1010 -> 2 ones
            if ten.countOnes != 2 { return 18 }
            if zero.countOnes != 0 { return 19 }
            let negOne: std.numeric.Int64 = -1;
            if negOne.countOnes != 64 { return 20 }

            // countZeros
            if zero.countZeros != 64 { return 21 }
            if negOne.countZeros != 0 { return 22 }

            // leadingZeros
            if one.leadingZeros != 63 { return 23 }
            if zero.leadingZeros != 64 { return 24 }
            if negOne.leadingZeros != 0 { return 25 }
            let val256: std.numeric.Int64 = 256;
            if val256.leadingZeros != 55 { return 26 }

            // trailingZeros
            let eight: std.numeric.Int64 = 8;
            if eight.trailingZeros != 3 { return 27 }
            let twelve: std.numeric.Int64 = 12;
            if twelve.trailingZeros != 2 { return 28 }
            if one.trailingZeros != 0 { return 29 }
            if zero.trailingZeros != 64 { return 30 }

            // byteSwapped
            let swapVal: std.numeric.Int64 = 1;
            // 1 as i64 in little-endian: 0x0000000000000001
            // byte-swapped: 0x0100000000000000 = 72057594037927936
            if swapVal.byteSwapped != 72057594037927936 { return 31 }

            0
        }
