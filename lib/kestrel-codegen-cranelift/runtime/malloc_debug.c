#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define BUMP_CAPACITY (64u * 1024u * 1024u)
#define BUMP_ALIGN 16u

static unsigned char bump[BUMP_CAPACITY];
static size_t bump_offset = 0;

static size_t align_up(size_t value, size_t align) {
    return (value + (align - 1)) & ~(align - 1);
}

static void *bump_alloc(int64_t size) {
    if (size <= 0) {
        return NULL;
    }

    size_t requested = (size_t)size;
    size_t payload_start = align_up(bump_offset + sizeof(size_t), BUMP_ALIGN);
    size_t payload_size = align_up(requested, BUMP_ALIGN);
    size_t end = payload_start + payload_size;

    if (end > BUMP_CAPACITY) {
        return NULL;
    }

    size_t *header = (size_t *)(bump + payload_start - sizeof(size_t));
    *header = requested;

    void *ptr = (void *)(bump + payload_start);
    bump_offset = end;
    return ptr;
}

static int read_alloc_size(void *ptr, size_t *out_size) {
    if (ptr == NULL) {
        return 0;
    }

    unsigned char *base = bump;
    unsigned char *p = (unsigned char *)ptr;
    if (p < base + sizeof(size_t) || p >= base + BUMP_CAPACITY) {
        return 0;
    }

    size_t *header = (size_t *)(p - sizeof(size_t));
    *out_size = *header;
    return 1;
}

void *malloc_debug(int64_t size) {
    void *ptr = bump_alloc(size);
    fprintf(stderr, "malloc_debug(%" PRId64 ") -> %p\n", size, ptr);
    return ptr;
}

void free_debug(void *ptr) {
    fprintf(stderr, "free_debug(%p)\n", ptr);
}

void *realloc_debug(void *ptr, int64_t size) {
    fprintf(stderr, "realloc_debug(%p, %" PRId64 ")", ptr, size);

    if (ptr == NULL) {
        void *new_ptr = bump_alloc(size);
        fprintf(stderr, " -> %p\n", new_ptr);
        return new_ptr;
    }

    if (size <= 0) {
        fprintf(stderr, " -> NULL\n");
        return NULL;
    }

    size_t old_size = 0;
    if (!read_alloc_size(ptr, &old_size)) {
        fprintf(stderr, " -> NULL (invalid pointer)\n");
        return NULL;
    }

    void *new_ptr = bump_alloc(size);
    if (new_ptr == NULL) {
        fprintf(stderr, " -> NULL (oom)\n");
        return NULL;
    }

    size_t new_size = (size_t)size;
    size_t copy_size = old_size < new_size ? old_size : new_size;
    if (copy_size > 0) {
        memmove(new_ptr, ptr, copy_size);
    }

    fprintf(stderr, " -> %p (copied %zu)\n", new_ptr, copy_size);
    return new_ptr;
}

void *memcpy_debug(void *dest, const void *src, int64_t n) {
    fprintf(stderr, "memcpy_debug(%p, %p, %" PRId64 ")\n", dest, src, n);
    if (n <= 0) {
        return dest;
    }
    return memcpy(dest, src, (size_t)n);
}

void *memmove_debug(void *dest, const void *src, int64_t n) {
    fprintf(stderr, "memmove_debug(%p, %p, %" PRId64 ")\n", dest, src, n);
    if (n <= 0) {
        return dest;
    }
    return memmove(dest, src, (size_t)n);
}

void *memset_debug(void *dest, int64_t c, int64_t n) {
    fprintf(stderr, "memset_debug(%p, %" PRId64 ", %" PRId64 ")\n", dest, c, n);
    if (n <= 0) {
        return dest;
    }
    return memset(dest, (int)c, (size_t)n);
}
