module wall.time

import std.ffi.(CString)
import std.memory.(Pointer)
import std.text.(String)

public func getCurrentTimestamp() -> String {
    var now: Int64 = 0;
    let _ = time(Pointer(to: now));

    var tm = Tm(
        sec: 0, min: 0, hour: 0,
        mday: 0, mon: 0, year: 0,
        wday: 0, yday: 0, isdst: 0
    );
    let _ = gmtime_r(Pointer(to: now), Pointer(to: tm));

    var buffer = Array[UInt8](repeating: 0, count: 32);
    let format = "%Y-%m-%dT%H:%M:%SZ".toCString();

    let len = strftime(
        buffer.asSlice().pointer,
        32,
        format,
        Pointer(to: tm)
    );
    format.free();

    if len > 0 {
        String.fromBytesUnchecked(buffer.asSlice().pointer, Int64(from: len))
    } else {
        "2026-01-01T00:00:00Z"
    }
}

public func getUnixTime() -> Int64 {
    var now: Int64 = 0;
    let _ = time(Pointer(to: now));
    now
}
