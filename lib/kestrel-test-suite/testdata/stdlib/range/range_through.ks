// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Construction via prefix ..= operator
            let r = ..=10;

            // contains - value below end
            if r.contains(0) == false { return 1 }
            if r.contains(9) == false { return 2 }
            if r.contains(-5) == false { return 3 }

            // contains - end is inclusive
            if r.contains(10) == false { return 4 }

            // contains - value above end
            if r.contains(11) { return 5 }

            // equality
            let r2 = ..=10;
            if r.isEqual(to: r2) == false { return 6 }

            let r3 = ..=11;
            if r.isEqual(to: r3) { return 7 }

            // end field
            let r4 = ..=42;
            if r4.end != 42 { return 8 }

            0
        }
