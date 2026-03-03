// Swoop — HTTP client for Kestrel
//
// Swoop is an immutable config object. Every config method returns a new Swoop.
// Verb methods (.fetch, .post, etc.) execute the request.
//
// Usage:
//   // One-liner
//   let res = try Swoop().fetch("http://api.example.com/users")
//
//   // With config
//   let res = try Swoop()
//       .header("Authorization", "Bearer token123")
//       .header("Accept", "application/json")
//       .fetch("http://api.example.com/users")
//
//   // Reusable client
//   let api = Swoop()
//       .baseUrl("http://api.example.com")
//       .header("Authorization", "Bearer token123")
//       .timeout(seconds: 30)
//
//   let users = try api.fetch("/users")
//   let user  = try api.post("/users", body: .Text(payload))

module swoop.swoop

import http.method.(HttpMethod)
import http.headers.(Headers)
import swoop.error.(SwoopError)
import swoop.response.(Response)
import swoop.url.(ClientUrl, parseClientUrl)
import swoop.body.(Body)
import swoop.send.(sendRequest)
import swoop.tls.(TlsStream)

// ============================================================================
// SWOOP
// ============================================================================

/// An immutable HTTP client configuration. Config methods return new instances;
/// verb methods execute requests.
public struct Swoop: Cloneable {
    var _baseUrl: String
    var _headers: Headers
    var _timeoutSeconds: Int64

    public init() {
        self._baseUrl = "";
        self._headers = Headers();
        self._timeoutSeconds = 30;
    }

    init(baseUrl: String, headers: Headers, timeoutSeconds: Int64) {
        self._baseUrl = baseUrl;
        self._headers = headers;
        self._timeoutSeconds = timeoutSeconds;
    }

    public func clone() -> Swoop {
        Swoop(self._baseUrl.clone(), self._headers.clone(), self._timeoutSeconds)
    }

    // ====================================================================
    // CONFIG (returns new Swoop)
    // ====================================================================

    /// Returns a new Swoop with an additional header.
    public func header(name: String, value: String) -> Swoop {
        var newHeaders = self._headers;
        newHeaders.add(name, value);
        Swoop(self._baseUrl, newHeaders, self._timeoutSeconds)
    }

    /// Returns a new Swoop with a base URL.
    public func baseUrl(url: String) -> Swoop {
        Swoop(url, self._headers, self._timeoutSeconds)
    }

    /// Returns a new Swoop with a timeout.
    public func timeout(seconds: Int64) -> Swoop {
        Swoop(self._baseUrl, self._headers, seconds)
    }

    // ====================================================================
    // VERB METHODS
    // ====================================================================

    /// Performs an HTTP GET request.
    public func fetch(url: String) -> Result[Response, SwoopError] {
        self.execute(HttpMethod.Get, url, null)
    }

    /// Performs an HTTP DELETE request.
    public func delete(url: String) -> Result[Response, SwoopError] {
        self.execute(HttpMethod.Delete, url, null)
    }

    /// Performs an HTTP HEAD request.
    public func head(url: String) -> Result[Response, SwoopError] {
        self.execute(HttpMethod.Head, url, null)
    }

    /// Performs an HTTP POST request with a body.
    public func post(url: String, body: Body) -> Result[Response, SwoopError] {
        self.execute(HttpMethod.Post, url, .Some(body))
    }

    /// Performs an HTTP PUT request with a body.
    public func put(url: String, body: Body) -> Result[Response, SwoopError] {
        self.execute(HttpMethod.Put, url, .Some(body))
    }

    /// Performs an HTTP PATCH request with a body.
    public func patch(url: String, body: Body) -> Result[Response, SwoopError] {
        self.execute(HttpMethod.Patch, url, .Some(body))
    }

    // ====================================================================
    // EXECUTION
    // ====================================================================

    /// Resolves the full URL (prepending baseUrl if needed) and executes the request.
    func execute(method: HttpMethod, url: String, body: Body?) -> Result[Response, SwoopError] {
        // Resolve URL: if it starts with http:// or https://, use as-is; otherwise prepend baseUrl
        let fullUrl = if url.starts(with: "http://") or url.starts(with: "https://") {
            url
        } else {
            self._baseUrl + url
        };

        // Parse the URL
        let parsed = try parseClientUrl(fullUrl);

        // Connect and send based on scheme
        if parsed.scheme == "https" {
            let tlsStream = match TlsStream.connect(parsed.host, parsed.port) {
                .Ok(s) => s,
                .Err(e) => return .Err(SwoopError.connectionFailed("TLS connection failed to " + parsed.host))
            };
            sendRequest(tlsStream, method, parsed, self._headers, body)
        } else {
            let stream = match TcpStream.connect(parsed.host, parsed.port) {
                .Ok(s) => s,
                .Err(e) => return .Err(SwoopError.connectionFailed("could not connect to " + parsed.host))
            };
            sendRequest(stream, method, parsed, self._headers, body)
        }
    }
}
