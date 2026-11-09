#!/usr/bin/env python3
"""Runs tc39/test262 tests through `sojourn-js run`.

Usage: tools/test262.py TEST262_DIR [SUBDIR...] [--jobs N] [--verbose]
       [--failures FILE]

SUBDIRs are relative to TEST262_DIR/test (default: language built-ins).
Each test runs in a fresh process with the harness files prepended, in
strict mode when its flags ask for it, and non-strict otherwise (test262
asks for both; this runner only does one). Tests needing features the
engine doesn't have yet are skipped.
"""
import concurrent.futures, os, re, resource, subprocess, sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
BINARY = os.path.join(HERE, "..", "build", "sojourn-js")
TIMEOUT = 10
MEMORY = 1 << 30  # bytes per test process

def limit_memory():
    resource.setrlimit(resource.RLIMIT_AS, (MEMORY, MEMORY))

# Features the engine doesn't support yet.
UNSUPPORTED = {
    "generators", "async-functions", "async-iteration", "class-fields-private",
    "class-methods-private", "class-static-methods-private", "class-fields-private-in",
    "BigInt", "Proxy", "Promise", "regexp-named-groups", "regexp-unicode-property-escapes",
    "regexp-lookbehind", "regexp-dotall", "regexp-match-indices", "regexp-v-flag",
    "regexp-modifiers", "regexp-duplicate-named-groups", "TypedArray", "ArrayBuffer",
    "DataView", "SharedArrayBuffer", "Atomics", "WeakRef", "FinalizationRegistry",
    "WeakMap", "WeakSet", "Map", "Set", "tail-call-optimization", "dynamic-import",
    "import-meta", "top-level-await", "Temporal", "ShadowRealm", "decorators",
    "Intl", "intl-normative-optional", "cross-realm", "IsHTMLDDA", "host-gc-required",
    "explicit-resource-management", "promise-with-resolvers", "Array.fromAsync",
    "iterator-helpers", "set-methods", "uint8array-base64", "resizable-arraybuffer",
    "arraybuffer-transfer", "Float16Array", "json-parse-with-source", "RegExp.escape",
    "Symbol.asyncIterator", "async-generators", "class-static-block", "regexp-sticky",
    "String.prototype.matchAll", "Symbol.match", "Symbol.replace", "Symbol.search",
    "Symbol.split", "Symbol.matchAll", "Reflect.construct", "error-cause",
    "Atomics.waitAsync", "change-array-by-copy", "array-grouping", "source-phase-imports",
    "import-attributes", "json-modules", "upsert", "Math.sumPrecise", "Error.isError",
    "promise-try", "iterator-sequencing", "await-dictionary", "immutable-arraybuffer",
    "canonical-tz", "legacy-regexp",
}

FRONTMATTER = re.compile(r"/\*---(.*?)---\*/", re.S)

def meta_list(meta, key):
    """A YAML list in either `[a, b]` or `- a` form."""
    m = re.search(r"^\s*%s:\s*\[(.*?)\]" % key, meta, re.M | re.S)
    if m:
        return [x.strip() for x in m.group(1).split(",") if x.strip()]
    m = re.search(r"^\s*%s:\s*\n((?:\s+-.*\n?)+)" % key, meta, re.M)
    if m:
        return [l.strip()[1:].strip() for l in m.group(1).splitlines() if l.strip()]
    return []

def negative(meta):
    m = re.search(r"negative:\s*\n\s+phase:\s*(\w+)\s*\n\s+type:\s*(\w+)", meta)
    if not m:
        m2 = re.search(r"negative:\s*\n\s+type:\s*(\w+)\s*\n\s+phase:\s*(\w+)", meta)
        return (m2.group(2), m2.group(1)) if m2 else None
    return (m.group(1), m.group(2))

def run_test(root, path):
    src = open(path, encoding="utf-8", errors="replace").read()
    m = FRONTMATTER.search(src)
    meta = m.group(1) if m else ""
    flags = set(meta_list(meta, "flags"))
    features = set(meta_list(meta, "features"))
    if flags & {"module", "async", "CanBlockIsTrue"}:
        return "skip", "flags"
    bad = features & UNSUPPORTED
    if bad:
        return "skip", sorted(bad)[0]
    harness = os.path.join(root, "harness")
    files = []
    if "raw" not in flags:
        files = [os.path.join(harness, "assert.js"), os.path.join(harness, "sta.js")]
        files += [os.path.join(harness, f) for f in meta_list(meta, "includes")]
    cmd = [BINARY, "run"] + (["--strict"] if "onlyStrict" in flags else []) + files + [path]
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=TIMEOUT, preexec_fn=limit_memory)
    except subprocess.TimeoutExpired:
        return "fail", "timeout"
    out = p.stdout.decode("utf-8", "replace")
    err = p.stderr.decode("utf-8", "replace")
    uncaught = re.search(r"^Uncaught (.*)$", out, re.M)
    neg = negative(meta)
    if p.returncode not in (0, 1) or (p.returncode == 1 and not uncaught):
        return "fail", "crash: " + (err.strip().splitlines() or ["exit %d" % p.returncode])[-1][:200]
    if neg:
        phase, kind = neg
        if not uncaught:
            return "fail", "expected %s %s" % (phase, kind)
        if not uncaught.group(1).startswith(kind):
            return "fail", "expected %s, got %s" % (kind, uncaught.group(1)[:200])
        return "pass", ""
    if uncaught:
        return "fail", uncaught.group(1)[:300]
    return "pass", ""

def main():
    args = sys.argv[1:]
    verbose = "--verbose" in args
    jobs = os.cpu_count() or 4
    failures_file = None
    if "--jobs" in args:
        jobs = int(args[args.index("--jobs") + 1])
    if "--failures" in args:
        failures_file = args[args.index("--failures") + 1]
    positional = []
    skip_next = False
    for a in args:
        if skip_next:
            skip_next = False
            continue
        if a in ("--jobs", "--failures"):
            skip_next = True
            continue
        if not a.startswith("--"):
            positional.append(a)
    root = positional[0]
    subdirs = positional[1:] or ["language", "built-ins"]
    tests = []
    for sub in subdirs:
        base = os.path.join(root, "test", sub)
        if os.path.isfile(base):
            tests.append(base)
            continue
        for d, _, names in os.walk(base):
            for n in names:
                if n.endswith(".js") and "_FIXTURE" not in n:
                    tests.append(os.path.join(d, n))
    tests.sort()
    counts = Counter()
    skips = Counter()
    failures = []
    with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
        for path, (status, why) in zip(tests, pool.map(lambda t: run_test(root, t), tests)):
            counts[status] += 1
            rel = os.path.relpath(path, os.path.join(root, "test"))
            if status == "skip":
                skips[why if isinstance(why, str) else why] += 1
            elif status == "fail":
                failures.append((rel, why))
                if verbose:
                    print("FAIL %s: %s" % (rel, why))
    ran = counts["pass"] + counts["fail"]
    print("%d tests: %d passed, %d failed, %d skipped (%.1f%% of those run)" % (
        len(tests), counts["pass"], counts["fail"], counts["skip"],
        100.0 * counts["pass"] / ran if ran else 0))
    if failures_file:
        with open(failures_file, "w") as f:
            for rel, why in failures:
                f.write("%s\t%s\n" % (rel, why))
    # Most common failure reasons, with digits squashed so similar ones group.
    reasons = Counter(re.sub(r"\d+", "N", why)[:120] for _, why in failures)
    for why, n in reasons.most_common(25):
        print("%6d  %s" % (n, why))

if __name__ == "__main__":
    main()
