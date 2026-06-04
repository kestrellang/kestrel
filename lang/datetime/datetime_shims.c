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
// System timezone helpers
// ============================================================================

int64_t kestrel_localtime_gmtoff(int64_t epoch_sec) {
    time_t t = (time_t)epoch_sec;
    struct tm result;
    localtime_r(&t, &result);
    return (int64_t)result.tm_gmtoff;
}

void kestrel_localtime_zone(int64_t epoch_sec, char* buf, int64_t buf_len) {
    time_t t = (time_t)epoch_sec;
    struct tm result;
    localtime_r(&t, &result);
    if (result.tm_zone && buf_len > 0) {
        strncpy(buf, result.tm_zone, (size_t)(buf_len - 1));
        buf[buf_len - 1] = '\0';
    } else if (buf_len > 0) {
        buf[0] = '\0';
    }
}

// ============================================================================
// Timezone registry — TZif parsing + offset lookup
// ============================================================================

#define MAX_TIMEZONES 256
#define MAX_TRANSITIONS 2048

struct tz_ttinfo {
    int32_t utoff;
    int8_t  is_dst;
    uint8_t abbr_idx;
};

struct tz_entry {
    char name[128];
    int64_t transitions[MAX_TRANSITIONS];
    uint8_t transition_types[MAX_TRANSITIONS];
    struct tz_ttinfo ttinfos[128];
    int64_t num_transitions;
    int64_t num_ttinfos;
    char abbreviations[512];
    int64_t abbr_len;
    int     loaded;
};

static struct tz_entry tz_registry[MAX_TIMEZONES];
static int64_t tz_count = 0;
static int tz_initialized = 0;

static void tz_ensure_init(void) {
    if (tz_initialized) return;
    tz_initialized = 1;
    // Register UTC as ID 0
    memset(&tz_registry[0], 0, sizeof(struct tz_entry));
    strncpy(tz_registry[0].name, "UTC", sizeof(tz_registry[0].name));
    tz_registry[0].num_transitions = 0;
    tz_registry[0].num_ttinfos = 1;
    tz_registry[0].ttinfos[0].utoff = 0;
    tz_registry[0].ttinfos[0].is_dst = 0;
    tz_registry[0].ttinfos[0].abbr_idx = 0;
    strncpy(tz_registry[0].abbreviations, "UTC", sizeof(tz_registry[0].abbreviations));
    tz_registry[0].abbr_len = 4;
    tz_registry[0].loaded = 1;
    tz_count = 1;
}

