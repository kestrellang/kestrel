// test: execution
// stdlib: true
// host: linux
// expect-stdout: linux\n

module Test

@main
func main() -> lang.i64 {
    println("\(Platform.current.operatingSystem)");
    if not Platform.current.operatingSystem.isEqual(to: .Linux) { return 1 }
    0
}
