// Non-variadic wrappers for libc functions whose real signatures are variadic.
//
// On Apple aarch64, variadic arguments are passed on the stack while fixed
// arguments go in registers. Kestrel's @extern(.C) bindings emit fixed-arg
// calling conventions, so calling a variadic libc function directly corrupts
// the argument values. These thin wrappers have fixed signatures that the
// compiler can call correctly.

#include <fcntl.h>

int kestrel_open(const char *path, int flags, int mode) {
    return open(path, flags, mode);
}