// Read a big-endian 32-bit integer
static int32_t read_be32(const uint8_t* p) {
    return (int32_t)(((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
                     ((uint32_t)p[2] << 8)  | ((uint32_t)p[3]));
}

// Read a big-endian 64-bit integer
static int64_t read_be64(const uint8_t* p) {
    return (int64_t)(((uint64_t)p[0] << 56) | ((uint64_t)p[1] << 48) |
                     ((uint64_t)p[2] << 40) | ((uint64_t)p[3] << 32) |
                     ((uint64_t)p[4] << 24) | ((uint64_t)p[5] << 16) |
                     ((uint64_t)p[6] << 8)  | ((uint64_t)p[7]));
}

// Parse a TZif file into a tz_entry. Returns 0 on success, -1 on failure.
static int parse_tzif(const uint8_t* data, size_t len, struct tz_entry* entry) {
    if (len < 44) return -1;
    // Check magic "TZif"
    if (memcmp(data, "TZif", 4) != 0) return -1;

    char version = data[4];
    // We need v2 or v3 for 64-bit timestamps
    if (version != '2' && version != '3') {
        // Fall back to v1 (32-bit) if no v2/v3
        version = '1';
    }

    // Parse v1 header to skip its data block
    int32_t v1_tzh_ttisutcnt = read_be32(data + 20);
    int32_t v1_tzh_ttisstdcnt = read_be32(data + 24);
    int32_t v1_tzh_leapcnt = read_be32(data + 28);
    int32_t v1_tzh_timecnt = read_be32(data + 32);
    int32_t v1_tzh_typecnt = read_be32(data + 36);
    int32_t v1_tzh_charcnt = read_be32(data + 40);

    size_t v1_datablock_size =
        (size_t)v1_tzh_timecnt * 4 +       // transition times (32-bit)
        (size_t)v1_tzh_timecnt * 1 +        // transition type indices
        (size_t)v1_tzh_typecnt * 6 +        // ttinfos
        (size_t)v1_tzh_charcnt +            // abbreviations
        (size_t)v1_tzh_leapcnt * 8 +        // leap seconds
        (size_t)v1_tzh_ttisstdcnt +         // std/wall indicators
        (size_t)v1_tzh_ttisutcnt;           // UT/local indicators

    if (version == '1') {
        // Use v1 data (32-bit timestamps)
        const uint8_t* p = data + 44;
        int64_t timecnt = v1_tzh_timecnt;
        int64_t typecnt = v1_tzh_typecnt;
        int64_t charcnt = v1_tzh_charcnt;

        if (timecnt > MAX_TRANSITIONS) timecnt = MAX_TRANSITIONS;
        entry->num_transitions = timecnt;

        for (int64_t i = 0; i < timecnt; i++) {
            entry->transitions[i] = (int64_t)read_be32(p);
            p += 4;
        }
        for (int64_t i = 0; i < timecnt; i++) {
            entry->transition_types[i] = *p++;
        }
        entry->num_ttinfos = typecnt;
        for (int64_t i = 0; i < typecnt && i < 128; i++) {
            entry->ttinfos[i].utoff = read_be32(p);
            entry->ttinfos[i].is_dst = (int8_t)p[4];
            entry->ttinfos[i].abbr_idx = p[5];
            p += 6;
        }
        if (charcnt > 511) charcnt = 511;
        memcpy(entry->abbreviations, p, (size_t)charcnt);
        entry->abbreviations[charcnt] = '\0';
        entry->abbr_len = charcnt;
        entry->loaded = 1;
        return 0;
    }

    // Skip v1 data block to reach v2/v3 header
    size_t v2_offset = 44 + v1_datablock_size;
    if (v2_offset + 44 > len) return -1;

    const uint8_t* v2 = data + v2_offset;
    if (memcmp(v2, "TZif", 4) != 0) return -1;

    int32_t tzh_ttisutcnt = read_be32(v2 + 20);
    int32_t tzh_ttisstdcnt = read_be32(v2 + 24);
    (void)tzh_ttisstdcnt;
    int32_t tzh_leapcnt = read_be32(v2 + 28);
    int32_t tzh_timecnt = read_be32(v2 + 32);
    int32_t tzh_typecnt = read_be32(v2 + 36);
    int32_t tzh_charcnt = read_be32(v2 + 40);

    const uint8_t* p = v2 + 44;
    int64_t timecnt = tzh_timecnt;
    int64_t typecnt = tzh_typecnt;
    int64_t charcnt = tzh_charcnt;

    // Check bounds
    size_t needed = (size_t)timecnt * 8 + (size_t)timecnt + (size_t)typecnt * 6 +
                    (size_t)charcnt + (size_t)tzh_leapcnt * 12 +
                    (size_t)tzh_ttisstdcnt + (size_t)tzh_ttisutcnt;
    if (v2_offset + 44 + needed > len) return -1;

    if (timecnt > MAX_TRANSITIONS) timecnt = MAX_TRANSITIONS;
    entry->num_transitions = timecnt;

    // Transition times (64-bit)
    for (int64_t i = 0; i < timecnt; i++) {
        entry->transitions[i] = read_be64(p);
        p += 8;
    }
    // Transition type indices
    for (int64_t i = 0; i < timecnt; i++) {
        entry->transition_types[i] = *p++;
    }
    // Skip remaining transition types if we truncated
    if (tzh_timecnt > MAX_TRANSITIONS) {
        p += (tzh_timecnt - MAX_TRANSITIONS);
    }

    // ttinfo structures
    entry->num_ttinfos = typecnt;
    for (int64_t i = 0; i < typecnt && i < 128; i++) {
        entry->ttinfos[i].utoff = read_be32(p);
        entry->ttinfos[i].is_dst = (int8_t)p[4];
        entry->ttinfos[i].abbr_idx = p[5];
        p += 6;
    }

    // Abbreviation strings
    if (charcnt > 511) charcnt = 511;
    memcpy(entry->abbreviations, p, (size_t)charcnt);
    entry->abbreviations[charcnt] = '\0';
    entry->abbr_len = charcnt;

    entry->loaded = 1;
    return 0;
}

// Register a timezone by loading its TZif file from /usr/share/zoneinfo/
int64_t kestrel_tz_register(const char* name) {
    tz_ensure_init();

    if (tz_count >= MAX_TIMEZONES) return -1;

    char path[512];
    snprintf(path, sizeof(path), "/usr/share/zoneinfo/%s", name);

    FILE* f = fopen(path, "rb");
    if (!f) return -1;

    fseek(f, 0, SEEK_END);
    long file_len = ftell(f);
    fseek(f, 0, SEEK_SET);

    if (file_len <= 0 || file_len > 1024 * 1024) {
        fclose(f);
        return -1;
    }

    uint8_t* data = (uint8_t*)malloc((size_t)file_len);
    if (!data) { fclose(f); return -1; }

    size_t read_len = fread(data, 1, (size_t)file_len, f);
    fclose(f);

    if ((long)read_len != file_len) { free(data); return -1; }

    int64_t id = tz_count;
    struct tz_entry* entry = &tz_registry[id];
    memset(entry, 0, sizeof(struct tz_entry));
    strncpy(entry->name, name, sizeof(entry->name) - 1);

    int result = parse_tzif(data, (size_t)file_len, entry);
    free(data);

    if (result != 0) return -1;

    tz_count++;
    return id;
}

// Find a timezone by name, returns ID or -1 if not found
int64_t kestrel_tz_find(const char* name) {
    tz_ensure_init();
    for (int64_t i = 0; i < tz_count; i++) {
        if (strcmp(tz_registry[i].name, name) == 0) return i;
    }
    return -1;
}

// Find or register a timezone by name
int64_t kestrel_tz_find_or_register(const char* name) {
    int64_t id = kestrel_tz_find(name);
    if (id >= 0) return id;
    return kestrel_tz_register(name);
}

// Binary search for offset at a given epoch second
static const struct tz_ttinfo* tz_lookup(const struct tz_entry* entry, int64_t epoch_sec) {
    if (entry->num_transitions == 0) {
        return (entry->num_ttinfos > 0) ? &entry->ttinfos[0] : NULL;
    }

    // Binary search for the last transition <= epoch_sec
    int64_t lo = 0, hi = entry->num_transitions - 1;
    if (epoch_sec < entry->transitions[0]) {
        // Before first transition — use first ttinfo
        return &entry->ttinfos[entry->transition_types[0]];
    }
    if (epoch_sec >= entry->transitions[hi]) {
        return &entry->ttinfos[entry->transition_types[hi]];
    }

    while (lo < hi) {
        int64_t mid = lo + (hi - lo + 1) / 2;
        if (entry->transitions[mid] <= epoch_sec) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    return &entry->ttinfos[entry->transition_types[lo]];
}

int32_t kestrel_tz_offset(int64_t tz_id, int64_t epoch_sec) {
    tz_ensure_init();
    if (tz_id < 0 || tz_id >= tz_count) return 0;
    const struct tz_ttinfo* info = tz_lookup(&tz_registry[tz_id], epoch_sec);
    return info ? info->utoff : 0;
}

int32_t kestrel_tz_is_dst(int64_t tz_id, int64_t epoch_sec) {
    tz_ensure_init();
    if (tz_id < 0 || tz_id >= tz_count) return 0;
    const struct tz_ttinfo* info = tz_lookup(&tz_registry[tz_id], epoch_sec);
    return info ? (int32_t)info->is_dst : 0;
}

void kestrel_tz_name(int64_t tz_id, char* buf, int64_t buf_len) {
    tz_ensure_init();
    if (tz_id < 0 || tz_id >= tz_count || buf_len <= 0) {
        if (buf_len > 0) buf[0] = '\0';
        return;
    }
    strncpy(buf, tz_registry[tz_id].name, (size_t)(buf_len - 1));
    buf[buf_len - 1] = '\0';
}

void kestrel_tz_abbr(int64_t tz_id, int64_t epoch_sec, char* buf, int64_t buf_len) {
    tz_ensure_init();
    if (tz_id < 0 || tz_id >= tz_count || buf_len <= 0) {
        if (buf_len > 0) buf[0] = '\0';
        return;
    }
    const struct tz_ttinfo* info = tz_lookup(&tz_registry[tz_id], epoch_sec);
    if (info && info->abbr_idx < tz_registry[tz_id].abbr_len) {
        const char* abbr = tz_registry[tz_id].abbreviations + info->abbr_idx;
        strncpy(buf, abbr, (size_t)(buf_len - 1));
        buf[buf_len - 1] = '\0';
    } else {
        buf[0] = '\0';
    }
}

// Get the number of transitions for a timezone (used for gap/fold detection)
int64_t kestrel_tz_transition_count(int64_t tz_id) {
    tz_ensure_init();
    if (tz_id < 0 || tz_id >= tz_count) return 0;
    return tz_registry[tz_id].num_transitions;
}

// Get a specific transition's epoch second and offset info
void kestrel_tz_transition_at(int64_t tz_id, int64_t index,
                               int64_t* epoch_out, int32_t* offset_before_out,
                               int32_t* offset_after_out) {
    tz_ensure_init();
    if (tz_id < 0 || tz_id >= tz_count || index < 0 ||
        index >= tz_registry[tz_id].num_transitions) {
        *epoch_out = 0;
        *offset_before_out = 0;
        *offset_after_out = 0;
        return;
    }
    const struct tz_entry* e = &tz_registry[tz_id];
    *epoch_out = e->transitions[index];
    *offset_after_out = e->ttinfos[e->transition_types[index]].utoff;
    if (index > 0) {
        *offset_before_out = e->ttinfos[e->transition_types[index - 1]].utoff;
    } else if (e->num_ttinfos > 0) {
        *offset_before_out = e->ttinfos[0].utoff;
    } else {
        *offset_before_out = 0;
    }
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
