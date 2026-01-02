module expressks.middleware

import expressks.http

public protocol Middleware {
    func handle(req: Request, next: (Request) -> Response) -> Response
}

