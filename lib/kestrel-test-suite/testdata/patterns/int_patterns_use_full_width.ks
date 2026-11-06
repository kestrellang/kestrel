// test: execution
// backends: cranelift,llvm
// expect-stdout: 1 2 1 2 1 1 1 2 1 1 2
// expect-exit: 0

// Integer patterns test the value at its own width. A switch used to read
// every integer as 32 bits, so `5000000000` matched `705032704` and a bound of
// 2^62 became 0. Unsigned types compare unsigned: a `UInt64` above
// `Int64.max` used to look negative and miss its range.

module Test

func lit(x: Int64) -> Int64 { match x { 5000000000 => 1, _ => 2 } }
func big(x: Int64) -> Int64 { match x { 0..=4611686018427387904 => 1, _ => 2 } }
func top(x: UInt64) -> Int64 { match x { 9223372036854775808..=18446744073709551615 => 1, _ => 2 } }
func low(x: UInt64) -> Int64 { match x { 0..=100 => 1, _ => 2 } }
func byte(x: UInt8) -> Int64 { match x { 200..=255 => 1, _ => 2 } }
func small(x: Int8) -> Int64 { match x { -128..=-1 => 1, _ => 2 } }

@main
func main() {
    let hi: UInt64 = 18446744073709551615;
    let b: UInt8 = 250;
    let n: Int8 = -3;
    println("\(lit(5000000000)) \(lit(705032704)) \(big(5)) \(big(-1)) \(top(hi)) \(low(7)) \(byte(b)) \(top(7)) \(small(n)) \(byte(255)) \(low(hi))");
}
