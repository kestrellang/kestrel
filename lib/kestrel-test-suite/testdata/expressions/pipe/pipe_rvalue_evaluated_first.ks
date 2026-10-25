// test: execution
// stdlib: true
// expect-exit: 0

// An rvalue on the left is evaluated before the call's other arguments,
// in reading order, and only once even when `_` is used twice.

module Test

struct Log {
    var trace: Int64
    var calls: Int64
}

func record(mutating log: Log, digit: Int64) -> Int64 {
    log.trace = log.trace * 10 + digit;
    log.calls = log.calls + 1;
    digit
}

func pair(a: Int64, b: Int64) -> Int64 { a * 10 + b }

@main
func main() -> lang.i64 {
    var log = Log(trace: 0, calls: 0);
    // `x() |> f(a(), _)`: x runs first
    let r = record(log, 1) |> pair(record(log, 2), _);
    if r != 21 { return 1 }
    if log.trace != 12 { return 2 }

    // `_` used twice: the left side still runs once
    var once = Log(trace: 0, calls: 0);
    let s = record(once, 4) |> pair(_, _ + 1);
    if s != 45 { return 3 }
    if once.calls != 1 { return 4 }
    0
}
