// datetime_shims.c — C shim for Kestrel datetime package
//
// Provides: nanosecond-precision clock access, system timezone query,
// and a global timezone registry backed by TZif file parsing.

#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>
#include <dirent.h>
#include <pthread.h>
#include <stdatomic.h>
#include <sys/stat.h>

// ============================================================================
// Clock access
// ============================================================================

void kestrel_clock_gettime(int64_t* sec_out, int64_t* nsec_out) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    *sec_out = (int64_t)ts.tv_sec;
    *nsec_out = (int64_t)ts.tv_nsec;
}

// ============================================================================
// Timezone registry — TZif parsing + offset lookup
// ============================================================================
//
// Zones are interned: a TimeZone on the Kestrel side is just an index into
// this registry. Entries live in fixed-size chunks that never move, so a
// lookup by id needs no lock; registration (which appends) takes `tz_lock`
// and publishes the new count with release ordering.

#define TZ_CHUNK_SIZE 64
#define TZ_MAX_CHUNKS 1024                      // 65,536 zones
#define TZ_FOOTER_LAST_YEAR 2500                // footer rules expanded through this year

struct tz_ttinfo {
    int32_t utoff;
    int8_t  is_dst;
    uint8_t abbr_idx;
};

struct tz_entry {
    char name[128];
    int64_t* transitions;            // ascending epoch seconds
    uint8_t* transition_types;       // ttinfo index in effect from each transition
    int64_t num_transitions;
    struct tz_ttinfo* ttinfos;
    int64_t num_ttinfos;
    char* abbreviations;             // NUL-separated pool indexed by abbr_idx
    int64_t abbr_len;
};

static struct tz_entry* tz_chunks[TZ_MAX_CHUNKS];
static _Atomic int64_t tz_count = 0;
static pthread_mutex_t tz_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_once_t tz_once = PTHREAD_ONCE_INIT;

// Entry for `id`, or NULL if it isn't registered.
static struct tz_entry* tz_get(int64_t id) {
    if (id < 0 || id >= atomic_load_explicit(&tz_count, memory_order_acquire)) return NULL;
    return &tz_chunks[id / TZ_CHUNK_SIZE][id % TZ_CHUNK_SIZE];
}

// Reserve the slot for the next id (caller holds tz_lock). NULL when full.
static struct tz_entry* tz_reserve(int64_t* id_out) {
    int64_t id = atomic_load_explicit(&tz_count, memory_order_relaxed);
    int64_t chunk = id / TZ_CHUNK_SIZE;
    if (chunk >= TZ_MAX_CHUNKS) return NULL;
    if (!tz_chunks[chunk]) {
        tz_chunks[chunk] = (struct tz_entry*)calloc(TZ_CHUNK_SIZE, sizeof(struct tz_entry));
        if (!tz_chunks[chunk]) return NULL;
    }
    struct tz_entry* entry = &tz_chunks[chunk][id % TZ_CHUNK_SIZE];
    memset(entry, 0, sizeof(*entry));
    *id_out = id;
    return entry;
}

// Make a filled reserved entry visible (caller holds tz_lock).
static void tz_publish(void) {
    atomic_fetch_add_explicit(&tz_count, 1, memory_order_release);
}

static void tz_entry_free(struct tz_entry* e) {
    free(e->transitions);
    free(e->transition_types);
    free(e->ttinfos);
    free(e->abbreviations);
    memset(e, 0, sizeof(*e));
}

// A zone with no transitions and a single time type named `abbr`.
static int tz_fill_constant(struct tz_entry* e, const char* name, int32_t utoff, const char* abbr) {
    strncpy(e->name, name, sizeof(e->name) - 1);
    e->ttinfos = (struct tz_ttinfo*)calloc(1, sizeof(struct tz_ttinfo));
    e->abbr_len = (int64_t)strlen(abbr) + 1;
    e->abbreviations = (char*)malloc((size_t)e->abbr_len);
    if (!e->ttinfos || !e->abbreviations) { tz_entry_free(e); return -1; }
    memcpy(e->abbreviations, abbr, (size_t)e->abbr_len);
    e->num_ttinfos = 1;
    e->ttinfos[0].utoff = utoff;
    return 0;
}

