// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let generated = std.collections.Array[std.num.Int64](4, { (i) in i * i });
            if generated.count != 4 { return 1 }
            0
        }
