// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;
public var deinit_order: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
        deinit_order = deinit_order * 10 + self.id;
    }
}

func maybe_resource(should_succeed succeed: Bool, with_id id: Int64) -> Resource? {
    if succeed {
        return .Some(Resource(id: id));
    }
    return .None;
}

func test_guard(fail_on fail_on: Int64) -> Int64 {
    guard let a = maybe_resource(should_succeed: fail_on != 1, with_id: 1) else {
        return deinit_count;
    }
    guard let b = maybe_resource(should_succeed: fail_on != 2, with_id: 2) else {
        return deinit_count;
    }
    guard let c = maybe_resource(should_succeed: fail_on != 3, with_id: 3) else {
        return deinit_count;
    }
    return deinit_count;
}

func main() -> lang.i64 {
    // Fail on guard 3: a and b allocated, c fails
    // Guard else path deinits b then a (reverse order)
    deinit_count = 0;
    deinit_order = 0;
    let result = test_guard(fail_on: 3);
    if deinit_count != 2 { return 1; }
    if deinit_order != 21 { return 2; }

    0
}
