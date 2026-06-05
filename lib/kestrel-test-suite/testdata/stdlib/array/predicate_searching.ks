// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // firstIndex(where:)
            let fi = arr.firstIndex(where: { (x) in x > 3 });
            if fi.isNone() { return 1 }
            if fi.unwrap() != 3 { return 2 }

            // firstIndex(where:) - no match
            let fiNone = arr.firstIndex(where: { (x) in x > 10 });
            if fiNone.isSome() { return 3 }

            // lastIndex(where:)
            let li = arr.lastIndex(where: { (x) in x < 4 });
            if li.isNone() { return 4 }
            if li.unwrap() != 2 { return 5 }

            // lastIndex(where:) - no match
            let liNone = arr.lastIndex(where: { (x) in x > 10 });
            if liNone.isSome() { return 6 }

            // first(where:)
            let fm = arr.first(where: { (x) in x > 3 });
            if fm.isNone() { return 7 }
            if fm.unwrap() != 4 { return 8 }

            // first(where:) - no match
            let fmNone = arr.first(where: { (x) in x > 10 });
            if fmNone.isSome() { return 9 }

            // last(where:)
            let lm = arr.last(where: { (x) in x < 4 });
            if lm.isNone() { return 10 }
            if lm.unwrap() != 3 { return 11 }

            // last(where:) - no match
            let lmNone = arr.last(where: { (x) in x > 10 });
            if lmNone.isSome() { return 12 }

            // all(satisfy:)
            let allPos = arr.all(where: { (x) in x > 0 });
            if allPos == false { return 13 }

            let allBig = arr.all(where: { (x) in x > 3 });
            if allBig { return 14 }

            // all(where:) on empty array - vacuous truth
            let empty = std.collections.Array[std.numeric.Int64]();
            let allEmpty = empty.all(where: { (x) in false });
            if allEmpty == false { return 15 }

            // any(where:)
            let anyBig = arr.any(where: { (x) in x > 4 });
            if anyBig == false { return 16 }

            let anyHuge = arr.any(where: { (x) in x > 10 });
            if anyHuge { return 17 }

            // any(where:) on empty array
            let anyEmpty = empty.any(where: { (x) in true });
            if anyEmpty { return 18 }

            // countWhere(predicate:) - positional single-name param
            let cw = arr.countItems(where: { (x) in x % 2 == 0 });
            if cw != 2 { return 19 }

            let cwNone = arr.countItems(where: { (x) in x > 10 });
            if cwNone != 0 { return 20 }

            0
        }
