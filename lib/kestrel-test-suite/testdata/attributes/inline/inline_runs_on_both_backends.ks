// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

module Test
import Prelude

@inline(.Always)
func double(x: Int64) -> Int64 { x * 2 }

@inline(.Never)
func triple(x: Int64) -> Int64 { x * 3 }

struct Counter {
    var value: Int64

    @inline
    var twice: Int64 { self.value * 2 }
}

@main
func main() -> Int64 {
    let c = Counter(value: 5);
    double(3) + triple(2) + c.twice - 22
}
