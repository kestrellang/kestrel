// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let maxVal = std.numeric.Int8.maxValue;
            let minVal = std.numeric.Int8.minValue;
            let one: std.numeric.Int8 = 1;
            let negOne: std.numeric.Int8 = -1;

            // addChecked — overflow at 127
            let addCheckZero: std.numeric.Int8 = 0;
            let addOk = maxVal.addChecked(addCheckZero);
            if addOk.isNone() { return 1 }
            let addOverflow = maxVal.addChecked(one);
            if addOverflow.isSome() { return 2 }

            // addChecked — normal case
            let ten: std.numeric.Int8 = 10;
            let five: std.numeric.Int8 = 5;
            let addNormal = ten.addChecked(five);
            if addNormal.isNone() { return 3 }
            let expectedFifteen: std.numeric.Int8 = 15;
            if addNormal.unwrap() != expectedFifteen { return 4 }

            // subtractChecked — underflow at -128
            let subOverflow = minVal.subtractChecked(one);
            if subOverflow.isSome() { return 5 }

            // subtractChecked — normal case
            let subNormal = ten.subtractChecked(five);
            if subNormal.isNone() { return 6 }
            let expectedFive: std.numeric.Int8 = 5;
            if subNormal.unwrap() != expectedFive { return 7 }

            // multiplyChecked — overflow near boundaries
            let big: std.numeric.Int8 = 100;
            let two: std.numeric.Int8 = 2;
            let mulOverflow = big.multiplyChecked(two);
            if mulOverflow.isSome() { return 8 }

            // multiplyChecked — normal case
            let three: std.numeric.Int8 = 3;
            let mulNormal = five.multiplyChecked(three);
            if mulNormal.isNone() { return 9 }
            let expectedMulFifteen: std.numeric.Int8 = 15;
            if mulNormal.unwrap() != expectedMulFifteen { return 10 }

            // negateChecked — overflow at -128 (no positive 128 in Int8)
            let negMin = minVal.negateChecked();
            if negMin.isSome() { return 11 }

            // negateChecked — normal case
            let negTen = ten.negateChecked();
            if negTen.isNone() { return 12 }
            let expectedNegTen: std.numeric.Int8 = -10;
            if negTen.unwrap() != expectedNegTen { return 13 }

            // absChecked — overflow at -128
            let absMin = minVal.absChecked();
            if absMin.isSome() { return 14 }

            // absChecked — normal case
            let negFive: std.numeric.Int8 = -5;
            let absFive = negFive.absChecked();
            if absFive.isNone() { return 15 }
            if absFive.unwrap() != five { return 16 }

            // addSaturating — clamps to 127
            let addSat = maxVal.addSaturating(one);
            if addSat != maxVal { return 17 }
            let satHundred: std.numeric.Int8 = 100;
            let addSatBig = maxVal.addSaturating(satHundred);
            if addSatBig != maxVal { return 18 }

            // addSaturating — clamps to -128
            let addSatNeg = minVal.addSaturating(negOne);
            if addSatNeg != minVal { return 19 }

            // subtractSaturating — clamps to -128
            let subSat = minVal.subtractSaturating(one);
            if subSat != minVal { return 20 }

            // subtractSaturating — clamps to 127
            let subSatPos = maxVal.subtractSaturating(negOne);
            if subSatPos != maxVal { return 21 }

            // multiplySaturating — clamps to 127
            let mulSat = big.multiplySaturating(two);
            if mulSat != maxVal { return 22 }

            // multiplySaturating — clamps to -128 (positive * negative overflow)
            let negBig: std.numeric.Int8 = -100;
            let mulSatNeg = negBig.multiplySaturating(two);
            if mulSatNeg != minVal { return 23 }

            // negateSaturating — -128 saturates to 127
            let negSatMin = minVal.negateSaturating();
            if negSatMin != maxVal { return 24 }

            // negateSaturating — normal case
            let negSatTen = ten.negateSaturating();
            let expectedNegSatTen: std.numeric.Int8 = -10;
            if negSatTen != expectedNegSatTen { return 25 }

            // absSaturating — -128 saturates to 127
            let absSatMin = minVal.absSaturating();
            if absSatMin != maxVal { return 26 }

            // absSaturating — normal case
            let absSatNeg = negFive.absSaturating();
            if absSatNeg != five { return 27 }

            0
        }
