// Rosetta Code: Higher-order functions
// https://rosettacode.org/wiki/Higher-order_functions
module Main

func first(f: () -> Int64) -> Int64 { f() }
func twice(f: (Int64) -> Int64, x: Int64) -> Int64 { f(f(x)) }
func second() -> Int64 { 42 }
func addThree(x: Int64) -> Int64 { x + 3 }

@main
func main() {
    println(first(second));
    println(twice(addThree, 10));
    println(twice({ it * it }, 3));
    println([1, 2, 3, 4].map { it * 10 });
}
