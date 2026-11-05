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
    public static var processorCount: Int64 { native_processor_count() }

    /// The memory page size in bytes.
    public static var pageSize: Int64 { native_page_size() }

    /// The machine's host name.
    public static func hostName() -> Result[String, IoError] {
        let size: Int64 = 256;
        let buf = malloc(size + 1);
        std.ffi.memset(buf, 0, size + 1);
        if native_gethostname(buf, size) != 0 {
            let error = IoError.last();
            free(buf);
            return .Err(error)
        }
        let name = stringFromNative(buf) ?? String();
        free(buf);
        .Ok(name)
    }
}
