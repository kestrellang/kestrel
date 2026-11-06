// test: diagnostics
// stdlib: true

module Test

import std.network.(TcpStream)

// `TcpStream` owns its descriptor like `File`: it is `not Copyable`, and
// `intoRawDescriptor` consumes it, so a detached stream can't be used.
func main() {
    if let .Ok(s) = TcpStream.connect("localhost", 80) {
        let fd = s.intoRawDescriptor();
        let again = s.rawDescriptor;       // ERROR: use of moved value
    }
}
