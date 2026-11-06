// test: execution
// stdlib: true
// host: darwin
// expect-stdout: darwin\n

module Test

@main
func main() -> lang.i64 {
    println("\(Platform.current.operatingSystem)");
    if not Platform.current.operatingSystem.isEqual(to: .Darwin) { return 1 }
    0
}