// UTC is always id 0.
static void tz_init_utc(void) {
    pthread_mutex_lock(&tz_lock);
    int64_t id;
    struct tz_entry* e = tz_reserve(&id);
    if (e && tz_fill_constant(e, "UTC", 0, "UTC") == 0) tz_publish();
    pthread_mutex_unlock(&tz_lock);
}

static void tz_ensure_init(void) {
    pthread_once(&tz_once, tz_init_utc);
}

// ----------------------------------------------------------------------------
// Civil-calendar helpers (proleptic Gregorian, days since 1970-01-01)
// ----------------------------------------------------------------------------

static int64_t floor_div(int64_t a, int64_t b) {
    int64_t q = a / b;
    return (a % b != 0 && ((a < 0) != (b < 0))) ? q - 1 : q;
}

static int64_t days_from_civil(int64_t y, int64_t m, int64_t d) {
    y -= m <= 2;
    int64_t era = floor_div(y, 400);
    int64_t yoe = y - era * 400;
    int64_t doy = (153 * (m + (m > 2 ? -3 : 9)) + 2) / 5 + d - 1;
    int64_t doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return era * 146097 + doe - 719468;
}

static int64_t year_of_epoch(int64_t epoch_sec) {
    int64_t z = floor_div(epoch_sec, 86400) + 719468;
    int64_t era = floor_div(z, 146097);
    int64_t doe = z - era * 146097;
    int64_t yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    int64_t doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    int64_t mp = (5 * doy + 2) / 153;
    int64_t m = mp + (mp < 10 ? 3 : -9);
    return yoe + era * 400 + (m <= 2);
}

static int is_leap(int64_t y) {
    return y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
}

