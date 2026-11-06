// Facts about the machine the process runs on.

module std.system

import std.numeric.(Int64)
import std.text.(String)
import std.result.(Result)
import std.streams.(IoError)
import std.ffi.(malloc, free)

/// The machine the process is running on.
public enum Machine {
    /// Processors this process may run on, at least 1. On Linux this is the
    /// CPU affinity mask, so it respects `taskset` and container CPU sets;
    /// it does not read cgroup CPU quotas.
    public static var processorCount: Int64 { unsafe native_processor_count() }

    /// The memory page size in bytes.
    public static var pageSize: Int64 { unsafe native_page_size() }

    /// The machine's host name.
    public static func hostName() -> Result[String, IoError] {
        let size: Int64 = 256;
        let buf = unsafe malloc(size + 1);
        unsafe std.ffi.memset(buf, 0, size + 1);
        if unsafe native_gethostname(buf, size) != 0 {
            let error = IoError.last();
            unsafe free(buf);
            return .Err(error)
        }
        let name = stringFromNative(buf) ?? String();
        unsafe free(buf);
        .Ok(name)
    }
}
