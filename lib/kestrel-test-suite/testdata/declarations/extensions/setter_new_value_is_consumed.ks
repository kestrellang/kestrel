// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// A setter's `newValue` is CONSUMED (SETTER_NEW_VALUE_MODE): the setter owns
// it and can move it into storage. That is what makes a non-Copyable
// property settable at all — a borrowed `newValue` could not be stored.
// The old value is dropped exactly once, the new one moved in.

module Test

var deinits: Int64 = 0

struct Handle: not Copyable {
    var fd: Int64
    deinit { deinits += 1; }
}

struct Owner: not Copyable {
    var stored: Handle
    var handle: Handle {
        get { Handle(fd: self.stored.fd) }
        set { self.stored = newValue; }
    }
}

@main
func main() -> Int64 {
    var o = Owner(stored: Handle(fd: 1));
    o.handle = Handle(fd: 5);
    if o.stored.fd != 5 { return 1 }
    if deinits != 1 { return 2 }      // the replaced Handle(1), nothing else
    0
}
