// TCP, UDP and Unix sockets, IP addresses and name lookup.
//
// Sockets are streams: `TcpStream` and `UnixStream` are `Readable` and
// `Writable`, so `readAll`, `writeAll`, `buffered()` and the rest work on
// them. Every socket is closed when dropped, created close-on-exec, and
// never raises SIGPIPE: writing to a closed peer is a `BrokenPipe` error.
// Calls that need a `sockaddr` go through `network_shims.c`.
//
// # Examples
//
// ```
// var stream = try TcpStream.connect("example.com", 80);
// try stream.write(text: "HEAD / HTTP/1.0\r\n\r\n");
// let reply = try stream.readText();
//
// let listener = try TcpListener.bind(port: 8080);
// for client in listener.incoming() {
//     var client = try client;
//     try client.write(line: "hello");
// }
// ```

module std.network
