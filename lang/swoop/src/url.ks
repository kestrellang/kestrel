// Client-side URL parsing
//
// Parses full URLs like "http://host:port/path?query" into components.
// The existing http.url module only handles request paths (/path?query).

module swoop.url

import swoop.error.(SwoopError)
import http.wire.(parseDecimal)

// ============================================================================
// CLIENT URL
// ============================================================================

/// A parsed client URL with scheme, host, port, path, and query string.
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
            self.path + "?" + self.queryString
        } else {
            self.path
        }
    }

    public func clone() -> ClientUrl {
        var c = ClientUrl(self.scheme.clone(), self.host.clone(), self.port, self.path.clone(), self.queryString.clone());
        c
    }

    /// Returns "host" or "host:port" for the Host header.
    public func hostHeader() -> String {
        let defaultPort: UInt16 = 80;
        if self.port == defaultPort {
            self.host
        } else {
            self.host + ":" + Int64(from: self.port).format()
        }
    }
}

// ============================================================================
// PARSER
// ============================================================================

/// Parses a URL string into a ClientUrl.
///
/// Supports: http://host/path, http://host:port/path, http://host:port/path?query
/// Does not support HTTPS (v1).
public func parseClientUrl(raw: String) -> Result[ClientUrl, SwoopError] {
    let len = raw.byteCount;

    // Check for "http://" prefix
    if not raw.starts(with: "http://") {
        return .Err(SwoopError.invalidUrl("only http:// URLs are supported"))
    }

    // Skip "http://"
    let afterScheme: Int64 = 7;

    // Find end of host:port (first '/' after scheme, or end of string)
    var pathStart = len;
    var si = afterScheme;
    while si < len {
        if raw.byteAtUnchecked(si) == 47 { // '/'
            pathStart = si;
            break
        }
        si = si + 1
    }

    // Extract host:port portion
    let hostPort = raw.substringBytes(from: afterScheme, to: pathStart);
    if hostPort.byteCount == 0 {
        return .Err(SwoopError.invalidUrl("missing host"))
    }

    // Split host and port on ':'
    var host = hostPort;
    var port: UInt16 = 80;
    match hostPort.find(":") {
        .Some(colonIdx) => {
            host = hostPort.substringBytes(from: 0, to: colonIdx);
            let portStr = hostPort.substringBytes(from: colonIdx + 1, to: hostPort.byteCount);
            let port64 = parseDecimal(portStr);
            if port64 > 0 and port64 <= 65535 {
                port = UInt16(from: port64)
            } else {
                return .Err(SwoopError.invalidUrl("invalid port"))
            }
        },
        .None => {}
    }

    // Extract path and query string
    var path = "/";
    var queryString = String();
    if pathStart < len {
        let remainder = raw.substringBytes(from: pathStart, to: len);
        match remainder.find("?") {
            .Some(qIdx) => {
                path = remainder.substringBytes(from: 0, to: qIdx);
                queryString = remainder.substringBytes(from: qIdx + 1, to: remainder.byteCount)
            },
            .None => {
                path = remainder
            }
        }
    }

    .Ok(ClientUrl("http", host, port, path, queryString))
}

// parseDecimal imported from http.wire
