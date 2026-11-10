// test: execution
// stdlib: true
// expect-stdout: beta gamma\nalpha, beta, gamma, delta\n\n123\nb+c+d\n
//
// `joined(by:)` lives on `Sliceable`, so a slice of an array joins without
// copying it into an array first. An empty slice joins to "".

module Test

@main
func main() {
    let ws = ["alpha", "beta", "gamma", "delta"];
    println(ws(1..<3).joined(by: " "));
    println(ws.joined(by: ", "));
    println(ws(2..<2).joined(by: ","));
    println([1, 2, 3].joined());
    let parts = "a b c d".split(by: " ").collect();
    println(parts(1..<parts.count).joined(by: "+"));
}
