// test: execution
// stdlib: true
// expect-stdout: Fizz\n7\n-3\n

// Branches mixing a plain string literal and an interpolated string should
// both be String. Today the first branch fixes the type as "string literal"
// and the interpolated branch is E109 "expected string literal got string
// interpolation" unless the binding is annotated `: String`.
// Found while writing examples/rosetta (FizzBuzz, number names).

module Test

func fizz(n: Int64) -> String {
    let s = match n % 3 {
        0 => "Fizz",
        _ => "\(n)",
    };
    s
}

@main
func main() {
    println(fizz(3));
    println(fizz(7));
    let n: Int64 = 3;
    let signed = if n > 5 { "" } else { "-\(n)" };
    println(signed);
}
