// test: diagnostics
// stdlib: false

// Companion to break_in_closure_inside_loop_fails.ks.

module Main

func test() {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 3) {
        let f: () -> () = { () in
            continue; // ERROR: outside of loop
        };
        i = lang.i64_add(i, 1);
    }
}
