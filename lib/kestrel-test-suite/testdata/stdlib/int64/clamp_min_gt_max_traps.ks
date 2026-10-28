// test: execution
// stdlib: true
// expect-exit: -1

// clamp(min:max:) with min > max used to be documented as "undefined";
// it now traps via fatalError (signal termination → exit code -1).

module Test

@main
func main() {
    let x: Int64 = 5;
    let _ = x.clamp(10, 0);
}
