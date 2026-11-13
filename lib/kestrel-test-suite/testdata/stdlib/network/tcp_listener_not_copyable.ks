// test: diagnostics
// stdlib: true

module Test

import std.network.(TcpListener)

// `TcpListener` owns a socket fd and closes it in `deinit`. It must be
// `not Copyable`: a copy would run that deinit twice and close the same fd
// twice. Copying is a move, so using the original afterwards is E500.
func main() {
    if let .Ok(a) = TcpListener.bind(port: 0) {
        let b = a;
        let fd = a.rawDescriptor;       // ERROR: use of moved value
    }
}
