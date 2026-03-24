// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 10;
            let b: std.num.Int64 = 5;
            let maxVal = std.num.Int64.maxValue;
            let minVal = std.num.Int64.minValue;

            // addSaturating - normal case
            if a.addSaturating(b) != 15 { return 1 }

            // addSaturating - overflow clamps to maxValue
            if maxVal.addSaturating(1) != maxVal { return 2 }
            if maxVal.addSaturating(100) != maxVal { return 3 }

            // subtractSaturating - normal case
            if a.subtractSaturating(b) != 5 { return 4 }

            // subtractSaturating - underflow clamps to minValue
            if minVal.subtractSaturating(1) != minVal { return 5 }

            // multiplySaturating - normal case
            if a.multiplySaturating(b) != 50 { return 6 }

            // multiplySaturating - overflow clamps to maxValue
            if maxVal.multiplySaturating(2) != maxVal { return 7 }

            // multiplySaturating - negative overflow clamps to minValue
            let negTwo: std.num.Int64 = -2;
            if maxVal.multiplySaturating(negTwo) != minVal { return 8 }

            // negateSaturating - normal case
            let fortyTwo: std.num.Int64 = 42;
            if fortyTwo.negateSaturating() != -42 { return 9 }

            // negateSaturating - minValue clamps to maxValue
            if minVal.negateSaturating() != maxVal { return 10 }

            // absSaturating - normal case
            let negFortyTwo: std.num.Int64 = -42;
            if negFortyTwo.absSaturating() != 42 { return 11 }

            // absSaturating - minValue clamps to maxValue
            if minVal.absSaturating() != maxVal { return 12 }

            0
        }
