// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Construction via postfix .. operator
            let r = 5..;

            // contains - value at start
            if r.contains(5) == false { return 1 }

            // contains - value above start
            if r.contains(100) == false { return 2 }

            // contains - value below start
            if r.contains(4) { return 3 }

            // contains - zero
            if r.contains(0) { return 4 }

            // equality
            let r2 = 5..;
            if r.isEqual(to: r2) == false { return 5 }

            let r3 = 6..;
            if r.isEqual(to: r3) { return 6 }

            // iteration with break (sum 0..4 = 0+1+2+3+4 = 10)
            var sum: std.numeric.Int64 = 0;
            for i in 0.. {
                if i >= 5 { break; }
                sum = sum + i;
            }
            if sum != 10 { return 7 }

            // start field
            let r4 = 42..;
            if r4.start != 42 { return 8 }

            0
        }
