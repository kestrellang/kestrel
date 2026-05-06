// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let arr = [10, 20, 30, 40, 50];

            // RangeFrom: arr(2..) = [30, 40, 50]
            let tail = arr(2..);
            if tail.count != 3 { return 1 }
            if tail(0) != 30 { return 2 }
            if tail(1) != 40 { return 3 }
            if tail(2) != 50 { return 4 }

            // RangeFrom: arr(0..) = entire array
            let all = arr(0..);
            if all.count != 5 { return 5 }

            // RangeFrom: arr(5..) = empty slice
            let empty = arr(5..);
            if empty.count != 0 { return 6 }

            // RangeUpTo: arr(..<3) = [10, 20, 30]
            let head = arr(..<3);
            if head.count != 3 { return 7 }
            if head(0) != 10 { return 8 }
            if head(1) != 20 { return 9 }
            if head(2) != 30 { return 10 }

            // RangeUpTo: arr(..<0) = empty
            let empty2 = arr(..<0);
            if empty2.count != 0 { return 11 }

            // RangeThrough: arr(..=2) = [10, 20, 30]
            let first3 = arr(..=2);
            if first3.count != 3 { return 12 }
            if first3(0) != 10 { return 13 }
            if first3(2) != 30 { return 14 }

            // RangeThrough: arr(..=4) = entire array
            let all2 = arr(..=4);
            if all2.count != 5 { return 15 }

            0
        }
