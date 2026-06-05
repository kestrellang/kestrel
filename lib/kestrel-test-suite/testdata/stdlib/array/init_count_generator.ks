// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let generated = std.collections.Array[std.numeric.Int64](of: 4, generatedBy: { (i) in i * i });
            if generated.count != 4 { return 1 }
            0
        }
