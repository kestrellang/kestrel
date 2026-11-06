// test: diagnostics

// An owning (`consuming`/`escaping`) closure moves its captures in. Capturing
// a value that was already moved — before the closure, or by the previous
// iteration of a loop — is a use after move, not a second move.

module Test

struct R: not Copyable {
    var id: Int64;
    deinit { }
}

func take(consuming r: R) { }

func straight() {
    let a = R(id: 1);
    take(a);
    let f: consuming () -> () = { () in take(a) }; // ERROR: use of moved value 'a'
    f();
}

func inLoop() {
    let a = R(id: 1);
    var i = 0;
    while i < 2 {
        let f: consuming () -> () = { () in take(a) }; // ERROR: value 'a' may have been moved
        f();
        i = i + 1;
    }
}
