// test: execution
// stdlib: true
// expect-exit: 0

// Copying enums out of an array. `Op` holds only plain-bits payloads (a
// one-field `UInt64` struct, a payload-free enum, `Int64`s) so its copy is a
// bit copy with no clone shim call; `Mixed` also has a `String` payload, so
// its copy runs the clone shim, which must retain the string (a missed retain
// double-frees it when both copies drop).
module Test

import std.numeric.(Int64, UInt64)
import std.text.(String)

struct Value { var bits: UInt64; }
enum BinOp { case Add case Sub }

enum Op {
    case Load(dst: Int64, value: Value)
    case Binary(op: BinOp, dst: Int64, a: Int64)
    case Halt
}

enum Mixed {
    case Load(dst: Int64, value: Value)
    case Error(code: Int64, message: String)
    case Halt
}

func score(op: Op) -> Int64 {
    match op {
        .Load(d, v) => d + Int64(bitPattern: v.bits),
        .Binary(o, d, a) => match o { .Add => d + a, .Sub => d - a },
        .Halt => 0,
    }
}

func mixedScore(m: Mixed) -> Int64 {
    match m {
        .Load(d, v) => d + Int64(bitPattern: v.bits),
        .Error(c, msg) => c + msg.bytes.count,
        .Halt => 0,
    }
}

@main
func main() -> Int64 {
    var ops: std.collections.Array[Op] = [
        Op.Load(dst: 1, value: Value(bits: 10)),
        Op.Binary(op: BinOp.Sub, dst: 7, a: 2),
        Op.Halt,
    ];
    var total: Int64 = 0;
    var i: Int64 = 0;
    while i < ops.count {
        let op = ops(i);
        let again = op;
        total = total + score(op) + score(again);
        i = i + 1;
    }
    if total != 32 { return 1; }

    var ms: std.collections.Array[Mixed] = [
        Mixed.Error(code: 3, message: "boom"),
        Mixed.Load(dst: 2, value: Value(bits: 5)),
    ];
    var mt: Int64 = 0;
    var j: Int64 = 0;
    while j < 3 {
        let m = ms(j % 2);
        let copy = m;
        mt = mt + mixedScore(m) + mixedScore(copy);
        j = j + 1;
    }
    // Error: 3 + 4 = 7, Load: 2 + 5 = 7; visited Error, Load, Error, twice each.
    if mt != 42 { return 2; }
    0
}
