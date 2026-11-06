// test: execution
// stdlib: true

// `unwrap(or:)` on an optional holding a non-Copyable payload moves the
// payload (or the default) out instead of bit-copying it.

module Test

struct Handle: not Copyable {
    var fd: Int64
    deinit { }
}

@main
func main() -> lang.i64 {
    let empty: Optional[Handle] = .None;
    let a = empty.unwrap(or: Handle(fd: -1));
    let full: Optional[Handle] = .Some(Handle(fd: 7));
    let b = full.unwrap(or: Handle(fd: -1));
    if a.fd == -1 and b.fd == 7 { 0 } else { 1 }
}
