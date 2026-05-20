module notes.time

import std.ffi.(FFISafe, CString)
import std.memory.(Pointer)

@extern(.C)
public func time(tloc: Pointer[Int64]) -> Int64

public struct Tm: FFISafe {
    public var sec: Int32
    public var min: Int32
    public var hour: Int32
    public var mday: Int32
    public var mon: Int32
    public var year: Int32
    public var wday: Int32
    public var yday: Int32
    public var isdst: Int32
}

@extern(.C)
public func gmtime_r(timep: Pointer[Int64], result: Pointer[Tm]) -> Pointer[Tm]

@extern(.C)
public func strftime(s: Pointer[UInt8], max: UInt64, format: CString, tm: Pointer[Tm]) -> UInt64
