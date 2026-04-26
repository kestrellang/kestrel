// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

protocol Doubler {
    func double() -> Self
}

extend std.num.Int64: Doubler {
    func double() -> std.num.Int64 = self + self
}

func doubleIt[T](x: T) -> T where T: Doubler = x.double()

func main() -> std.num.Int64 {
    let _ = println(doubleIt(21));
    0
}
