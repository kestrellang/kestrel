// C helpers behind std.system.
//
// Each function here exists for one of three reasons: the libc call takes a
// struct whose layout differs per target (posix_spawn attributes, file
// actions), the answer is a target fact the C compiler already knows
// (architecture, pointer width), or the platforms disagree on how to get it
// (executable path, argv). Kestrel code calls only these fixed-signature
// wrappers, so no struct offsets are hand-coded in .ks files.

#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <pwd.h>
#include <sched.h>
#include <signal.h>
#include <spawn.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#ifdef __APPLE__
#include <crt_externs.h>
#include <mach-o/dyld.h>
#define KESTREL_ENVIRON (*_NSGetEnviron())
#else
extern char **environ;
#define KESTREL_ENVIRON environ
#endif

// ---------------------------------------------------------------------------
// argv capture
//
// glibc and dyld both pass (argc, argv, envp) to initializers, so one
// constructor captures argv before main runs on both platforms.
// ---------------------------------------------------------------------------

static int kestrel_saved_argc = 0;
static char **kestrel_saved_argv = NULL;

__attribute__((constructor)) static void kestrel_capture_argv(int argc, char **argv, char **envp) {
    (void)envp;
    kestrel_saved_argc = argc;
    kestrel_saved_argv = argv;
}

int kestrel_argc(void) { return kestrel_saved_argc; }
char **kestrel_argv(void) { return kestrel_saved_argv; }
char **kestrel_environ(void) { return KESTREL_ENVIRON; }

// ---------------------------------------------------------------------------
// Process spawning
// ---------------------------------------------------------------------------

// Stdio modes, matching `Stdio` in process.ks.
enum { KESTREL_STDIO_INHERIT = 0, KESTREL_STDIO_PIPE = 1, KESTREL_STDIO_NULL = 2 };

static int kestrel_cloexec_pipe(int fds[2]) {
#ifdef __APPLE__
    if (pipe(fds) != 0) return -1;
    fcntl(fds[0], F_SETFD, FD_CLOEXEC);
    fcntl(fds[1], F_SETFD, FD_CLOEXEC);
    return 0;
#else
    return pipe2(fds, O_CLOEXEC);
#endif
}

// Spawns `program` (PATH lookup unless it contains '/') with `argv`.
// `envp` NULL means inherit; `cwd` NULL means the parent's directory.
// `modes[i]` sets fd i (0, 1, 2). For each piped fd, the parent's end is
// written to `parent_fds[i]`; others get -1. Returns 0 or an errno value.
int kestrel_spawn(const char *program, char *const *argv, char *const *envp, const char *cwd,
                  const int32_t *modes, int32_t *pid_out, int32_t *parent_fds) {
    int pipes[3][2] = {{-1, -1}, {-1, -1}, {-1, -1}};
    posix_spawn_file_actions_t actions;
    int err = posix_spawn_file_actions_init(&actions);
    if (err != 0) return err;

    for (int i = 0; i < 3; i++) {
        parent_fds[i] = -1;
        if (modes[i] == KESTREL_STDIO_PIPE) {
            if (kestrel_cloexec_pipe(pipes[i]) != 0) { err = errno; goto fail; }
            // fd 0 reads from the pipe; fds 1 and 2 write into it.
            int child_end = (i == 0) ? pipes[i][0] : pipes[i][1];
            err = posix_spawn_file_actions_adddup2(&actions, child_end, i);
            if (err != 0) goto fail;
        } else if (modes[i] == KESTREL_STDIO_NULL) {
            err = posix_spawn_file_actions_addopen(&actions, i, "/dev/null", i == 0 ? O_RDONLY : O_WRONLY, 0);
            if (err != 0) goto fail;
        }
    }
    if (cwd != NULL) {
        err = posix_spawn_file_actions_addchdir_np(&actions, cwd);
        if (err != 0) goto fail;
    }

    pid_t pid;
    err = posix_spawnp(&pid, program, &actions, NULL, argv, envp != NULL ? envp : KESTREL_ENVIRON);
    if (err != 0) goto fail;

    posix_spawn_file_actions_destroy(&actions);
    for (int i = 0; i < 3; i++) {
        if (pipes[i][0] < 0) continue;
        close(i == 0 ? pipes[i][0] : pipes[i][1]);
        parent_fds[i] = (i == 0) ? pipes[i][1] : pipes[i][0];
    }
    *pid_out = (int32_t)pid;
    return 0;

fail:
    posix_spawn_file_actions_destroy(&actions);
    for (int i = 0; i < 3; i++) {
        if (pipes[i][0] >= 0) close(pipes[i][0]);
        if (pipes[i][1] >= 0) close(pipes[i][1]);
    }
    return err;
}

