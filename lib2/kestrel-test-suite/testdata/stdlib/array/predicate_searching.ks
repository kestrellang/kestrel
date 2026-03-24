// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // firstIndex(matching:)
            let fi = arr.firstIndex(matching: { (x) in x > 3 });
            if fi.isNone() { return 1 }
            if fi.unwrap() != 3 { return 2 }

            // firstIndex(matching:) - no match
            let fiNone = arr.firstIndex(matching: { (x) in x > 10 });
            if fiNone.isSome() { return 3 }

            // lastIndex(matching:)
            let li = arr.lastIndex(matching: { (x) in x < 4 });
            if li.isNone() { return 4 }
            if li.unwrap() != 2 { return 5 }

            // lastIndex(matching:) - no match
            let liNone = arr.lastIndex(matching: { (x) in x > 10 });
            if liNone.isSome() { return 6 }

            // first(matching:)
            let fm = arr.first(matching: { (x) in x > 3 });
            if fm.isNone() { return 7 }
            if fm.unwrap() != 4 { return 8 }

            // first(matching:) - no match
            let fmNone = arr.first(matching: { (x) in x > 10 });
            if fmNone.isSome() { return 9 }

            // last(matching:)
            let lm = arr.last(matching: { (x) in x < 4 });
            if lm.isNone() { return 10 }
            if lm.unwrap() != 3 { return 11 }

            // last(matching:) - no match
            let lmNone = arr.last(matching: { (x) in x > 10 });
            if lmNone.isSome() { return 12 }

            // all(satisfy:)
            let allPos = arr.all(satisfy: { (x) in x > 0 });
            if allPos == false { return 13 }

            let allBig = arr.all(satisfy: { (x) in x > 3 });
            if allBig { return 14 }

            // all(satisfy:) on empty array - vacuous truth
            let empty = std.collections.Array[std.num.Int64]();
            let allEmpty = empty.all(satisfy: { (x) in false });
            if allEmpty == false { return 15 }

            // any(satisfy:)
            let anyBig = arr.any(satisfy: { (x) in x > 4 });
            if anyBig == false { return 16 }

            let anyHuge = arr.any(satisfy: { (x) in x > 10 });
            if anyHuge { return 17 }

            // any(satisfy:) on empty array
            let anyEmpty = empty.any(satisfy: { (x) in true });
            if anyEmpty { return 18 }

            // countWhere(predicate:) - positional single-name param
            let cw = arr.countWhere({ (x) in x % 2 == 0 });
            if cw != 2 { return 19 }

            let cwNone = arr.countWhere({ (x) in x > 10 });
            if cwNone != 0 { return 20 }

            0
        }
