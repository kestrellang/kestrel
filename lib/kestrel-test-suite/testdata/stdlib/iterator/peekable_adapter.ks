// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);

            // ---- peekable() ----
            var iter = arr.iter().peekable();

            // Peek doesn't consume
            let p1 = iter.peek();
            if p1.isNone() { return 1 }
            if p1.unwrap() != 1 { return 2 }

            // Peek again returns same value
            let p2 = iter.peek();
            if p2.unwrap() != 1 { return 3 }

            // next() consumes
            let n1 = iter.next();
            if n1.unwrap() != 1 { return 4 }

            // Peek now shows next element
            let p3 = iter.peek();
            if p3.unwrap() != 2 { return 5 }

            // Consume remaining
            iter.next();
            iter.next();
            let pEnd = iter.peek();
            if pEnd.isSome() { return 6 }

            0
        }
