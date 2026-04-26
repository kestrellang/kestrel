// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);

            // Test iteration
            var sum: std.num.Int64 = 0;
            var iter = arr.iter();
            var done: std.core.Bool = false;
            while done == false {
                let next = iter.next();
                if next.isSome() {
                    sum = sum + next.unwrap()
                } else {
                    done = true
                }
            }
            if sum != 60 { return 1 }

            0
        }
