/// Client-side URL parsing — full URLs like `http://host:port/path?query`.
///
/// The `http.url` module handles request paths (`/path?query`); this
/// module adds scheme, host, and port for client-side use.

module swoop.url

import swoop.error.(SwoopError)
import http.wire.(parseDecimal)

// ============================================================================
// CLIENT URL
// ============================================================================

/// A parsed client URL with scheme, host, port, path, and query string.
///
/// # Examples
///
/// ```
/// let url = try parseClientUrl("https://api.example.com:8443/v1/users?page=1");
/// url.scheme;       // "https"
/// url.host;         // "api.example.com"
/// url.port;         // 8443
/// url.path;         // "/v1/users"
/// url.queryString;  // "page=1"
/// ```
public struct ClientUrl: Cloneable {
    public var scheme: String
    public var host: String
    public var port: UInt16
    public var path: String
    public var queryString: String

    public init(scheme: String, host: String, port: UInt16, path: String, queryString: String) {
        self.scheme = scheme;
        self.host = host;
        self.port = port;
        self.path = path;
        self.queryString = queryString;
    }

    /// Returns the full request path including query string (e.g. "/users?page=1").
    public func requestPath() -> String {
        if self.queryString.byteCount > 0 {
            var full = String();
            full.append(self.path);
            full.append("?");
            full.append(self.queryString);
            full
        } else {
            self.path
        }
    }

    public func clone() -> ClientUrl {
        ClientUrl(self.scheme.clone(), self.host.clone(), self.port, self.path.clone(), self.queryString.clone())
    }

    /// Returns "host" or "host:port" for the Host header.
    /// Omits port when it matches the scheme default (80 for http, 443 for https).
    public func hostHeader() -> String {
        let httpDefault: UInt16 = 80;
        let httpsDefault: UInt16 = 443;
        if self.port == httpDefault or self.port == httpsDefault {
            self.host
        } else {
            var hostHeader = String();
            hostHeader.append(self.host);
            hostHeader.append(":");
            hostHeader.append(Int64(from: self.port).format());
            hostHeader
        }
    }
}

// ============================================================================
// PARSER
// ============================================================================

/// Parses a URL string into a ClientUrl.
///
/// Supports: http://host/path, https://host/path, with optional :port and ?query
public func parseClientUrl(raw: String) -> Result[ClientUrl, SwoopError] {
    let len = raw.byteCount;

    // Determine scheme
    var scheme = String();
    var afterScheme: Int64 = 0;
    var defaultPort: UInt16 = 80;

    if raw.starts(with: "https://") {
        scheme = "https";
        afterScheme = 8;
        defaultPort = 443
    } else if raw.starts(with: "http://") {
        scheme = "http";
        afterScheme = 7;
        defaultPort = 80
    } else {
        return .Err(SwoopError.invalidUrl("only http:// and https:// URLs are supported"))
    }

    // Find end of host:port (first '/' after scheme, or end of string)
    var pathStart = len;
    var si = afterScheme;
    while si < len {
        if raw.bytes(unchecked: si) == 47 { // '/'
            pathStart = si;
            break
        }
        si = si + 1
    }

    // Extract host:port portion
    let rawSlice = raw.asSlice();
    let hostPort = rawSlice.subslice(from: afterScheme, to: pathStart).toOwned();
    if hostPort.byteCount == 0 {
        return .Err(SwoopError.invalidUrl("missing host"))
    }

    var host = hostPort;
    var port = defaultPort;
    let hpSlice = hostPort.asSlice();
    if let .Some(colonIdx) = hostPort.firstIndex(of: ":") {
        host = hpSlice.subslice(from: hpSlice.start, to: colonIdx.value).toOwned();
        let portStr = hpSlice.subslice(from: colonIdx.value + 1, to: hpSlice.end).toOwned();
        let port64 = parseDecimal(portStr);
        if port64 > 0 and port64 <= 65535 {
            port = UInt16(from: port64)
        } else {
            return .Err(SwoopError.invalidUrl("invalid port"))
        }
    }

    var path = "/";
    var queryString = String();
    if pathStart < len {
        let remainder = rawSlice.subslice(from: pathStart, to: len).toOwned();
        let remSlice = remainder.asSlice();
        if let .Some(qIdx) = remainder.firstIndex(of: "?") {
            path = remSlice.subslice(from: remSlice.start, to: qIdx.value).toOwned();
            queryString = remSlice.subslice(from: qIdx.value + 1, to: remSlice.end).toOwned()
        } else {
            path = remainder
        }
    }

    .Ok(ClientUrl(scheme, host, port, path, queryString))
}
