// Middleware protocol and built-in middlewares
//
// Middleware can intercept requests, modify them, add context,
// or short-circuit the response without calling the next handler.

module expressks.middleware;

import expressks.http.(Request, Response, HttpStatus, HttpMethod);

// Handler type for route handlers and middleware next function
public type Handler = (Request) -> Response;

// Middleware protocol - all middlewares must implement this
public protocol Middleware {
    // Handle the request, optionally calling next to continue the chain
    func handle(req: Request, next: Handler) -> Response;
}

// Logging middleware - logs request method and path
public struct LoggingMiddleware: Middleware {
    public init() {}

    public func handle(req: Request, next: Handler) -> Response {
        // Log would go here - for now just pass through
        // In a real implementation, this would write to a logger
        // print("[" + req.method.toString() + "] " + req.path);

        let response = next(req);

        // Log response status
        // print("Response: " + response.status().code.toString());

        response
    }
}

// CORS middleware - handles Cross-Origin Resource Sharing
public struct CorsMiddleware: Middleware {
    private let allowOrigin: String;
    private let allowMethods: String;
    private let allowHeaders: String;
    private let allowCredentials: Bool;
    private let maxAge: Int;

    public init() {
        self.allowOrigin = "*";
        self.allowMethods = "GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS";
        self.allowHeaders = "Content-Type, Authorization, X-Requested-With";
        self.allowCredentials = false;
        self.maxAge = 86400;  // 24 hours
    }

    public init(
        origin: String,
        methods: String,
        headers: String,
        credentials: Bool,
        maxAge: Int
    ) {
        self.allowOrigin = origin;
        self.allowMethods = methods;
        self.allowHeaders = headers;
        self.allowCredentials = credentials;
        self.maxAge = maxAge;
    }

    public func handle(req: Request, next: Handler) -> Response {
        // Handle preflight OPTIONS requests
        if req.method == .Options {
            return .Empty(status: HttpStatus.noContent);
            // Note: CORS headers would be added by response writer
            // Access-Control-Allow-Origin, Access-Control-Allow-Methods, etc.
        }

        // Continue to next handler
        next(req)
    }

    // Get CORS headers to add to response
    public func corsHeaders() -> (String, String, String, String, String) {
        (
            self.allowOrigin,
            self.allowMethods,
            self.allowHeaders,
            if self.allowCredentials { "true" } else { "false" },
            intToString(self.maxAge)
        )
    }
}

// Auth middleware - validates authorization header
public struct AuthMiddleware: Middleware {
    private let validateToken: (String) -> Bool;

    public init(validator: (String) -> Bool) {
        self.validateToken = validator;
    }

    public func handle(req: Request, next: Handler) -> Response {
        match req.header(name: "authorization") {
            .Some(let authHeader) => {
                // Extract token (assumes "Bearer <token>" format)
                let token = if authHeader.starts(with: "Bearer ") {
                    substringFrom(str: authHeader, start: 7)
                } else {
                    authHeader
                };

                if self.validateToken(token) {
                    // Token is valid, continue
                    next(req)
                } else {
                    // Invalid token
                    .Text(status: HttpStatus.unauthorized, body: "Invalid token")
                }
            },
            .None => {
                // No authorization header
                .Text(status: HttpStatus.unauthorized, body: "Authorization required")
            }
        }
    }
}

// Request ID middleware - adds a unique request ID to context
public struct RequestIdMiddleware: Middleware {
    private var counter: Int;

    public init() {
        self.counter = 0;
    }

    public func handle(req: Request, next: Handler) -> Response {
        self.counter += 1;
        var request = req;
        request.setContext(key: "requestId", value: intToString(self.counter));
        next(request)
    }
}

// Rate limiter middleware (simple in-memory implementation)
public struct RateLimitMiddleware: Middleware {
    private let maxRequests: Int;
    private let windowSeconds: Int;
    // In a real implementation, this would track requests per IP
    private var requestCount: Int;

    public init(maxRequests: Int, windowSeconds: Int) {
        self.maxRequests = maxRequests;
        self.windowSeconds = windowSeconds;
        self.requestCount = 0;
    }

    public func handle(req: Request, next: Handler) -> Response {
        self.requestCount += 1;

        if self.requestCount > self.maxRequests {
            return .Text(status: HttpStatus.tooManyRequests, body: "Too Many Requests");
        }

        next(req)
    }
}

// Body size limit middleware
public struct BodyLimitMiddleware: Middleware {
    private let maxBytes: Int;

    public init(maxBytes: Int) {
        self.maxBytes = maxBytes;
    }

    public func handle(req: Request, next: Handler) -> Response {
        if req.body.byteCount > self.maxBytes {
            return .Text(status: HttpStatus.badRequest, body: "Request body too large");
        }

        next(req)
    }
}

// Method override middleware - allows overriding HTTP method via header or query
public struct MethodOverrideMiddleware: Middleware {
    public init() {}

    public func handle(req: Request, next: Handler) -> Response {
        // Check X-HTTP-Method-Override header
        let override = match req.header(name: "x-http-method-override") {
            .Some(let method) => HttpMethod.parse(method),
            .None => {
                // Check _method query parameter
                match req.query("_method") {
                    .Some(let method) => HttpMethod.parse(method),
                    .None => .None
                }
            }
        };

        match override {
            .Some(let newMethod) => {
                // Create new request with overridden method
                let newReq = Request(
                    method: newMethod,
                    path: req.path,
                    headers: req.headersMap,
                    params: req.paramsMap,
                    query: req.queryMap,
                    body: req.body
                );
                next(newReq)
            },
            .None => next(req)
        }
    }
}

// Compose multiple middlewares into one
public struct ComposedMiddleware: Middleware {
    private let middlewares: Array[any Middleware];

    public init(middlewares: Array[any Middleware]) {
        self.middlewares = middlewares;
    }

    public func handle(req: Request, next: Handler) -> Response {
        if self.middlewares.count == 0 {
            return next(req);
        }

        // Build chain from last to first
        var handler = next;
        for i in 0..<self.middlewares.count {
            let idx = self.middlewares.count - 1 - i;
            let middleware = self.middlewares(unchecked: idx);
            let currentHandler = handler;
            handler = { (r) in middleware.handle(req: r, next: currentHandler) };
        }

        handler(req)
    }
}

// Helper: convert int to string (duplicated for module independence)
func intToString(value: Int) -> String {
    if value == 0 {
        return "0";
    }

    var result = String();
    var n = if value < 0 { -value } else { value };
    var digits = Array[UInt8]();

    while n > 0 {
        digits.append(((n % 10) + 48) as UInt8);
        n = n / 10;
    }

    if value < 0 {
        result.append(codePoint: CodePoint(value: 45));
    }

    for i in 0..<digits.count {
        let idx = digits.count - 1 - i;
        result.append(codePoint: CodePoint(value: digits(unchecked: idx) as UInt32));
    }

    result
}

// Helper: substring from position
func substringFrom(str: String, start: Int) -> String {
    var result = String();
    for i in start..<str.byteCount {
        result.append(codePoint: CodePoint(value: str.byteAt(index: i) as UInt32));
    }
    result
}


