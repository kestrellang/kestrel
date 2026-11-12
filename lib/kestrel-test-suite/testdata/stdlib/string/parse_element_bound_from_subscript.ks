// test: execution
// stdlib: true
// expect-stdout: 7.5\n

// A binding typed late by the ref-decay defaults (`let p = pieces(2)`) still
// reaches a borrowed `&StringSlice` param through `String`'s `Indirection`:
// the argument waits for the binding's type instead of pinning it to the
// param's.

module Test

@main
func main() {
    let s = "1 2 7.5";
    let pieces = s.split(by: " ").collect();
    let p = pieces(2);
    let b = Float64(parsing: p) ?? 0.0;
    println(b);
}
