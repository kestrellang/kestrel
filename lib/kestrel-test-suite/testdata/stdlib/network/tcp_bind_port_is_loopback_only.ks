// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `bind(port:)` used to listen on every interface (`[::]`), so a dev
// server was reachable from the network. It now listens on loopback only;
// `bind(anyInterface:)` keeps the old, network-facing behaviour.
@main
func main() -> lang.i64 {
    let local = match TcpListener.bind(port: 0) { .Ok(l) => l, .Err(_) => return 1 };
    let address = local.localAddress().unwrap();
    if not address.ip.isLoopback { return 2 }
    // "localhost" resolves to the same loopback family, so a client finds it.
    let client = match TcpStream.connect("localhost", address.port, timeout: .milliseconds(2000)) {
        .Ok(c) => c,
        .Err(_) => return 3
    };
    if not client.peerAddress().unwrap().ip.isLoopback { return 4 }

    let open = match TcpListener.bind(anyInterface: 0) { .Ok(l) => l, .Err(_) => return 5 };
    if open.localAddress().unwrap().ip.isLoopback { return 6 }
    0
}
