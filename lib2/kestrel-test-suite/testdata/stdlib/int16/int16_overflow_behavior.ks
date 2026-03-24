// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let maxVal = std.num.Int16.maxValue;
            let minVal = std.num.Int16.minValue;
            let one: std.num.Int16 = 1;
            let negOne: std.num.Int16 = -1;

            // addChecked — overflow at 32767
            let addOverflow = maxVal.addChecked(one);
            if addOverflow.isSome() { return 1 }

            // addChecked — normal case
            let hundred: std.num.Int16 = 100;
            let fifty: std.num.Int16 = 50;
            let addNormal = hundred.addChecked(fifty);
            if addNormal.isNone() { return 2 }
            let expectedAdd: std.num.Int16 = 150;
            if addNormal.unwrap() != expectedAdd { return 3 }

            // subtractChecked — underflow at -32768
            let subOverflow = minVal.subtractChecked(one);
            if subOverflow.isSome() { return 4 }

            // subtractChecked — normal case
            let subNormal = hundred.subtractChecked(fifty);
            if subNormal.isNone() { return 5 }
            let expectedSub: std.num.Int16 = 50;
            if subNormal.unwrap() != expectedSub { return 6 }

            // multiplyChecked — overflow near boundaries
            let big: std.num.Int16 = 20000;
            let two: std.num.Int16 = 2;
            let mulOverflow = big.multiplyChecked(two);
            if mulOverflow.isSome() { return 7 }

            // multiplyChecked — normal case
            let ten: std.num.Int16 = 10;
            let five: std.num.Int16 = 5;
            let mulNormal = ten.multiplyChecked(five);
            if mulNormal.isNone() { return 8 }
            let expectedMul: std.num.Int16 = 50;
            if mulNormal.unwrap() != expectedMul { return 9 }

            // negateChecked — overflow at -32768
            let negMin = minVal.negateChecked();
            if negMin.isSome() { return 10 }

            // negateChecked — normal case
            let negHundred = hundred.negateChecked();
            if negHundred.isNone() { return 11 }
            let expectedNeg: std.num.Int16 = -100;
            if negHundred.unwrap() != expectedNeg { return 12 }

            // absChecked — overflow at -32768
            let absMin = minVal.absChecked();
            if absMin.isSome() { return 13 }

            // absChecked — normal case
            let negFifty: std.num.Int16 = -50;
            let absFifty = negFifty.absChecked();
            if absFifty.isNone() { return 14 }
            if absFifty.unwrap() != fifty { return 15 }

            // addSaturating — clamps to 32767
            let addSat = maxVal.addSaturating(one);
            if addSat != maxVal { return 16 }
            let thousand: std.num.Int16 = 1000;
            let addSatBig = maxVal.addSaturating(thousand);
            if addSatBig != maxVal { return 17 }

            // addSaturating — clamps to -32768
            let addSatNeg = minVal.addSaturating(negOne);
            if addSatNeg != minVal { return 18 }

            // subtractSaturating — clamps to -32768
            let subSat = minVal.subtractSaturating(one);
            if subSat != minVal { return 19 }

            // subtractSaturating — clamps to 32767
            let subSatPos = maxVal.subtractSaturating(negOne);
            if subSatPos != maxVal { return 20 }

            // multiplySaturating — clamps to 32767
            let mulSat = big.multiplySaturating(two);
            if mulSat != maxVal { return 21 }

            // multiplySaturating — clamps to -32768 (positive * negative overflow)
            let negBig: std.num.Int16 = -20000;
            let mulSatNeg = negBig.multiplySaturating(two);
            if mulSatNeg != minVal { return 22 }

            // negateSaturating — -32768 saturates to 32767
            let negSatMin = minVal.negateSaturating();
            if negSatMin != maxVal { return 23 }

            // negateSaturating — normal case
            let negSatHundred = hundred.negateSaturating();
            let expectedNegSat: std.num.Int16 = -100;
            if negSatHundred != expectedNegSat { return 24 }

            // absSaturating — -32768 saturates to 32767
            let absSatMin = minVal.absSaturating();
            if absSatMin != maxVal { return 25 }

            // absSaturating — normal case
            let absSatNeg = negFifty.absSaturating();
            if absSatNeg != fifty { return 26 }

            0
        }
