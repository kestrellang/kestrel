// HTTP Server - socket-based HTTP/1.1 server
//
// This module provides the Server struct that binds to a port and
// handles incoming HTTP connections.

module expressks.server;

import std.collections.dictionary;
import std.collections.array;
import std.memory.pointer;
import std.memory.buffer;
import expressks.http.(Request, Response, HttpStatus);
import expressks.router.Router;
import expressks.internal.socket.(
    AF_INET, SOCK_STREAM, IPPROTO_TCP, SOL_SOCKET, SO_REUSEADDR,
    SockAddrIn, socket, bind, listen, accept, setsockopt,
    readSocket, writeSocket, close
);
import expressks.internal.parser.(parseHttpRequest, formatHttpResponse);

// Server configuration
public struct ServerConfig {
    public let port: Int;
    public let host: String;
    public let backlog: Int;
    public let maxRequestSize: Int;

    public init(port: Int) {
        self.port = port;
        self.host = "0.0.0.0";
        self.backlog = 128;
        self.maxRequestSize = 8192;
    }

    public init(port: Int, host: String, backlog: Int, maxRequestSize: Int) {
        self.port = port;
        self.host = host;
        self.backlog = backlog;
        self.maxRequestSize = maxRequestSize;
    }
}

// The HTTP Server
public struct Server {
    private let router: Router;
    private var socketFd: Int32;
    private let config: ServerConfig;
    private var running: Bool;

    public init(router: Router) {
        self.router = router;
        self.socketFd = -1;
        self.config = ServerConfig(port: 8080);
        self.running = false;
    }

    public init(router: Router, config: ServerConfig) {
        self.router = router;
        self.socketFd = -1;
        self.config = config;
        self.running = false;
    }

    // Start the server on the specified port
    public func listen(port: Int) {
        // Create socket
        self.socketFd = socket(domain: AF_INET, type: SOCK_STREAM, protocol: IPPROTO_TCP);

        if self.socketFd < 0 {
            // Failed to create socket
            return;
        }

        // Set SO_REUSEADDR to allow quick restart
        var optval: Int32 = 1;
        setsockopt(
            sockfd: self.socketFd,
            level: SOL_SOCKET,
            optname: SO_REUSEADDR,
            optval: Pointer(to: ref optval),
            optlen: 4
        );

        // Bind to address
        var addr = SockAddrIn.any(port: port as UInt16);
        let bindResult = bind(
            sockfd: self.socketFd,
            addr: Pointer(to: ref addr),
            addrlen: 16
        );

        if bindResult < 0 {
            close(fd: self.socketFd);
            self.socketFd = -1;
            return;
        }

        // Start listening
        let listenResult = listen(sockfd: self.socketFd, backlog: self.config.backlog as Int32);
        if listenResult < 0 {
            close(fd: self.socketFd);
            self.socketFd = -1;
            return;
        }

        self.running = true;

        // Accept loop
        self.acceptLoop();
    }

    // Main accept loop
    private func acceptLoop() {
        var clientAddr = SockAddrIn(port: 0, addr: 0);
        var addrLen: UInt32 = 16;

        while self.running {
            // Accept new connection
            let clientFd = accept(
                sockfd: self.socketFd,
                addr: Pointer(to: ref clientAddr),
                addrlen: Pointer(to: ref addrLen)
            );

            if clientFd < 0 {
                // Accept failed, try again
                continue;
            }

            // Handle the client
            self.handleClient(fd: clientFd);
        }
    }

    // Handle a single client connection
    private func handleClient(fd: Int32) {
        // Read request
        var buffer = Buffer[UInt8](capacity: self.config.maxRequestSize);
        let bytesRead = readSocket(
            fd: fd,
            buf: buffer.pointer(),
            count: buffer.capacity()
        );

        if bytesRead <= 0 {
            close(fd: fd);
            return;
        }

        // Convert bytes to string
        let requestStr = bytesToString(buffer: buffer, length: bytesRead);

        // Parse request
        let response = match parseHttpRequest(raw: requestStr) {
            .Ok(let request) => {
                // Route the request
                self.router.handle(req: request)
            },
            .Err(let err) => {
                // Parse error
                Response.Text(status: HttpStatus.badRequest, body: "Bad Request: " + err.message)
            }
        };

        // Format and send response
        let responseStr = self.formatResponse(response: response);
        let responseBytes = stringToBytes(str: responseStr);

        writeSocket(
            fd: fd,
            buf: responseBytes.pointer(),
            count: responseBytes.count()
        );

        // Close connection
        close(fd: fd);
    }

    // Format a Response into HTTP response string
    private func formatResponse(response: Response) -> String {
        var extraHeaders = Dictionary[String, String]();

        match response {
            .Html(let status, let body) => {
                formatHttpResponse(
                    statusCode: status.code,
                    statusMessage: status.message,
                    contentType: "text/html; charset=utf-8",
                    body: body,
                    extraHeaders: extraHeaders
                )
            },
            .Json(let status, let body) => {
                formatHttpResponse(
                    statusCode: status.code,
                    statusMessage: status.message,
                    contentType: "application/json",
                    body: body,
                    extraHeaders: extraHeaders
                )
            },
            .Text(let status, let body) => {
                formatHttpResponse(
                    statusCode: status.code,
                    statusMessage: status.message,
                    contentType: "text/plain; charset=utf-8",
                    body: body,
                    extraHeaders: extraHeaders
                )
            },
            .Empty(let status) => {
                formatHttpResponse(
                    statusCode: status.code,
                    statusMessage: status.message,
                    contentType: "",
                    body: "",
                    extraHeaders: extraHeaders
                )
            },
            .Redirect(let url, let status) => {
                extraHeaders.insert(value: url, for: "Location");
                formatHttpResponse(
                    statusCode: status.code,
                    statusMessage: status.message,
                    contentType: "",
                    body: "",
                    extraHeaders: extraHeaders
                )
            }
        }
    }

    // Stop the server
    public func stop() {
        self.running = false;
        if self.socketFd >= 0 {
            close(fd: self.socketFd);
            self.socketFd = -1;
        }
    }

    // Check if server is running
    public func isRunning() -> Bool {
        self.running
    }
}

// Helper: convert buffer bytes to string
func bytesToString(buffer: Buffer[UInt8], length: Int) -> String {
    var result = String();
    for i in 0..<length {
        result.append(codePoint: CodePoint(value: buffer.get(index: i) as UInt32));
    }
    result
}

// Helper: convert string to byte buffer
func stringToBytes(str: String) -> Buffer[UInt8] {
    var buffer = Buffer[UInt8](capacity: str.byteCount);
    for i in 0..<str.byteCount {
        buffer.set(index: i, value: str.byteAt(index: i));
    }
    buffer
}
