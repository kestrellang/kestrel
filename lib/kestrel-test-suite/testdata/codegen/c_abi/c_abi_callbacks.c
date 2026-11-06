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

// C function pointers handed back to Kestrel, which calls them.
static int64_t c_double(int64_t x, void *userdata) { (void)userdata; return x * 2; }
static I32x2 c_sum_pair(I32x2 v) { return (I32x2){v.a + v.b, v.a - v.b}; }
static uint8_t c_u8_add(uint8_t a, uint8_t b) { return (uint8_t)(a + b); }
static int8_t c_i8_neg(int8_t a) { return (int8_t)-a; }
typedef uint8_t (*U8Fn)(uint8_t a, uint8_t b);
typedef int8_t (*I8Fn)(int8_t a);
IntFn cb_get_double(void) { return c_double; }
PairFn cb_get_sum_pair(void) { return c_sum_pair; }
U8Fn cb_get_u8_add(void) { return c_u8_add; }
I8Fn cb_get_i8_neg(void) { return c_i8_neg; }
IntFn cb_maybe_double(int64_t which) { return which ? c_double : NULL; }
