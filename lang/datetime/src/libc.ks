module datetime

import std.ffi.(FFISafe, CString)
import std.memory.(Pointer)

// Clock access
@extern(.C)
func kestrel_clock_gettime(sec: Pointer[Int64], nsec: Pointer[Int64])

// System timezone helpers
@extern(.C)
func kestrel_localtime_gmtoff(epochSec: Int64) -> Int64

@extern(.C)
func kestrel_localtime_zone(epochSec: Int64, buf: Pointer[UInt8], bufLen: Int64)

@extern(.C)
func kestrel_system_timezone_name(buf: Pointer[UInt8], bufLen: Int64)

// Timezone registry
@extern(.C)
func kestrel_tz_find_or_register(name: CString) -> Int64

@extern(.C)
func kestrel_tz_find(name: CString) -> Int64

@extern(.C)
func kestrel_tz_offset(tzId: Int64, epochSec: Int64) -> Int32

@extern(.C)
func kestrel_tz_is_dst(tzId: Int64, epochSec: Int64) -> Int32

@extern(.C)
func kestrel_tz_name(tzId: Int64, buf: Pointer[UInt8], bufLen: Int64)

@extern(.C)
func kestrel_tz_abbr(tzId: Int64, epochSec: Int64, buf: Pointer[UInt8], bufLen: Int64)

@extern(.C)
func kestrel_tz_transition_count(tzId: Int64) -> Int64

@extern(.C)
func kestrel_tz_transition_at(tzId: Int64, index: Int64,
                               epochOut: Pointer[Int64],
                               offsetBeforeOut: Pointer[Int32],
                               offsetAfterOut: Pointer[Int32])

// libc time functions
@extern(.C)
func time(tloc: Pointer[Int64]) -> Int64

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
func gmtime_r(timep: Pointer[Int64], result: Pointer[Tm]) -> Pointer[Tm]
