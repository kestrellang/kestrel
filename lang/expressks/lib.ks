module expressks

public import expressks.http.(Request, Response, Html, Json, Text)
public import expressks.router.(Router)
public import expressks.middleware.(Middleware)
public import expressks.server.(Server)

public func createApp() -> App {
    App()
}

public struct App {
    public let router: Router
    
    public init() {
        self.router = Router()
    }
    
    public func get(path: String, handler: (Request) -> Response) {
        self.router.get(path: path, handler: handler)
    }
    
    public func post(path: String, handler: (Request) -> Response) {
        self.router.post(path: path, handler: handler)
    }
    
    public func listen(port: Int) {
        let server = Server(router: self.router)
        server.listen(port: port)
    }
}

