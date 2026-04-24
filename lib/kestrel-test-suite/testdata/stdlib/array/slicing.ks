// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10); arr.append(20); arr.append(30); arr.append(40); arr.append(50);

            // prefix(count:) - positional single-name param
            let pre = arr.prefix(3);
            if pre.count != 3 { return 1 }
            if pre(unchecked: 0) != 10 { return 2 }
            if pre(unchecked: 1) != 20 { return 3 }
            if pre(unchecked: 2) != 30 { return 4 }

            // prefix(count: 0) - empty
            let preEmpty = arr.prefix(0);
            if preEmpty.count != 0 { return 5 }

            // suffix(count:) - positional single-name param
            let suf = arr.suffix(2);
            if suf.count != 2 { return 6 }
            if suf(unchecked: 0) != 40 { return 7 }
            if suf(unchecked: 1) != 50 { return 8 }

            // drop(first:)
            let df = arr.drop(first: 2);
            if df.count != 3 { return 9 }
            if df(unchecked: 0) != 30 { return 10 }
            if df(unchecked: 1) != 40 { return 11 }
            if df(unchecked: 2) != 50 { return 12 }

            // drop(last:)
            let dl = arr.drop(last: 2);
            if dl.count != 3 { return 13 }
            if dl(unchecked: 0) != 10 { return 14 }
            if dl(unchecked: 1) != 20 { return 15 }
            if dl(unchecked: 2) != 30 { return 16 }

            // drop(first: 0) - keeps everything
            let dfAll = arr.drop(first: 0);
            if dfAll.count != 5 { return 17 }

            // drop(last: 0) - keeps everything
            let dlAll = arr.drop(last: 0);
            if dlAll.count != 5 { return 18 }

            0
        }