// Reaps `pid`. Returns 1 when it has exited (`kind` 1 = exited with `value`
// as the code, 2 = ended by signal `value`), 0 when `nohang` and still
// running, -1 on error (errno set).
int kestrel_wait(int32_t pid, int32_t nohang, int32_t *kind, int32_t *value) {
    int status = 0;
    pid_t r;
    do {
        r = waitpid((pid_t)pid, &status, nohang ? WNOHANG : 0);
    } while (r < 0 && errno == EINTR);
    if (r < 0) return -1;
    if (r == 0) return 0;
    if (WIFEXITED(status)) { *kind = 1; *value = WEXITSTATUS(status); }
    else if (WIFSIGNALED(status)) { *kind = 2; *value = WTERMSIG(status); }
    else { *kind = 2; *value = 0; }
    return 1;
}

// Waits until either fd is readable or closed, or `timeout_ms` passes
// (-1 = forever). A negative fd is skipped. Returns poll's count, 0 on
// timeout, -1 on error; `ready[i]` is 1 for each fd with data or hangup.
int kestrel_poll_two(int32_t fd0, int32_t fd1, int32_t timeout_ms, int32_t *ready) {
    struct pollfd fds[2] = {{fd0, POLLIN, 0}, {fd1, POLLIN, 0}};
    int r;
    do {
        r = poll(fds, 2, timeout_ms);
    } while (r < 0 && errno == EINTR);
    ready[0] = (fd0 >= 0 && fds[0].revents != 0) ? 1 : 0;
    ready[1] = (fd1 >= 0 && fds[1].revents != 0) ? 1 : 0;
    return r;
}

void kestrel_sleep_ms(int32_t ms) {
    struct timespec ts = {ms / 1000, (long)(ms % 1000) * 1000000L};
    while (nanosleep(&ts, &ts) != 0 && errno == EINTR) {}
}


// Milliseconds on a clock that never jumps, for timeouts.
int64_t kestrel_monotonic_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

// ---------------------------------------------------------------------------
// This process and machine
// ---------------------------------------------------------------------------

// Writes the running executable's absolute path into `buf` (not NUL
// terminated). Returns its length, or -1 if it doesn't fit or the lookup
// failed.
int64_t kestrel_executable_path(char *buf, int64_t size) {
#ifdef __APPLE__
    uint32_t n = (uint32_t)size;
    if (_NSGetExecutablePath(buf, &n) != 0) return -1;
    return (int64_t)strlen(buf);
#else
    ssize_t n = readlink("/proc/self/exe", buf, (size_t)size);
    if (n < 0 || n >= size) return -1;
    return (int64_t)n;
#endif
}

// The current user's home directory from the password database, or NULL.
const char *kestrel_passwd_home(void) {
    struct passwd *pw = getpwuid(getuid());
    return pw != NULL ? pw->pw_dir : NULL;
}

// Processors this process may run on: the affinity mask on Linux, the
// online count elsewhere. Never less than 1.
int64_t kestrel_processor_count(void) {
#ifdef __linux__
    cpu_set_t set;
    if (sched_getaffinity(0, sizeof(set), &set) == 0) {
        int n = CPU_COUNT(&set);
        if (n > 0) return n;
    }
#endif
    long n = sysconf(_SC_NPROCESSORS_ONLN);
    return n > 0 ? n : 1;
}

int64_t kestrel_page_size(void) { return (int64_t)sysconf(_SC_PAGESIZE); }

// ---------------------------------------------------------------------------
// Target facts, answered by the C compiler for the real target.
// ---------------------------------------------------------------------------

// Matches the case order of `Architecture` in platform.ks; -1 = unknown.
int32_t kestrel_architecture(void) {
#if defined(__x86_64__)
    return 0;
#elif defined(__aarch64__)
    return 1;
#elif defined(__i386__)
    return 2;
#elif defined(__arm__)
    return 3;
#elif defined(__riscv) && __riscv_xlen == 64
    return 4;
#elif defined(__wasm32__)
    return 5;
#else
    return -1;
#endif
}

int32_t kestrel_pointer_width(void) { return (int32_t)(sizeof(void *) * 8); }

int32_t kestrel_is_little_endian(void) {
    const uint16_t probe = 1;
    return *(const uint8_t *)&probe == 1;
}