static int64_t days_in_month(int64_t y, int64_t m) {
    static const int64_t dim[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    return m == 2 && is_leap(y) ? 29 : dim[m - 1];
}

// ----------------------------------------------------------------------------
// POSIX TZ footer (TZif v2+): "EST5EDT,M3.2.0,M11.1.0"
// ----------------------------------------------------------------------------

enum tz_rule_kind { RULE_JULIAN_NO_LEAP, RULE_ZERO_BASED, RULE_MONTH_WEEK_DAY };

struct tz_rule {
    enum tz_rule_kind kind;
    int64_t day;            // Jn: 1..365; n: 0..365; M: weekday 0 (Sunday)..6
    int64_t month, week;    // M form only; week 5 = last
    int64_t time;           // seconds after local midnight (may be negative or > 24h)
};

struct tz_footer {
    char std_abbr[16], dst_abbr[16];
    int32_t std_utoff, dst_utoff;   // seconds east of UTC
    int has_dst;
    struct tz_rule start, end;      // start: DST begins (local std time); end: DST ends (local dst time)
};

static const char* tz_parse_abbr(const char* p, char* out, size_t out_len) {
    size_t n = 0;
    if (*p == '<') {
        p++;
        while (*p && *p != '>') { if (n + 1 < out_len) out[n++] = *p; p++; }
        if (*p != '>') return NULL;
        p++;
    } else {
        while ((*p >= 'A' && *p <= 'Z') || (*p >= 'a' && *p <= 'z')) { if (n + 1 < out_len) out[n++] = *p; p++; }
    }
    out[n] = '\0';
    return n >= 3 ? p : NULL;
}

// [+-]hh[:mm[:ss]] -> seconds (sign applied as written).
static const char* tz_parse_hms(const char* p, int64_t* out) {
    int sign = 1;
    if (*p == '+' || *p == '-') { sign = *p == '-' ? -1 : 1; p++; }
    if (*p < '0' || *p > '9') return NULL;
    int64_t parts[3] = {0, 0, 0};
    for (int i = 0; i < 3; i++) {
        if (i > 0) { if (*p != ':') break; p++; }
        if (*p < '0' || *p > '9') return NULL;
        int64_t v = 0;
        while (*p >= '0' && *p <= '9') { v = v * 10 + (*p - '0'); p++; }
        parts[i] = v;
    }
    *out = sign * (parts[0] * 3600 + parts[1] * 60 + parts[2]);
    return p;
}

static const char* tz_parse_int(const char* p, int64_t* out) {
    if (*p < '0' || *p > '9') return NULL;
    int64_t v = 0;
    while (*p >= '0' && *p <= '9') { v = v * 10 + (*p - '0'); p++; }
    *out = v;
    return p;
}

static const char* tz_parse_rule(const char* p, struct tz_rule* r) {
    if (*p == 'M') {
        r->kind = RULE_MONTH_WEEK_DAY;
        p = tz_parse_int(p + 1, &r->month);
        if (!p || *p != '.') return NULL;
        p = tz_parse_int(p + 1, &r->week);
        if (!p || *p != '.') return NULL;
        p = tz_parse_int(p + 1, &r->day);
        if (!p || r->month < 1 || r->month > 12 || r->week < 1 || r->week > 5 || r->day > 6) return NULL;
    } else if (*p == 'J') {
        r->kind = RULE_JULIAN_NO_LEAP;
        p = tz_parse_int(p + 1, &r->day);
        if (!p || r->day < 1 || r->day > 365) return NULL;
    } else {
        r->kind = RULE_ZERO_BASED;
        p = tz_parse_int(p, &r->day);
        if (!p || r->day > 365) return NULL;
    }
    r->time = 7200;
    if (*p == '/') p = tz_parse_hms(p + 1, &r->time);
    return p;
}

// Parse the footer string; 0 on success. POSIX offsets are west-positive.
static int tz_parse_footer(const char* s, struct tz_footer* f) {
    memset(f, 0, sizeof(*f));
    int64_t off;
    const char* p = tz_parse_abbr(s, f->std_abbr, sizeof(f->std_abbr));
    if (!p || !(p = tz_parse_hms(p, &off))) return -1;
    f->std_utoff = (int32_t)-off;
    if (*p == '\0') return 0;
    p = tz_parse_abbr(p, f->dst_abbr, sizeof(f->dst_abbr));
    if (!p) return -1;
    f->has_dst = 1;
    f->dst_utoff = f->std_utoff + 3600;
    if (*p && *p != ',') {
        if (!(p = tz_parse_hms(p, &off))) return -1;
        f->dst_utoff = (int32_t)-off;
    }
    // Without explicit rules POSIX defaults to US rules
    if (*p == '\0') {
        f->start = (struct tz_rule){RULE_MONTH_WEEK_DAY, 0, 3, 2, 7200};
        f->end = (struct tz_rule){RULE_MONTH_WEEK_DAY, 0, 11, 1, 7200};
        return 0;
    }
    if (*p != ',' || !(p = tz_parse_rule(p + 1, &f->start))) return -1;
    if (*p != ',' || !(p = tz_parse_rule(p + 1, &f->end))) return -1;
    return *p == '\0' ? 0 : -1;
}

// Local-midnight day number (since 1970-01-01) on which `r` falls in `year`.
static int64_t tz_rule_day(const struct tz_rule* r, int64_t year) {
    int64_t jan1 = days_from_civil(year, 1, 1);
    switch (r->kind) {
    case RULE_JULIAN_NO_LEAP:
        return jan1 + r->day - 1 + (is_leap(year) && r->day >= 60 ? 1 : 0);
    case RULE_ZERO_BASED:
        return jan1 + r->day;
    case RULE_MONTH_WEEK_DAY: default: {
        int64_t first = days_from_civil(year, r->month, 1);
        int64_t first_dow = ((first % 7) + 7 + 4) % 7;   // 1970-01-01 was a Thursday (4)
        int64_t dom = 1 + (r->day - first_dow + 7) % 7 + (r->week - 1) * 7;
        while (dom > days_in_month(year, r->month)) dom -= 7;
        return first + dom - 1;
    }
    }
}

// Index of a ttinfo matching (utoff, is_dst, abbr), appending one (and its
// abbreviation) if none exists. -1 on allocation failure or 256-type overflow.
static int64_t tz_intern_ttinfo(struct tz_entry* e, int32_t utoff, int8_t is_dst, const char* abbr) {
    for (int64_t i = 0; i < e->num_ttinfos; i++) {
        const struct tz_ttinfo* t = &e->ttinfos[i];
        if (t->utoff == utoff && t->is_dst == is_dst && strcmp(e->abbreviations + t->abbr_idx, abbr) == 0) return i;
    }
    if (e->num_ttinfos >= 256) return -1;
    int64_t abbr_idx = e->abbr_len;
    size_t len = strlen(abbr) + 1;
    if (abbr_idx + (int64_t)len > 255) return -1;
    char* pool = (char*)realloc(e->abbreviations, (size_t)abbr_idx + len);
    struct tz_ttinfo* types = (struct tz_ttinfo*)realloc(e->ttinfos, (size_t)(e->num_ttinfos + 1) * sizeof(struct tz_ttinfo));
    if (pool) e->abbreviations = pool;
    if (types) e->ttinfos = types;
    if (!pool || !types) return -1;
    memcpy(e->abbreviations + abbr_idx, abbr, len);
    e->abbr_len = abbr_idx + (int64_t)len;
    e->ttinfos[e->num_ttinfos] = (struct tz_ttinfo){utoff, is_dst, (uint8_t)abbr_idx};
    return e->num_ttinfos++;
}

// Append explicit transitions generated from the footer's DST rule for every
// year after the table's last transition, through TZ_FOOTER_LAST_YEAR, so all
// lookups (offsets, gap/fold detection) read one uniform table.
static int tz_extend_with_footer(struct tz_entry* e, const struct tz_footer* f) {
    if (!f->has_dst) return 0;
    int64_t std_type = tz_intern_ttinfo(e, f->std_utoff, 0, f->std_abbr);
    int64_t dst_type = tz_intern_ttinfo(e, f->dst_utoff, 1, f->dst_abbr);
    if (std_type < 0 || dst_type < 0) return -1;

    int64_t last = e->num_transitions ? e->transitions[e->num_transitions - 1] : INT64_MIN;
    int64_t first_year = e->num_transitions ? year_of_epoch(last) : 1970;
    if (first_year > TZ_FOOTER_LAST_YEAR) return 0;
    int64_t extra = (TZ_FOOTER_LAST_YEAR - first_year + 1) * 2;
    int64_t cap = e->num_transitions + extra;
    int64_t* times = (int64_t*)realloc(e->transitions, (size_t)cap * sizeof(int64_t));
    if (times) e->transitions = times;
    uint8_t* types = (uint8_t*)realloc(e->transition_types, (size_t)cap);
    if (types) e->transition_types = types;
    if (!times || !types) return -1;

    for (int64_t y = first_year; y <= TZ_FOOTER_LAST_YEAR; y++) {
        // DST starts at a std-local wall time and ends at a dst-local wall time.
        int64_t on = tz_rule_day(&f->start, y) * 86400 + f->start.time - f->std_utoff;
        int64_t off = tz_rule_day(&f->end, y) * 86400 + f->end.time - f->dst_utoff;
        int64_t first_t = on < off ? on : off, second_t = on < off ? off : on;
        uint8_t first_type = (uint8_t)(on < off ? dst_type : std_type);
        uint8_t second_type = (uint8_t)(on < off ? std_type : dst_type);
        if (first_t > last) {
            e->transitions[e->num_transitions] = first_t;
            e->transition_types[e->num_transitions++] = first_type;
            last = first_t;
        }
        if (second_t > last) {
            e->transitions[e->num_transitions] = second_t;
            e->transition_types[e->num_transitions++] = second_type;
            last = second_t;
        }
    }
    return 0;
}

// ----------------------------------------------------------------------------
// TZif parsing
// ----------------------------------------------------------------------------

static int32_t read_be32(const uint8_t* p) {
    return (int32_t)(((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
                     ((uint32_t)p[2] << 8)  | ((uint32_t)p[3]));
}

static int64_t read_be64(const uint8_t* p) {
    return (int64_t)(((uint64_t)p[0] << 56) | ((uint64_t)p[1] << 48) |
                     ((uint64_t)p[2] << 40) | ((uint64_t)p[3] << 32) |
                     ((uint64_t)p[4] << 24) | ((uint64_t)p[5] << 16) |
                     ((uint64_t)p[6] << 8)  | ((uint64_t)p[7]));
}

struct tzif_header {
    int64_t isutcnt, isstdcnt, leapcnt, timecnt, typecnt, charcnt;
};

static struct tzif_header tzif_read_header(const uint8_t* h) {
    return (struct tzif_header){read_be32(h + 20), read_be32(h + 24), read_be32(h + 28),
                                read_be32(h + 32), read_be32(h + 36), read_be32(h + 40)};
}

// Bytes in a data block whose transition times are `time_size` wide.
static size_t tzif_block_size(const struct tzif_header* h, size_t time_size) {
    return (size_t)h->timecnt * (time_size + 1) + (size_t)h->typecnt * 6 + (size_t)h->charcnt +
           (size_t)h->leapcnt * (time_size + 4) + (size_t)h->isstdcnt + (size_t)h->isutcnt;
}

// Parse a TZif file into `e` (v2+ 64-bit block preferred, v1 otherwise) and
// extend it with the footer rule. Returns 0 on success.
static int parse_tzif(const uint8_t* data, size_t len, struct tz_entry* e) {
    if (len < 44 || memcmp(data, "TZif", 4) != 0) return -1;
    struct tzif_header h = tzif_read_header(data);
    size_t time_size = 4;
    const uint8_t* block = data + 44;
    const uint8_t* end = data + len;
    if (block + tzif_block_size(&h, 4) > end) return -1;

    if (data[4] >= '2') {
        const uint8_t* v2 = block + tzif_block_size(&h, 4);
        if (v2 + 44 > end || memcmp(v2, "TZif", 4) != 0) return -1;
        h = tzif_read_header(v2);
        time_size = 8;
        block = v2 + 44;
        if (block + tzif_block_size(&h, 8) > end) return -1;
    }
    if (h.typecnt < 1 || h.typecnt > 256 || h.charcnt > 255) return -1;

    const uint8_t* p = block;
    e->num_transitions = h.timecnt;
    e->transitions = (int64_t*)malloc((size_t)(h.timecnt ? h.timecnt : 1) * sizeof(int64_t));
    e->transition_types = (uint8_t*)malloc((size_t)(h.timecnt ? h.timecnt : 1));
    e->ttinfos = (struct tz_ttinfo*)malloc((size_t)h.typecnt * sizeof(struct tz_ttinfo));
    e->abbreviations = (char*)malloc((size_t)h.charcnt + 1);
    if (!e->transitions || !e->transition_types || !e->ttinfos || !e->abbreviations) return -1;

    for (int64_t i = 0; i < h.timecnt; i++, p += time_size) {
        e->transitions[i] = time_size == 8 ? read_be64(p) : (int64_t)read_be32(p);
    }
    for (int64_t i = 0; i < h.timecnt; i++) {
        e->transition_types[i] = *p++;
        if (e->transition_types[i] >= h.typecnt) return -1;
    }
    e->num_ttinfos = h.typecnt;
    for (int64_t i = 0; i < h.typecnt; i++, p += 6) {
        e->ttinfos[i] = (struct tz_ttinfo){read_be32(p), (int8_t)p[4], p[5]};
    }
    memcpy(e->abbreviations, p, (size_t)h.charcnt);
    e->abbreviations[h.charcnt] = '\0';
    e->abbr_len = h.charcnt + 1;

    // v2+ footer: "\n<POSIX TZ>\n" right after the 64-bit block
    if (time_size == 8) {
        const uint8_t* footer = block + tzif_block_size(&h, 8);
        if (footer < end && *footer == '\n') {
            const uint8_t* nl = memchr(footer + 1, '\n', (size_t)(end - footer - 1));
            if (nl && nl - footer - 1 < 128) {
                char tz[128];
                memcpy(tz, footer + 1, (size_t)(nl - footer - 1));
                tz[nl - footer - 1] = '\0';
                struct tz_footer f;
                if (tz[0] && tz_parse_footer(tz, &f) == 0 && tz_extend_with_footer(e, &f) != 0) return -1;
            }
        }
    }
    return 0;
}

// ----------------------------------------------------------------------------
// Registration
// ----------------------------------------------------------------------------

// Parse a fixed-offset zone name of the exact form "+HH:MM" / "-HH:MM".
// Returns 1 and writes the offset in seconds on success, 0 otherwise.
static int parse_fixed_offset_name(const char* name, int32_t* offset_out) {
    if (strlen(name) != 6) return 0;
    if (name[0] != '+' && name[0] != '-') return 0;
    if (name[3] != ':') return 0;
    const char digits[4] = { name[1], name[2], name[4], name[5] };
    for (int i = 0; i < 4; i++) {
        if (digits[i] < '0' || digits[i] > '9') return 0;
    }
    int32_t hours = (digits[0] - '0') * 10 + (digits[1] - '0');
    int32_t minutes = (digits[2] - '0') * 10 + (digits[3] - '0');
    if (hours > 23 || minutes > 59) return 0;
    int32_t magnitude = hours * 3600 + minutes * 60;
    *offset_out = name[0] == '-' ? -magnitude : magnitude;
    return 1;
}

// An IANA name may only name a file under the zoneinfo root: relative, made of
// [A-Za-z0-9_+-.] segments separated by '/', with no empty, "." or ".." segment.
static int tz_name_is_safe(const char* name) {
    size_t len = strlen(name);
    if (len == 0 || len >= 128 || name[0] == '/') return 0;
    const char* seg = name;
    for (const char* p = name;; p++) {
        if (*p == '/' || *p == '\0') {
            size_t n = (size_t)(p - seg);
            if (n == 0 || (n == 1 && seg[0] == '.') || (n == 2 && seg[0] == '.' && seg[1] == '.')) return 0;
            if (*p == '\0') return 1;
            seg = p + 1;
            continue;
        }
        char c = *p;
        int ok = (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') ||
                 c == '_' || c == '-' || c == '+' || c == '.';
        if (!ok) return 0;
    }
}

// Load and fill `e` from /usr/share/zoneinfo/<name>. 0 on success.
static int tz_load_file(struct tz_entry* e, const char* name) {
    if (!tz_name_is_safe(name)) return -1;
    char path[512];
    snprintf(path, sizeof(path), "/usr/share/zoneinfo/%s", name);
    FILE* f = fopen(path, "rb");
    if (!f) return -1;
    fseek(f, 0, SEEK_END);
    long file_len = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (file_len <= 0 || file_len > 1024 * 1024) { fclose(f); return -1; }
    uint8_t* data = (uint8_t*)malloc((size_t)file_len);
    if (!data) { fclose(f); return -1; }
    size_t read_len = fread(data, 1, (size_t)file_len, f);
    fclose(f);
    int result = (long)read_len == file_len ? parse_tzif(data, (size_t)file_len, e) : -1;
    free(data);
    strncpy(e->name, name, sizeof(e->name) - 1);
    return result;
}

// Id of an already-registered zone named `name`, or -1 (caller holds tz_lock).
static int64_t tz_find_locked(const char* name) {
    int64_t count = atomic_load_explicit(&tz_count, memory_order_relaxed);
    for (int64_t i = 0; i < count; i++) {
        if (strcmp(tz_get(i)->name, name) == 0) return i;
    }
    return -1;
}

// Find or register a zone: a fixed offset ("+05:30"; zero is UTC) or an IANA
// name loaded from /usr/share/zoneinfo. Returns its id, or -1.
int64_t kestrel_tz_find_or_register(const char* name) {
    tz_ensure_init();
    int32_t fixed_offset = 0;
    int is_fixed = parse_fixed_offset_name(name, &fixed_offset);
    if (is_fixed && fixed_offset == 0) return 0;

    pthread_mutex_lock(&tz_lock);
    int64_t id = tz_find_locked(name);
    if (id < 0) {
        struct tz_entry* e = tz_reserve(&id);
        int ok = e && (is_fixed ? tz_fill_constant(e, name, fixed_offset, name) : tz_load_file(e, name)) == 0;
        if (ok) {
            tz_publish();
        } else {
            if (e) tz_entry_free(e);
            id = -1;
        }
    }
    pthread_mutex_unlock(&tz_lock);
    return id;
}

// ----------------------------------------------------------------------------
// Queries
// ----------------------------------------------------------------------------

// Index of the last transition at or before `epoch_sec`, or -1 if none.
static int64_t tz_transition_index(const struct tz_entry* e, int64_t epoch_sec) {
    int64_t lo = 0, hi = e->num_transitions - 1, found = -1;
    while (lo <= hi) {
        int64_t mid = lo + (hi - lo) / 2;
        if (e->transitions[mid] <= epoch_sec) { found = mid; lo = mid + 1; } else { hi = mid - 1; }
    }
    return found;
}

// Time type in effect at `epoch_sec`. Before the first transition RFC 8536
// specifies time type 0 (typically local mean time).
static const struct tz_ttinfo* tz_lookup(const struct tz_entry* e, int64_t epoch_sec) {
    int64_t i = tz_transition_index(e, epoch_sec);
    return &e->ttinfos[i < 0 ? 0 : e->transition_types[i]];
}

int32_t kestrel_tz_offset(int64_t tz_id, int64_t epoch_sec) {
    tz_ensure_init();
    const struct tz_entry* e = tz_get(tz_id);
    return e ? tz_lookup(e, epoch_sec)->utoff : 0;
}

int32_t kestrel_tz_is_dst(int64_t tz_id, int64_t epoch_sec) {
    tz_ensure_init();
    const struct tz_entry* e = tz_get(tz_id);
    return e ? (int32_t)tz_lookup(e, epoch_sec)->is_dst : 0;
}

void kestrel_tz_name(int64_t tz_id, char* buf, int64_t buf_len) {
    tz_ensure_init();
    if (buf_len <= 0) return;
    const struct tz_entry* e = tz_get(tz_id);
    strncpy(buf, e ? e->name : "", (size_t)(buf_len - 1));
    buf[buf_len - 1] = '\0';
}

void kestrel_tz_abbr(int64_t tz_id, int64_t epoch_sec, char* buf, int64_t buf_len) {
    tz_ensure_init();
    if (buf_len <= 0) return;
    const struct tz_entry* e = tz_get(tz_id);
    const struct tz_ttinfo* info = e ? tz_lookup(e, epoch_sec) : NULL;
    const char* abbr = info && info->abbr_idx < e->abbr_len ? e->abbreviations + info->abbr_idx : "";
    strncpy(buf, abbr, (size_t)(buf_len - 1));
    buf[buf_len - 1] = '\0';
}

// If the wall-clock time `naive_secs` (civil seconds read as if UTC) falls in
// a DST gap (skipped) or fold (repeated) of zone `tz_id`, write the UTC
// offsets before/after that transition and return 1; otherwise return 0.
// Offsets are under 26h, so only transitions within two days can matter:
// binary-search to the window instead of scanning the table.
int32_t kestrel_tz_bracketing_transition(int64_t tz_id, int64_t naive_secs,
                                         int32_t* offset_before_out, int32_t* offset_after_out) {
    tz_ensure_init();
    const struct tz_entry* e = tz_get(tz_id);
    if (!e) return 0;
    const int64_t window = 2 * 86400;
    int64_t i = tz_transition_index(e, naive_secs - window) + 1;
    for (; i < e->num_transitions && e->transitions[i] <= naive_secs + window; i++) {
        int32_t before = e->ttinfos[i > 0 ? e->transition_types[i - 1] : 0].utoff;
        int32_t after = e->ttinfos[e->transition_types[i]].utoff;
        int64_t civil_before = e->transitions[i] + before;
        int64_t civil_after = e->transitions[i] + after;
        // Gap: [civil_before, civil_after) never occurs. Fold: [civil_after, civil_before) occurs twice.
        int in_gap = after > before && naive_secs >= civil_before && naive_secs < civil_after;
        int in_fold = after < before && naive_secs >= civil_after && naive_secs < civil_before;
        if (in_gap || in_fold) {
            *offset_before_out = before;
            *offset_after_out = after;
            return 1;
        }
    }
    return 0;
}

// Get system timezone name by reading /etc/localtime symlink or TZ env var
void kestrel_system_timezone_name(char* buf, int64_t buf_len) {
    if (buf_len <= 0) return;

    // Try TZ environment variable first
    const char* tz_env = getenv("TZ");
    if (tz_env && tz_env[0] != '\0') {
        // Strip leading ':' if present (POSIX convention)
        if (tz_env[0] == ':') tz_env++;
        strncpy(buf, tz_env, (size_t)(buf_len - 1));
        buf[buf_len - 1] = '\0';
        return;
    }

    // Try reading /etc/localtime symlink
    char link_target[512];
    ssize_t link_len = readlink("/etc/localtime", link_target, sizeof(link_target) - 1);
    if (link_len > 0) {
        link_target[link_len] = '\0';
        // Extract IANA name from path like /usr/share/zoneinfo/America/New_York
        const char* marker = "/zoneinfo/";
        char* pos = strstr(link_target, marker);
        if (pos) {
            pos += strlen(marker);
            strncpy(buf, pos, (size_t)(buf_len - 1));
            buf[buf_len - 1] = '\0';
            return;
        }
    }

    // Fallback to UTC
    strncpy(buf, "UTC", (size_t)(buf_len - 1));
    buf[buf_len - 1] = '\0';
}

// ============================================================================
// Zone enumeration
// ============================================================================

struct tz_name_list {
    char** names;
    size_t count;
    size_t capacity;
};

static void tz_name_list_push(struct tz_name_list* list, const char* name) {
    if (list->count == list->capacity) {
        size_t cap = list->capacity ? list->capacity * 2 : 512;
        char** grown = (char**)realloc(list->names, cap * sizeof(char*));
        if (!grown) return;
        list->names = grown;
        list->capacity = cap;
    }
    char* copy = strdup(name);
    if (copy) list->names[list->count++] = copy;
}

// True if `path` is a regular file starting with the TZif magic.
static int tz_is_tzif_file(const char* path) {
    FILE* f = fopen(path, "rb");
    if (!f) return 0;
    char magic[4] = {0};
    size_t n = fread(magic, 1, 4, f);
    fclose(f);
    return n == 4 && memcmp(magic, "TZif", 4) == 0;
}

// Recursively collect zone names relative to the zoneinfo root. Skips the
// "posix"/"right" mirror trees and the non-zone aliases localtime/posixrules/Factory.
static void tz_collect(const char* root, const char* rel, struct tz_name_list* list) {
    char dir_path[1024];
    snprintf(dir_path, sizeof(dir_path), "%s%s%s", root, rel[0] ? "/" : "", rel);
    DIR* dir = opendir(dir_path);
    if (!dir) return;
    struct dirent* ent;
    while ((ent = readdir(dir)) != NULL) {
        const char* name = ent->d_name;
        if (name[0] == '.' || name[0] == '+') continue;
        if (!rel[0] && (strcmp(name, "posix") == 0 || strcmp(name, "right") == 0 ||
                        strcmp(name, "localtime") == 0 || strcmp(name, "posixrules") == 0 ||
                        strcmp(name, "Factory") == 0)) continue;
        char child_rel[512];
        snprintf(child_rel, sizeof(child_rel), "%s%s%s", rel, rel[0] ? "/" : "", name);
        char child_path[1024];
        snprintf(child_path, sizeof(child_path), "%s/%s", root, child_rel);
        struct stat st;
        if (stat(child_path, &st) != 0) continue;
        if (S_ISDIR(st.st_mode)) {
            tz_collect(root, child_rel, list);
        } else if (S_ISREG(st.st_mode) && tz_is_tzif_file(child_path)) {
            tz_name_list_push(list, child_rel);
        }
    }
    closedir(dir);
}

static int tz_name_cmp(const void* a, const void* b) {
    return strcmp(*(const char* const*)a, *(const char* const*)b);
}

// Write every zone name under /usr/share/zoneinfo, sorted and newline-separated,
// NUL-terminated, into `buf`. Returns the byte length the full list needs
// (including the NUL); call with a small buffer first to size it.
int64_t kestrel_tz_list_identifiers(char* buf, int64_t buf_len) {
    struct tz_name_list list = {0};
    tz_collect("/usr/share/zoneinfo", "", &list);
    qsort(list.names, list.count, sizeof(char*), tz_name_cmp);

    int64_t needed = 1;
    for (size_t i = 0; i < list.count; i++) {
        needed += (int64_t)strlen(list.names[i]) + (i + 1 < list.count ? 1 : 0);
    }
    if (buf_len >= needed) {
        char* out = buf;
        for (size_t i = 0; i < list.count; i++) {
            size_t len = strlen(list.names[i]);
            memcpy(out, list.names[i], len);
            out += len;
            if (i + 1 < list.count) *out++ = '\n';
        }
        *out = '\0';
    }
    for (size_t i = 0; i < list.count; i++) free(list.names[i]);
    free(list.names);
    return needed;
}
