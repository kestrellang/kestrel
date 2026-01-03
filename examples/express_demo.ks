// ExpressKS Demo
//
// Demonstrates the ExpressKS web framework with:
// - Route handlers with trailing closure syntax
// - Path parameters (:id)
// - Query string parsing
// - Generic JSON serialization
// - Middleware usage

import expressks;
import std.collections.dictionary;
import std.serde.Serialize;

// Example: a serializable struct for JSON responses
struct User: Serialize {
    let id: Int;
    let name: String;
    let email: String;

    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        let obj = try serializer.beginObject(name: "User", fieldCount: 3);
        try obj.serializeField(name: "id", value: self.id);
        try obj.serializeField(name: "name", value: self.name);
        try obj.serializeField(name: "email", value: self.email);
        obj.end()
    }
}

struct StatusResponse: Serialize {
    let status: String;
    let version: String;
    let uptime: Int;

    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        let obj = try serializer.beginObject(name: "StatusResponse", fieldCount: 3);
        try obj.serializeField(name: "status", value: self.status);
        try obj.serializeField(name: "version", value: self.version);
        try obj.serializeField(name: "uptime", value: self.uptime);
        obj.end()
    }
}

// Mock database of users
func getUser(id: String) -> Optional[User] {
    if id == "1" {
        .Some(User(id: 1, name: "Alice", email: "alice@example.com"))
    } else if id == "2" {
        .Some(User(id: 2, name: "Bob", email: "bob@example.com"))
    } else {
        .None
    }
}

func main() {
    let app = expressks.createApp();

    // Global middleware
    app.use(expressks.LoggingMiddleware());
    app.use(expressks.CorsMiddleware());

    // Home page - HTML response
    app.get(path: "/") { (req) in
        expressks.Html("<html><body><h1>Welcome to ExpressKS!</h1><p>A Kestrel web framework.</p></body></html>")
    };

    // About page
    app.get(path: "/about") { (req) in
        expressks.Html("<html><body><h1>About</h1><p>ExpressKS is an Express.js-like web framework for Kestrel.</p></body></html>")
    };

    // API status - JSON response with Serialize
    app.get(path: "/api/status") { (req) in
        let status = StatusResponse(
            status: "ok",
            version: "1.0.0",
            uptime: 12345
        );
        expressks.Json(status)
    };

    // Get user by ID - path parameter
    app.get(path: "/api/users/:id") { (req) in
        match req.params("id") {
            .Some(let userId) => {
                match getUser(id: userId) {
                    .Some(let user) => expressks.Json(user),
                    .None => expressks.NotFound(message: "User not found")
                }
            },
            .None => expressks.BadRequest(message: "Missing user ID")
        }
    };

    // Search with query parameters
    app.get(path: "/api/search") { (req) in
        let query = req.query("q").unwrap(or: "");
        let page = req.query("page").unwrap(or: "1");

        // Build response with search info
        var result = Dictionary[String, String]();
        result.insert(value: query, for: "query");
        result.insert(value: page, for: "page");
        result.insert(value: "10", for: "results");

        expressks.JsonRaw("{\"query\":\"" + query + "\",\"page\":" + page + ",\"results\":10}")
    };

    // POST endpoint
    app.post(path: "/api/users") { (req) in
        // In a real app, parse the body and create a user
        expressks.Json(status: expressks.HttpStatus.created, value: User(
            id: 3,
            name: "New User",
            email: "new@example.com"
        ))
    };

    // PUT endpoint - update user
    app.put(path: "/api/users/:id") { (req) in
        match req.params("id") {
            .Some(let userId) => {
                // In a real app, update the user
                expressks.Json(User(id: 1, name: "Updated User", email: "updated@example.com"))
            },
            .None => expressks.BadRequest(message: "Missing user ID")
        }
    };

    // DELETE endpoint
    app.delete(path: "/api/users/:id") { (req) in
        match req.params("id") {
            .Some(let userId) => {
                // In a real app, delete the user
                expressks.NoContent()
            },
            .None => expressks.BadRequest(message: "Missing user ID")
        }
    };

    // Protected route with auth middleware
    let authValidator = { (token) in token == "secret-token" };
    app.use(path: "/api/admin", expressks.AuthMiddleware(validator: authValidator));

    app.get(path: "/api/admin/dashboard") { (req) in
        expressks.Html("<h1>Admin Dashboard</h1><p>Welcome, admin!</p>")
    };

    // Redirect example
    app.get(path: "/old-path") { (req) in
        expressks.Redirect(to: "/new-path")
    };

    app.get(path: "/new-path") { (req) in
        expressks.Text("You were redirected here!")
    };

    // Start the server
    app.listen(port: 8080);
}
