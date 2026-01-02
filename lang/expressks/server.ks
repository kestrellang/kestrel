module expressks.server

import expressks.http
import expressks.router
import std.memory.buffer
import std.text.char

// Abstracted C externs for networking
@extern(.c)
func socket(domain: Int32, type: Int32, protocol: Int32) -> Int32 {}

@extern(.c)
func bind(sockfd: Int32, addr: Int32, addrlen: Int32) -> Int32 {} // Simplified signature

@extern(.c)
func listen(sockfd: Int32, backlog: Int32) -> Int32 {}

@extern(.c)
func accept(sockfd: Int32, addr: Int32, addrlen: Int32) -> Int32 {} // Simplified signature

@extern(.c, mangleName: "read")
func read_socket(fd: Int32, buf: Buffer[Byte], count: Int32) -> Int32 {}

@extern(.c, mangleName: "write")
func write_socket(fd: Int32, buf: Buffer[Byte], count: Int32) -> Int32 {}

@extern(.c)
func close(fd: Int32) -> Int32 {}


public struct Server {
    let router: Router
    
    public init(router: Router) {
        self.router = router
    }

    public func listen(port: Int) {
        // AF_INET = 2, SOCK_STREAM = 1, IPPROTO_TCP = 6
        let fd = socket(domain: 2, type: 1, protocol: 6)
        
        // TODO: bind address struct handling
        // bind(fd, ...)
        
        // listen(fd, 10)
        
        // Mock loop for now since we can't really accept without struct definitions
        // loop {
        //    let clientFd = accept(fd, 0, 0)
        //    self.handleClient(clientFd)
        // }
    }
    
    private func handleClient(fd: Int32) {
        // Read request
        // Parse request (Mocked)
        let req = Request(method: "GET", path: "/", headers: Dictionary())
        
        let res = self.router.handle(req: req)
        
        // Write response
        match res {
            .Html(let body) => {
                // write HTTP headers + body
            },
            .Json(let body) => {
                // write HTTP headers + body
            },
            .Text(let body) => {
                // write HTTP headers + body
            },
            .Empty => {}
        }
        
        close(fd: fd)
    }
}
