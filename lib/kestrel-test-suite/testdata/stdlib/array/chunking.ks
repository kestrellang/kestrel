// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);

            // chunks(of:) — returns ChunksView; .iter() for the cursor
            var chunkIter = arr.chunks(of: 2).iter();

            // First chunk: [1, 2]
            let c1 = chunkIter.next();
            if c1.isNone() { return 1 }
            let chunk1 = c1.unwrap();
            if chunk1.count != 2 { return 2 }
            if chunk1(unchecked: 0) != 1 { return 3 }
            if chunk1(unchecked: 1) != 2 { return 4 }

            // Second chunk: [3, 4]
            let c2 = chunkIter.next();
            if c2.isNone() { return 5 }
            let chunk2 = c2.unwrap();
            if chunk2.count != 2 { return 6 }
            if chunk2(unchecked: 0) != 3 { return 7 }
            if chunk2(unchecked: 1) != 4 { return 8 }

            // Third chunk: [5] (smaller last chunk)
            let c3 = chunkIter.next();
            if c3.isNone() { return 9 }
            let chunk3 = c3.unwrap();
            if chunk3.count != 1 { return 10 }
            if chunk3(unchecked: 0) != 5 { return 11 }

            // No more chunks
            let c4 = chunkIter.next();
            if c4.isSome() { return 12 }

            // windows(of:)
            var arr2 = std.collections.Array[std.numeric.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(3); arr2.append(4);
            var winIter = arr2.windows(of: 2).iter();

            // Window 1: [1, 2]
            let w1 = winIter.next();
            if w1.isNone() { return 13 }
            let win1 = w1.unwrap();
            if win1.count != 2 { return 14 }
            if win1(unchecked: 0) != 1 { return 15 }
            if win1(unchecked: 1) != 2 { return 16 }

            // Window 2: [2, 3]
            let w2 = winIter.next();
            if w2.isNone() { return 17 }
            let win2 = w2.unwrap();
            if win2(unchecked: 0) != 2 { return 18 }
            if win2(unchecked: 1) != 3 { return 19 }

            // Window 3: [3, 4]
            let w3 = winIter.next();
            if w3.isNone() { return 20 }
            let win3 = w3.unwrap();
            if win3(unchecked: 0) != 3 { return 21 }
            if win3(unchecked: 1) != 4 { return 22 }

            // No more windows
            let w4 = winIter.next();
            if w4.isSome() { return 23 }

            0
        }
