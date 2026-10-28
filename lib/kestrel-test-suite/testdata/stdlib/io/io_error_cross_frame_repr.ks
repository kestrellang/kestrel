// test: execution
// stdlib: true
// backends: cranelift,llvm

// F40 regression against the SHIPPED stdlib shape.
//
// `IoError` is a one-field struct whose field is `IoErrorKind`, a
// payload-carrying enum (`.Other(Int32)`) laid out in 8 bytes => Aggregate.
// Cranelift's `classify_named` collapsed that to `Scalar(I64)`, so an
// `IoError` value was really the constructing frame's stack-slot ADDRESS.
// Every `Result[T, IoError]` returned by `Read`/`Write`/`File`/`os.fs` carries
// this type, so the corruption was reachable from ordinary error handling.
//
// The sibling `io_error_types.ks` passes on both backends because it only ever
// constructs and consumes one `IoError` inside a single frame — the one shape
// that worked. This file covers what it cannot: values that OUTLIVE the frame
// that built them, and several live at once.
module Test

func makeError(c: std.numeric.Int32) -> std.io.error.IoError {
    std.io.error.IoError(code: c)
}

func errno(e: std.io.error.IoError) -> std.numeric.Int32 { e.kind.errno() }

@main
func main() -> lang.i64 {
    let two: std.numeric.Int32 = 2;
    let thirteen: std.numeric.Int32 = 13;
    let other: std.numeric.Int32 = 77;

    // Constructed in a callee, read after that frame returned. Three live at
    // once, so slot reuse in `makeError` would make them all agree.
    let a = makeError(two);
    let b = makeError(thirteen);
    let c = makeError(other);
    if errno(c) != other { return 1 }
    if errno(b) != thirteen { return 2 }
    if errno(a) != two { return 3 }

    // `.Other(code)` is the arm that actually carries a payload.
    if not c.description().isEqual(to: "unknown error") { return 4 }
    if not a.description().isEqual(to: "no such file or directory") { return 5 }

    // Heap storage: an Array of errors, read back after appends.
    var es = std.collections.Array[std.io.error.IoError]();
    es.append(makeError(two));
    es.append(makeError(other));
    es.append(std.io.error.permissionDenied());
    if es.count != 3 { return 6 }
    if es(0).kind.errno() != two { return 7 }
    if es(1).kind.errno() != other { return 8 }
    if es(2).kind.errno() != thirteen { return 9 }

    // Through a Result, the way every io API actually hands one back.
    let r: std.result.Result[std.numeric.Int64, std.io.error.IoError] = .Err(makeError(other));
    match r {
        .Ok(_) => return 10,
        .Err(e) => { if errno(e) != other { return 11 } }
    };
    0
}
