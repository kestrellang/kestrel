// C side of `cfunction_callbacks.ks`: APIs that take C function pointers.
#include <stdint.h>
#include <stddef.h>

typedef struct { int32_t a, b; } I32x2;
typedef struct { float x, y, z; } F32x3;

typedef int64_t (*IntFn)(int64_t x, void *userdata);
typedef I32x2 (*PairFn)(I32x2 v);
typedef F32x3 (*VecFn)(F32x3 v);

// A C "descriptor" holding a callback, the way wgpu's callback infos do.
typedef struct {
    IntFn callback;
    void *userdata;
} CallbackInfo;

typedef struct {
    const void *next;
    uint32_t mode;
    IntFn callback;
    void *userdata1;
    void *userdata2;
} ModeCallbackInfo;

typedef struct { const char *data; size_t length; } View;
typedef int64_t (*ViewFn)(View v);

int64_t cb_apply_mode_info(ModeCallbackInfo info, int64_t x) { return info.mode * 1000 + info.callback(x, info.userdata1); }
int64_t cb_view(ViewFn f) { return f((View){"hello", 5}); }
int64_t cb_apply(IntFn f, void *userdata, int64_t x) { return f(x, userdata); }
int64_t cb_apply_info(CallbackInfo info, int64_t x) { return info.callback ? info.callback(x, info.userdata) : -1; }
int64_t cb_pair(PairFn f) { I32x2 r = f((I32x2){3, 4}); return r.a * 10 + r.b; }
int64_t cb_vec(VecFn f) { F32x3 r = f((F32x3){1.0f, 2.0f, 3.0f}); return (int64_t)(r.x * 100 + r.y * 10 + r.z); }
int64_t cb_is_null(IntFn f) { return f == NULL; }
