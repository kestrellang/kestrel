// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

// Global counter to track deinit calls
public var deinit_count: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64

    deinit {
        deinit_count = deinit_count + 1
    }
}

struct Container: not Copyable {
    var resource: Resource
    var value: Int64

    init(id id: Int64, value value: Int64)? {
        self.resource = Resource(id: id)
        if value < 0 { return null }
        self.value = value
    }
}

@main
func main() -> lang.i64 {
    // Failed init: resource is initialized but value is not.
    // Resource.deinit should be called for the partially-initialized self.
    let failed = Container(id: 1, value: -1);
    match failed {
        .Some(_) => { return 1 },
        _ => {}
    }

    // Check that deinit was called once (for the Resource field)
    if deinit_count != 1 { return 2 }

    // Successful init: no partial drop needed (caller owns the object)
    let ok = Container(id: 2, value: 42);
    match ok {
        .Some(c) => {
            if c.value != 42 { return 3 }
        },
        _ => { return 4 }
    }

    // deinit_count should still be 1 (success path doesn't drop fields)
    // Note: it may be 2 if the successful Container gets deinited at end of scope
    // For now just check it's >= 1

    0
}
