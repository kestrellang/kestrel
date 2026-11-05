// The process-wide buffers behind std.streams' standard input and output.
//
// They live in C so every StandardInput / StandardOutput value shares one
// buffer (two values can't interleave half-lines), and so the output buffer
// is flushed by `atexit` however the process ends normally: returning from
// main, or std.system's `exit`.

#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define KESTREL_STREAM_CAPACITY 8192

static unsigned char out_buffer[KESTREL_STREAM_CAPACITY];
static int64_t out_count = 0;
// -1 not yet known, 0 block-buffered, 1 line-buffered (a terminal).
static int out_line_mode = -1;

static unsigned char in_buffer[KESTREL_STREAM_CAPACITY];
static int64_t in_start = 0;
static int64_t in_end = 0;

// Writes all of `bytes` to `fd`, retrying EINTR. 0 or -1 with errno set.
static int write_all(int fd, const unsigned char *bytes, int64_t count) {
    while (count > 0) {
        ssize_t n = write(fd, bytes, (size_t)count);
        if (n < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        bytes += n;
        count -= n;
    }
    return 0;
}

// Writes the output buffer to fd 1. 0, or -1 with errno set.
int64_t kestrel_stdout_flush(void) {
    int64_t count = out_count;
    out_count = 0;
    return write_all(1, out_buffer, count);
}

// Buffers `count` bytes for fd 1: line-buffered on a terminal, otherwise
// flushed when full. Returns `count`, or -1 with errno set.
int64_t kestrel_stdout_write(const unsigned char *bytes, int64_t count) {
    if (out_line_mode < 0) out_line_mode = isatty(1) ? 1 : 0;
    if (out_count + count > KESTREL_STREAM_CAPACITY && kestrel_stdout_flush() < 0) return -1;
    if (count >= KESTREL_STREAM_CAPACITY) return write_all(1, bytes, count) < 0 ? -1 : count;
    memcpy(out_buffer + out_count, bytes, (size_t)count);
    out_count += count;
    if (out_line_mode && memchr(bytes, '\n', (size_t)count) && kestrel_stdout_flush() < 0) return -1;
    return count;
}

// Reads up to `count` bytes of fd 0 through the shared buffer. Returns the
// bytes read, 0 at end of file, or -1 with errno set.
int64_t kestrel_stdin_read(unsigned char *bytes, int64_t count) {
    if (count <= 0) return 0;
    if (in_start == in_end) {
        // Nothing buffered: a large read skips the buffer.
        if (count >= KESTREL_STREAM_CAPACITY) return read(0, bytes, (size_t)count);
        ssize_t n = read(0, in_buffer, KESTREL_STREAM_CAPACITY);
        if (n <= 0) return n;
        in_start = 0;
        in_end = n;
    }
    int64_t n = in_end - in_start;
    if (n > count) n = count;
    memcpy(bytes, in_buffer + in_start, (size_t)n);
    in_start += n;
    return n;
}

static void flush_at_exit(void) { kestrel_stdout_flush(); }

__attribute__((constructor)) static void kestrel_streams_init(void) { atexit(flush_at_exit); }

// ---------------------------------------------------------------------------
// errno
//
// The E* numbers differ between Linux and macOS, so classification happens
// here, where the C headers know them. The kind numbers match the case order
// of `IoErrorKind` in error.ks.
// ---------------------------------------------------------------------------

int32_t kestrel_errno(void) { return errno; }

int32_t kestrel_errno_kind(int32_t code) {
    switch (code) {
    case ENOENT: return 1;          // NotFound
    case EACCES: return 2;          // PermissionDenied
    case EEXIST: return 3;          // AlreadyExists
    case EINVAL: return 4;          // InvalidInput
    case EILSEQ: return 5;          // InvalidData
    case ETIMEDOUT: return 7;       // TimedOut
    case EAGAIN: return 8;          // WouldBlock
#if EWOULDBLOCK != EAGAIN
    case EWOULDBLOCK: return 8;
#endif
    case EINTR: return 9;           // Interrupted
    case EPIPE: return 10;          // BrokenPipe
    case ENOMEM: return 11;         // OutOfMemory
    case ENOTDIR: return 12;        // NotADirectory
    case EISDIR: return 13;         // IsADirectory
    case ENOSPC: return 14;         // NoSpaceLeft
    case EIO: return 15;            // IoFailure
    case EBADF: return 16;          // BadFileDescriptor
    case EPERM: return 17;          // NotPermitted
    case ECONNREFUSED: return 18;   // ConnectionRefused
    case ECONNRESET: return 19;     // ConnectionReset
    case ECONNABORTED: return 20;   // ConnectionAborted
    case ENOTCONN: return 21;       // NotConnected
    case EADDRINUSE: return 22;     // AddressInUse
    case EADDRNOTAVAIL: return 23;  // AddressNotAvailable
    case ENOTEMPTY: return 25;      // DirectoryNotEmpty
    case EXDEV: return 26;          // CrossesDevices
    case ENOSYS: return 27;         // Unsupported
#if defined(ENOTSUP)
    case ENOTSUP: return 27;
#endif
#if defined(EOPNOTSUPP) && EOPNOTSUPP != ENOTSUP
    case EOPNOTSUPP: return 27;
#endif
    case EAFNOSUPPORT: return 27;
    case EPROTONOSUPPORT: return 27;
    default: return 0;              // Other
    }
}

// The platform's message for `code`, for errors with no named kind.
const char *kestrel_strerror(int32_t code) { return strerror(code); }
