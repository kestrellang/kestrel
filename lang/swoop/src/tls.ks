// TLS stream using OpenSSL/LibreSSL
//
// Provides TlsStream, a TLS-encrypted TCP connection that implements
// the Read and Write protocols. Uses OpenSSL 3.x (homebrew) or LibreSSL.

module swoop.tls

// ============================================================================
// OPENSSL C BINDINGS
// ============================================================================

// OPENSSL_init_ssl(opts, settings) — OpenSSL 3.x init (replaces SSL_library_init)
@extern(.C, mangleName: "OPENSSL_init_ssl")
func libc_OPENSSL_init_ssl(opts: lang.i64, settings: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "TLS_client_method")
func libc_TLS_client_method() -> lang.ptr[lang.i8]

@extern(.C, mangleName: "SSL_CTX_new")
func libc_SSL_CTX_new(method: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "SSL_CTX_free")
func libc_SSL_CTX_free(ctx: lang.ptr[lang.i8])

@extern(.C, mangleName: "SSL_CTX_set_default_verify_paths")
func libc_SSL_CTX_set_default_verify_paths(ctx: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "SSL_CTX_set_verify")
func libc_SSL_CTX_set_verify(ctx: lang.ptr[lang.i8], mode: lang.i32, cb: lang.ptr[lang.i8])

@extern(.C, mangleName: "SSL_new")
func libc_SSL_new(ctx: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "SSL_free")
func libc_SSL_free(ssl: lang.ptr[lang.i8])

@extern(.C, mangleName: "SSL_set_fd")
func libc_SSL_set_fd(ssl: lang.ptr[lang.i8], fd: lang.i32) -> lang.i32

@extern(.C, mangleName: "SSL_ctrl")
func libc_SSL_ctrl(ssl: lang.ptr[lang.i8], cmd: lang.i32, larg: lang.i64, parg: lang.ptr[lang.i8]) -> lang.i64

@extern(.C, mangleName: "SSL_connect")
func libc_SSL_connect(ssl: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "SSL_read")
func libc_SSL_read(ssl: lang.ptr[lang.i8], buf: lang.ptr[lang.i8], num: lang.i32) -> lang.i32

@extern(.C, mangleName: "SSL_write")
func libc_SSL_write(ssl: lang.ptr[lang.i8], buf: lang.ptr[lang.i8], num: lang.i32) -> lang.i32

@extern(.C, mangleName: "SSL_shutdown")
func libc_SSL_shutdown(ssl: lang.ptr[lang.i8]) -> lang.i32

@extern(.C, mangleName: "close")
func posix_close(fd: lang.i32) -> lang.i32

// ============================================================================
// CONSTANTS
// ============================================================================

func SSL_VERIFY_PEER() -> Int32 { 1 }
func SSL_CTRL_SET_TLSEXT_HOSTNAME() -> Int32 { 55 }

// ============================================================================
// TLS STREAM
// ============================================================================

/// A TLS-encrypted TCP stream that implements Read and Write.
public struct TlsStream: Read, Write {
    private var ssl: lang.ptr[lang.i8]
    private var ctx: lang.ptr[lang.i8]
    var fd: Int32

    init(ssl: lang.ptr[lang.i8], ctx: lang.ptr[lang.i8], fd: Int32) {
        self.ssl = ssl;
        self.ctx = ctx;
        self.fd = fd;
    }

    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        let count32 = if buf.count > 2147483647 { 2147483647 } else { Int32(from: buf.count) };
        let n = Int32(raw: libc_SSL_read(
            self.ssl,
            lang.cast_ptr[_, lang.i8](buf.pointer.raw),
            count32.raw
        ));
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(Int64(from: n))
    }

    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let count32 = if buf.count > 2147483647 { 2147483647 } else { Int32(from: buf.count) };
        let n = Int32(raw: libc_SSL_write(
            self.ssl,
            lang.cast_ptr[_, lang.i8](buf.pointer.raw),
            count32.raw
        ));
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(Int64(from: n))
    }

    public mutating func flush() -> Result[(), Error] {
        .Ok(())
    }

    deinit {
        // SSL_free and SSL_CTX_free are no-ops on null in LibreSSL
        let _ = libc_SSL_shutdown(self.ssl);
        libc_SSL_free(self.ssl);
        libc_SSL_CTX_free(self.ctx);
        if self.fd >= 0 {
            let _ = posix_close(self.fd.raw);
        }
    }
}

// ============================================================================
// CONNECT
// ============================================================================

extend TlsStream {
    /// Connects to a remote host over TLS, returning a TlsStream.
    public static func connect(host: String, port: UInt16) -> Result[TlsStream, Error] {
        // One-time init (safe to call multiple times)
        // OPENSSL_INIT_LOAD_SSL_STRINGS (0x00200000) | OPENSSL_INIT_LOAD_CRYPTO_STRINGS (0x00000002)
        let initOpts: Int64 = 2097154;
        let _ = libc_OPENSSL_init_ssl(initOpts.raw, lang.ptr_null[lang.i8]());

        // TCP connect, then take ownership of the fd
        var tcpStream = try TcpStream.connect(host, port);
        let fd = tcpStream.detachFd();

        // Create SSL_CTX
        let method = libc_TLS_client_method();
        let ctx = libc_SSL_CTX_new(method);
        if lang.ptr_is_null(ctx) {
            let _ = posix_close(fd.raw);
            return .Err(Error(1))
        }

        // Load system CA certificates and enable peer verification
        let _ = libc_SSL_CTX_set_default_verify_paths(ctx);
        let verifyMode = SSL_VERIFY_PEER();
        libc_SSL_CTX_set_verify(ctx, verifyMode.raw, lang.ptr_null[lang.i8]());

        // Create SSL
        let ssl = libc_SSL_new(ctx);
        if lang.ptr_is_null(ssl) {
            libc_SSL_CTX_free(ctx);
            let _ = posix_close(fd.raw);
            return .Err(Error(2))
        }

        // Attach socket fd
        let _ = libc_SSL_set_fd(ssl, fd.raw);

        // Set SNI hostname
        var hostBuf = Array[UInt8]();
        var i: Int64 = 0;
        while i < host.byteCount {
            hostBuf.append(host.byteAtUnchecked(i));
            i = i + 1
        }
        hostBuf.append(0);
        let _ = libc_SSL_ctrl(
            ssl,
            SSL_CTRL_SET_TLSEXT_HOSTNAME().raw,
            0,
            lang.cast_ptr[_, lang.i8](hostBuf.asPointer().raw)
        );

        // TLS handshake
        let connectResult = Int32(raw: libc_SSL_connect(ssl));
        if connectResult != 1 {
            libc_SSL_free(ssl);
            libc_SSL_CTX_free(ctx);
            let _ = posix_close(fd.raw);
            return .Err(Error(connectResult))
        }

        .Ok(TlsStream(ssl, ctx, fd))
    }
}
