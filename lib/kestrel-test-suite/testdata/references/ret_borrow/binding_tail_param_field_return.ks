// test: execution
// stdlib: true
// backends: cranelift,llvm

// A named `&mutating` binding of a receiver field carries the Param root
// (not the field-address temp), so returning the binding as the function
// tail passes the escape check and the caller writes through it.
module Test

struct Box {
    var v: Int64

    mutating func view() -> &mutating Int64 {
        let r = &mutating self.v;
        r
    }
}

@main
func main() -> lang.i64 {
    var b = Box(v: 1);
    b.view() = 7;
    if b.v != 7 { return 1; }
    0
}
