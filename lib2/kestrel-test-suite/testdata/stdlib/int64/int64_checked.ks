// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let a: std.num.Int64 = 10;
            let b: std.num.Int64 = 5;

            // addChecked - normal case
            let addResult = a.addChecked(b);
            if addResult.isNone() { return 1 }
            if addResult.unwrap() != 15 { return 2 }

            // addChecked - overflow
            let maxVal = std.num.Int64.maxValue;
            let overflowAdd = maxVal.addChecked(1);
            if overflowAdd.isSome() { return 3 }

            // subtractChecked - normal case
            let subResult = a.subtractChecked(b);
            if subResult.isNone() { return 4 }
            if subResult.unwrap() != 5 { return 5 }

            // subtractChecked - underflow
            let minVal = std.num.Int64.minValue;
            let overflowSub = minVal.subtractChecked(1);
            if overflowSub.isSome() { return 6 }

            // multiplyChecked - normal case
            let mulResult = a.multiplyChecked(b);
            if mulResult.isNone() { return 7 }
            if mulResult.unwrap() != 50 { return 8 }

            // multiplyChecked - overflow
            let overflowMul = maxVal.multiplyChecked(2);
            if overflowMul.isSome() { return 9 }

            // divideChecked - normal case
            let divResult = a.divideChecked(b);
            if divResult.isNone() { return 10 }
            if divResult.unwrap() != 2 { return 11 }

            // divideChecked - division by zero
            let zero: std.num.Int64 = 0;
            let divZero = a.divideChecked(zero);
            if divZero.isSome() { return 12 }

            // divideChecked - minValue / -1 overflow
            let negOne: std.num.Int64 = -1;
            let divOverflow = minVal.divideChecked(negOne);
            if divOverflow.isSome() { return 13 }

            // negateChecked - normal case
            let negResult = a.negateChecked();
            if negResult.isNone() { return 14 }
            if negResult.unwrap() != -10 { return 15 }

            // negateChecked - overflow (minValue)
            let negOverflow = minVal.negateChecked();
            if negOverflow.isSome() { return 16 }

            // absChecked - normal case
            let negFive: std.num.Int64 = -5;
            let absResult = negFive.absChecked();
            if absResult.isNone() { return 17 }
            if absResult.unwrap() != 5 { return 18 }

            // absChecked - overflow (minValue)
            let absOverflow = minVal.absChecked();
            if absOverflow.isSome() { return 19 }

            0
        }
