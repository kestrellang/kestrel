// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var a = std.collections.Array[std.numeric.Int64]();
            a.append(1); a.append(2); a.append(3);
            var b = std.collections.Array[std.numeric.Int64]();
            b.append(1); b.append(2); b.append(3);
            var c = std.collections.Array[std.numeric.Int64]();
            c.append(1); c.append(2);
            var d = std.collections.Array[std.numeric.Int64]();
            d.append(3); d.append(2); d.append(1);

            // equals(other:) - positional single-name param
            if a.isEqual(to: b) == false { return 1 }
            if a.isEqual(to: c) { return 2 }
            if a.isEqual(to: d) { return 3 }

            // empty arrays are equal
            let e1 = std.collections.Array[std.numeric.Int64]();
            let e2 = std.collections.Array[std.numeric.Int64]();
            if e1.isEqual(to: e2) == false { return 4 }

            // contains(element:) - positional single-name param
            if a.contains(2) == false { return 5 }
            if a.contains(10) { return 6 }

            // firstIndex(of:)
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(2); arr.append(1);
            let fi = arr.firstIndex(of: 2);
            if fi.isNone() { return 7 }
            if fi.unwrap() != 1 { return 8 }

            let fiNone = arr.firstIndex(of: 10);
            if fiNone.isSome() { return 9 }

            // lastIndex(of:)
            let li = arr.lastIndex(of: 2);
            if li.isNone() { return 10 }
            if li.unwrap() != 3 { return 11 }

            let liNone = arr.lastIndex(of: 10);
            if liNone.isSome() { return 12 }

            // starts(with:)
            var prefix12 = std.collections.Array[std.numeric.Int64]();
            prefix12.append(1); prefix12.append(2);
            if a.starts(with: prefix12) == false { return 13 }
            if a.starts(with: a) == false { return 14 }
            var prefix23 = std.collections.Array[std.numeric.Int64]();
            prefix23.append(2); prefix23.append(3);
            if a.starts(with: prefix23) { return 15 }
            // empty prefix always matches
            let emptyArr = std.collections.Array[std.numeric.Int64]();
            if a.starts(with: emptyArr) == false { return 16 }
            // prefix longer than array
            if c.starts(with: a) { return 17 }

            // ends(with:)
            if a.ends(with: prefix23) == false { return 18 }
            if a.ends(with: a) == false { return 19 }
            if a.ends(with: prefix12) { return 20 }
            if a.ends(with: emptyArr) == false { return 21 }

            // split(separator:) — returns ArraySplitView; .toArray() materializes
            var splitArr = std.collections.Array[std.numeric.Int64]();
            splitArr.append(1); splitArr.append(0); splitArr.append(2); splitArr.append(0); splitArr.append(3);
            let parts = splitArr.split(0).toArray();
            if parts.count != 3 { return 22 }
            if parts(0).count != 1 { return 23 }
            if parts(0)(unchecked: 0) != 1 { return 24 }
            if parts(1).count != 1 { return 25 }
            if parts(1)(unchecked: 0) != 2 { return 26 }
            if parts(2).count != 1 { return 27 }
            if parts(2)(unchecked: 0) != 3 { return 28 }

            // split with no separator found
            let noSepParts = a.split(0).toArray();
            if noSepParts.count != 1 { return 29 }
            if noSepParts(0).count != 3 { return 30 }

            // dedup() - removes consecutive duplicates
            var dedArr = std.collections.Array[std.numeric.Int64]();
            dedArr.append(1); dedArr.append(1); dedArr.append(2); dedArr.append(2);
            dedArr.append(2); dedArr.append(3); dedArr.append(1); dedArr.append(1);
            dedArr.dedup();
            if dedArr.count != 4 { return 31 }
            if dedArr(0) != 1 { return 32 }
            if dedArr(1) != 2 { return 33 }
            if dedArr(2) != 3 { return 34 }
            if dedArr(3) != 1 { return 35 }

            // deduped() - returns new array
            var dedSrc = std.collections.Array[std.numeric.Int64]();
            dedSrc.append(1); dedSrc.append(1); dedSrc.append(2); dedSrc.append(2); dedSrc.append(3);
            let dedResult = dedSrc.deduped();
            if dedResult.count != 3 { return 36 }
            if dedResult(0) != 1 { return 37 }
            if dedResult(1) != 2 { return 38 }
            if dedResult(2) != 3 { return 39 }
            // original unchanged
            if dedSrc.count != 5 { return 40 }

            0
        }
