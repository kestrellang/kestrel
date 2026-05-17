// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // bitwiseAnd: 0b1100 & 0b1010 = 0b1000 = 8
            let a: std.numeric.Int64 = 12;
            let b: std.numeric.Int64 = 10;
            if a.bitwiseAnd(b) != 8 { return 1 }

            // bitwiseOr: 0b1100 | 0b1010 = 0b1110 = 14
            if a.bitwiseOr(b) != 14 { return 2 }

            // bitwiseXor: 0b1100 ^ 0b1010 = 0b0110 = 6
            if a.bitwiseXor(b) != 6 { return 3 }

            // bitwiseNot: ~0 = -1
            let zero: std.numeric.Int64 = 0;
            if zero.bitwiseNot() != -1 { return 4 }
            // bitwiseNot: ~(-1) = 0
            let negOne: std.numeric.Int64 = -1;
            if negOne.bitwiseNot() != 0 { return 5 }

            // shiftLeft: 1 << 4 = 16
            let one: std.numeric.Int64 = 1;
            if one.shiftLeft(by: 4) != 16 { return 6 }

            // shiftRight: 16 >> 2 = 4
            let sixteen: std.numeric.Int64 = 16;
            if sixteen.shiftRight(by: 2) != 4 { return 7 }

            // shiftRight with negative preserves sign: -16 >> 2 = -4
            let negSixteen: std.numeric.Int64 = -16;
            if negSixteen.shiftRight(by: 2) != -4 { return 8 }

            // rotateLeft
            // rotate 1 left by 1: should be 2
            if one.rotateLeft(by: 1) != 2 { return 9 }
            // rotate by 0: unchanged
            let val: std.numeric.Int64 = 42;
            if val.rotateLeft(by: 0) != 42 { return 10 }

            // rotateRight
            // rotate 2 right by 1: should be 1
            let two: std.numeric.Int64 = 2;
            if two.rotateRight(by: 1) != 1 { return 11 }
            // rotate by 0: unchanged
            if val.rotateRight(by: 0) != 42 { return 12 }

            // Verify rotateLeft and rotateRight are inverses
            let testVal: std.numeric.Int64 = 12345;
            if testVal.rotateLeft(by: 7).rotateRight(by: 7) != 12345 { return 13 }

            0
        }
