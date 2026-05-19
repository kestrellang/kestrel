/// Swoop — immutable, fluent HTTP client for Kestrel.
///
/// Config methods (`.header`, `.timeout`) return new `Swoop` instances;
/// verb methods (`.fetch`, `.post`, etc.) execute requests. Supports HTTP
/// and HTTPS (via OpenSSL).
///
/// # Examples
///
/// ```
/// // One-liner
/// let res = try Swoop().fetch("http://api.example.com/users");
///
/// // Reusable client with shared config
/// let api = Swoop(baseUrl: "http://api.example.com")
///     .header("Authorization", "Bearer token123");
///
/// let users = try api.fetch("/users");
/// let user  = try api.post("/users", JsonBody(payload));
/// ```

module swoop.swoop

import http.method.(HttpMethod)
import http.headers.(Headers)
import swoop.error.(SwoopError)
import swoop.response.(Response)
import swoop.url.(ClientUrl, parseClientUrl)
import http.content.(Content)
import swoop.content.(JsonBody)
import swoop.send.(sendRequest)
import swoop.tls.(TlsStream)

// ============================================================================
// SWOOP
// ============================================================================

/// An immutable HTTP client configuration.
///
/// Config methods return new instances; verb methods execute requests.
/// All fields are private — configure via the fluent API.
///
/// # Examples
///
/// ```
/// let api = Swoop(baseUrl: "https://api.example.com")
///     .header("Accept", "application/json");
/// let res = try api.fetch("/users");
/// ```
public struct Swoop: Cloneable {
    var _baseUrl: String
    var _headers: Headers
    var _timeoutSeconds: Int64

    public init() {
        self._baseUrl = "";
        self._headers = Headers();
        self._timeoutSeconds = 30;
    }

    /// Creates a client with a base URL. Relative paths passed to verb
    /// methods are resolved against this URL.
    public init(baseUrl url: String) {
        self._baseUrl = url;
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

    /// Returns a new Swoop with a timeout.
    public func timeout(seconds: Int64) -> Swoop {
        Swoop(self._baseUrl, self._headers, seconds)
    }

    // ====================================================================
    // INSTANCE VERB METHODS
    // ====================================================================

    /// Performs an HTTP GET request.
    public func fetch(url: String) -> Result[Response, SwoopError] = self.execute(HttpMethod.Get, url)

    /// Performs an HTTP DELETE request.
    public func delete(url: String) -> Result[Response, SwoopError] = self.execute(HttpMethod.Delete, url)

    /// Performs an HTTP HEAD request.
    public func head(url: String) -> Result[Response, SwoopError] = self.execute(HttpMethod.Head, url)

    /// Performs an HTTP POST request with content.
    public func post[C](url: String, content: C) -> Result[Response, SwoopError] where C: Content = self.executeWith(HttpMethod.Post, url, content)

    /// Performs an HTTP PUT request with content.
    public func put[C](url: String, content: C) -> Result[Response, SwoopError] where C: Content = self.executeWith(HttpMethod.Put, url, content)

    /// Performs an HTTP PATCH request with content.
    public func patch[C](url: String, content: C) -> Result[Response, SwoopError] where C: Content = self.executeWith(HttpMethod.Patch, url, content)

    // ====================================================================
    // EXECUTION (no content)
    // ====================================================================

    func execute(method: HttpMethod, url: String) -> Result[Response, SwoopError] {
        let fullUrl = if url.starts(with: "http://") or url.starts(with: "https://") {
            url
        } else {
            self._baseUrl + url
        };

        let parsed = try parseClientUrl(fullUrl);

        if parsed.scheme == "https" {
            let tlsStream = match TlsStream.connect(parsed.host, parsed.port) {
                .Ok(s) => s,
                .Err(e) => return .Err(SwoopError.connectionFailed("TLS connection failed to " + parsed.host))
            };
            sendRequest(tlsStream, method, parsed, self._headers)
        } else {
            let stream = match TcpStream.connect(parsed.host, parsed.port) {
                .Ok(s) => s,
                .Err(e) => return .Err(SwoopError.connectionFailed("could not connect to " + parsed.host))
            };
            sendRequest(stream, method, parsed, self._headers)
        }
    }

    // ====================================================================
    // EXECUTION (with content)
    // ====================================================================

    func executeWith[C](method: HttpMethod, url: String, content: C) -> Result[Response, SwoopError] where C: Content {
        let fullUrl = if url.starts(with: "http://") or url.starts(with: "https://") {
            url
        } else {
            self._baseUrl + url
        };

        let parsed = try parseClientUrl(fullUrl);

        if parsed.scheme == "https" {
            let tlsStream = match TlsStream.connect(parsed.host, parsed.port) {
                .Ok(s) => s,
                .Err(e) => return .Err(SwoopError.connectionFailed("TLS connection failed to " + parsed.host))
            };
            sendRequest(tlsStream, method, parsed, self._headers, content)
        } else {
            let stream = match TcpStream.connect(parsed.host, parsed.port) {
                .Ok(s) => s,
                .Err(e) => return .Err(SwoopError.connectionFailed("could not connect to " + parsed.host))
            };
            sendRequest(stream, method, parsed, self._headers, content)
        }
    }
}
