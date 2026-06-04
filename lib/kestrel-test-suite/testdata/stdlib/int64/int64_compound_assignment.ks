// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // addAssign
            var x: std.numeric.Int64 = 10;
            x.addAssign(5);
            if x != 15 { return 1 }

            // subtractAssign
            var y: std.numeric.Int64 = 20;
            y.subtractAssign(8);
            if y != 12 { return 2 }

            // multiplyAssign
            var z: std.numeric.Int64 = 6;
            z.multiplyAssign(7);
            if z != 42 { return 3 }

            // divideAssign
            var d: std.numeric.Int64 = 100;
            d.divideAssign(4);
            if d != 25 { return 4 }

            // modAssign
            var m: std.numeric.Int64 = 17;
            m.modAssign(5);
            if m != 2 { return 5 }

            // bitwiseAndAssign: 0b1111 & 0b1010 = 0b1010 = 10
            var ba: std.numeric.Int64 = 15;
            ba.bitwiseAndAssign(10);
            if ba != 10 { return 6 }

            // bitwiseOrAssign: 0b1100 | 0b0011 = 0b1111 = 15
            var bo: std.numeric.Int64 = 12;
            bo.bitwiseOrAssign(3);
            if bo != 15 { return 7 }

            // bitwiseXorAssign: 0b1111 ^ 0b1010 = 0b0101 = 5
            var bx: std.numeric.Int64 = 15;
            bx.bitwiseXorAssign(10);
            if bx != 5 { return 8 }

            // shiftLeftAssign: 1 << 4 = 16
            var sl: std.numeric.Int64 = 1;
            sl.shiftLeftAssign(by: 4);
            if sl != 16 { return 9 }

            // shiftRightAssign: 32 >> 2 = 8
            var sr: std.numeric.Int64 = 32;
            sr.shiftRightAssign(by: 2);
            if sr != 8 { return 10 }

            // Chained compound assignments
            var v: std.numeric.Int64 = 5;
            v.addAssign(3);       // 8
            v.multiplyAssign(2);  // 16
            v.subtractAssign(6);  // 10
            v.divideAssign(2);    // 5
            if v != 5 { return 11 }

            0
        }
