// Test stdlib Array[T] to reproduce the issue

import std.num.(Int64, UInt8)
import std.memory.(Pointer)
import std.collections.(Array)

@extern(.C, mangleName: "exit")
func libc_exit(code: lang.i32)

func main() {
    // Create Array and check if data is correct
    var buf = Array[UInt8](capacity: Int64(intLiteral: 1));
    buf.append(UInt8(intLiteral: 104));

    // Read data back via getUnchecked
    let readBack: UInt8 = buf.getUnchecked(Int64(intLiteral: 0));
    let expected = UInt8(intLiteral: 104);

    if readBack == expected {
        libc_exit(lang.cast_i64_i32(0))
    } else {
        libc_exit(lang.cast_i64_i32(1))
    }
}
