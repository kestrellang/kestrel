// C helpers behind std.files.
//
// They exist where the libc call takes a struct whose layout differs per
// target (`struct stat`, `struct dirent`), or where the constants differ
// (open flags, d_type values). Kestrel code calls only these fixed-signature
// wrappers, so no struct offsets are hand-coded in .ks files.

#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

// Open-option bits, matching `OpenOptions.nativeFlags()` in file.ks.
enum {
    KESTREL_READ = 1,
    KESTREL_WRITE = 2,
    KESTREL_APPEND = 4,
    KESTREL_TRUNCATE = 8,
    KESTREL_CREATE = 16,
    KESTREL_CREATE_NEW = 32,
};

// open(2) with close-on-exec. Returns the fd, or -1 with errno set.
int32_t kestrel_file_open(const char *path, int32_t options, int32_t mode) {
    int write = options & (KESTREL_WRITE | KESTREL_APPEND);
    int flags = O_CLOEXEC;
    if ((options & KESTREL_READ) && write) flags |= O_RDWR;
    else if (write) flags |= O_WRONLY;
    else flags |= O_RDONLY;
    if (options & KESTREL_APPEND) flags |= O_APPEND;
    if (options & KESTREL_TRUNCATE) flags |= O_TRUNC;
    if (options & KESTREL_CREATE_NEW) flags |= O_CREAT | O_EXCL;
    else if (options & KESTREL_CREATE) flags |= O_CREAT;
    int fd;
    do {
        fd = open(path, flags, (mode_t)mode);
    } while (fd < 0 && errno == EINTR);
    return fd;
}

// lseek(2); `whence` is 0 start, 1 current, 2 end.
int64_t kestrel_file_seek(int32_t fd, int64_t offset, int32_t whence) {
    int w = whence == 0 ? SEEK_SET : whence == 1 ? SEEK_CUR : SEEK_END;
    return (int64_t)lseek(fd, (off_t)offset, w);
}

int32_t kestrel_file_sync(int32_t fd) { return fsync(fd); }

int32_t kestrel_file_set_length(int32_t fd, int64_t length) { return ftruncate(fd, (off_t)length); }

// ---------------------------------------------------------------------------
// Metadata
//
// `out` receives seven int64s: kind (0 file, 1 directory, 2 symbolic link,
// 3 other), size, permission bits, modified and accessed time in
// nanoseconds since the Unix epoch, device and inode. Keep in step with
// `metadataCellCount` in metadata.ks.
// ---------------------------------------------------------------------------

static int64_t kind_of(mode_t mode) {
    if (S_ISREG(mode)) return 0;
    if (S_ISDIR(mode)) return 1;
    if (S_ISLNK(mode)) return 2;
    return 3;
}

static void fill_metadata(const struct stat *st, int64_t *out) {
    out[0] = kind_of(st->st_mode);
    out[1] = (int64_t)st->st_size;
    out[2] = (int64_t)(st->st_mode & 07777);
#ifdef __APPLE__
    out[3] = (int64_t)st->st_mtimespec.tv_sec * 1000000000 + st->st_mtimespec.tv_nsec;
    out[4] = (int64_t)st->st_atimespec.tv_sec * 1000000000 + st->st_atimespec.tv_nsec;
#else
    out[3] = (int64_t)st->st_mtim.tv_sec * 1000000000 + st->st_mtim.tv_nsec;
    out[4] = (int64_t)st->st_atim.tv_sec * 1000000000 + st->st_atim.tv_nsec;
#endif
    out[5] = (int64_t)st->st_dev;
    out[6] = (int64_t)st->st_ino;
}

// stat(2), or lstat(2) when `follow` is 0. 0, or -1 with errno set.
int32_t kestrel_path_metadata(const char *path, int32_t follow, int64_t *out) {
    struct stat st;
    int r = follow ? stat(path, &st) : lstat(path, &st);
    if (r != 0) return -1;
    fill_metadata(&st, out);
    return 0;
}

int32_t kestrel_file_metadata(int32_t fd, int64_t *out) {
    struct stat st;
    if (fstat(fd, &st) != 0) return -1;
    fill_metadata(&st, out);
    return 0;
}

// ---------------------------------------------------------------------------
// Directories
// ---------------------------------------------------------------------------

void *kestrel_directory_open(const char *path) { return opendir(path); }

void kestrel_directory_close(void *dir) {
    if (dir) closedir((DIR *)dir);
}

// The next entry other than "." and "..". Stores its name (valid until the
// next call) and kind (0 file, 1 directory, 2 symbolic link, 3 other,
// 4 unknown: ask lstat). Returns 1 for an entry, 0 at the end, -1 with
// errno set on failure.
int32_t kestrel_directory_next(void *dir, const char **name, int32_t *kind) {
    for (;;) {
        errno = 0;
        struct dirent *entry = readdir((DIR *)dir);
        if (!entry) return errno ? -1 : 0;
        const char *n = entry->d_name;
        if (n[0] == '.' && (n[1] == 0 || (n[1] == '.' && n[2] == 0))) continue;
        *name = n;
        switch (entry->d_type) {
        case DT_REG: *kind = 0; break;
        case DT_DIR: *kind = 1; break;
        case DT_LNK: *kind = 2; break;
        case DT_UNKNOWN: *kind = 4; break;
        default: *kind = 3; break;
        }
        return 1;
    }
}

// mkdtemp(3) on `template`, which must end in "XXXXXX". Rewrites it in
// place. 0, or -1 with errno set.
int32_t kestrel_make_temporary_directory(char *template_path) {
    return mkdtemp(template_path) ? 0 : -1;
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

// realpath(3). A malloc'd string the caller frees, or NULL with errno set.
char *kestrel_path_canonical(const char *path) { return realpath(path, NULL); }

// readlink(2) into `buffer`. The length, or -1 with errno set. A result as
// long as `capacity` may be truncated: retry with a larger buffer.
int64_t kestrel_path_read_link(const char *path, char *buffer, int64_t capacity) {
    return (int64_t)readlink(path, buffer, (size_t)capacity);
}
