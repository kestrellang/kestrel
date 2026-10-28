// test: execution
// stdlib: true
// expect-exit: -1

// stepBy(0) used to be documented as "undefined (spins forever)"; it now
// traps at construction via fatalError (signal termination → exit -1).

module Test

@main
func main() {
    let arr = [1, 2, 3];
    var it = arr.iter().stepBy(0);
    let _ = it.next();
}
