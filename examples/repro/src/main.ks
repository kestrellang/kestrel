// Instrumented: trace all TcpStream.deinit calls

module repro.main

import http.wire.(stringToBytes)
import std.io.error.(IoError)

func serve(port: UInt16) -> Result[(), IoError] {
    var listener = try TcpListener.bind(port);
    loop {
        println("=== calling accept ===");
        var stream = try listener.accept();
        let fd = stream.rawFd();
        println("accepted fd=\(fd)");
        var buf = Array[UInt8](repeating: 0, count: 1024);
        let slice = ArraySlice(pointer: buf.asPointer(), count: 1024);
        let bytesRead = recv(fd, slice.pointer, slice.count, 0);
        println("recv=\(bytesRead)");
        let resp = stringToBytes("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK");
         send(fd, resp.asPointer(), resp.count, 0);
        println("sent response");
    }
}

func main() {
    match serve(8090) {
        .Ok(_) => {},
        .Err(e) => {}
    }
}
