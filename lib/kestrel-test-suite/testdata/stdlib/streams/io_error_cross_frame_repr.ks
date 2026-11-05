// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression against the shipped stdlib shape, moved from std.io.
//
// `IoError` is a struct of an `IoErrorKind` enum and an `Int32` code.
// Cranelift once collapsed a small aggregate to `Scalar(I64)`, so an
// `IoError` value was really the constructing frame's stack-slot address.
// Every `Result[T, IoError]` std hands back carries this type. This file
// covers values that outlive the frame that built them, several live at
// once, in an array, and through a `Result`.

module Test

import std.streams.(IoError, IoErrorKind)

func makeError(c: Int32) -> IoError {
    IoError(errno: c)
}

func code(e: IoError) -> Int32 { e.errno.unwrap(or: -1) }

@main
func main() -> lang.i64 {
    let two: Int32 = 2;
    let thirteen: Int32 = 13;
    let other: Int32 = 77;
    // Constructed in a callee, read after that frame returned.
    let a = makeError(two);
    let b = makeError(thirteen);
    let c = makeError(other);
    if code(c) != other { return 1 }
    if code(b) != thirteen { return 2 }
    if code(a) != two { return 3 }
    if c.kind != IoErrorKind.Other { return 4 }
    if a.kind != IoErrorKind.NotFound { return 5 }

    var es = Array[IoError]();
    es.append(makeError(two));
    es.append(makeError(other));
    es.append(IoError.permissionDenied);
    if es.count != 3 { return 6 }
    if code(es(0)) != two { return 7 }
    if code(es(1)) != other { return 8 }
    if es(2).kind != IoErrorKind.PermissionDenied or es(2).errno.isSome() { return 9 }

    let r: Result[Int64, IoError] = .Err(makeError(other));
    match r {
        .Ok(_) => return 10,
        .Err(e) => { if code(e) != other { return 11 } }
    };
    0
}
