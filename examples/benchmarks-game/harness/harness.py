#!/usr/bin/env python3
"""Build and time Benchmarks Game programs on one CPU core.

Steps:
  build   - compile every program for every language
  select  - run each at a reduced size, check output, keep the fastest per language
  full    - run each language's fastest program at the official size (min of N runs)
"""
import threading, json, os, shutil, subprocess, sys, time, glob, hashlib

ROOT = "/tmp/claude-0/bg"
SRC = f"{ROOT}/dl/x"
H = f"{ROOT}/h"
BUILD = f"{H}/build"
INP = f"{ROOT}/inputs"
KS = f"{ROOT}/ks"
DOTNET = "/opt/tc/dotnet/dotnet"
SWIFTC = "/opt/tc/swift/usr/bin/swiftc"
KESTREL = "/home/user/kestrel/target/release/kestrel"

# bench: (full args, full stdin, small args, small stdin)
BENCH = {
    "nbody":         (["50000000"], None, ["10000000"], None),
    "spectralnorm":  (["5500"], None, ["2500"], None),
    "fannkuchredux": (["12"], None, ["11"], None),
    "mandelbrot":    (["16000"], None, ["8000"], None),
    "fasta":         (["25000000"], None, ["5000000"], None),
    "knucleotide":   (["0"], "fasta25000000.txt", ["0"], "fasta5000000.txt"),
    "revcomp":       (["0"], "fasta25000000.txt", ["0"], "fasta5000000.txt"),
    "binarytrees":   (["21"], None, ["19"], None),
    "pidigits":      (["10000"], None, ["5000"], None),
    "regexredux":    (["0"], "fasta5000000.txt", ["0"], "fasta2500000.txt"),
}
LANGS = {"c": "gcc", "rust": "rust", "go": "go", "swift": "swift",
         "csharp": "csharp", "typescript": "typescript", "node": "node",
         "python": "python3"}

KS_LINK = {"pidigits": ["-l", "gmp"], "regexredux": ["-l", "pcre2-8"]}


def sh(cmd, cwd=None, timeout=900, env=None):
    e = dict(os.environ)
    if env:
        e.update(env)
    try:
        p = subprocess.run(cmd, cwd=cwd, shell=isinstance(cmd, str), capture_output=True,
                           timeout=timeout, env=e)
        return p.returncode, (p.stdout + p.stderr).decode(errors="replace")
    except subprocess.TimeoutExpired:
        return -999, "timeout"


def variants(bench, lang):
    ext = LANGS[lang]
    files = sorted(glob.glob(f"{SRC}/{bench}/{bench}.{ext}") +
                   glob.glob(f"{SRC}/{bench}/{bench}.{ext}-*.{ext}"))
    out = []
    for f in files:
        base = os.path.basename(f)
        v = "1" if base == f"{bench}.{ext}" else base.split("-")[-1].split(".")[0]
        out.append((v, f))
    return out


def load(name):
    p = f"{H}/{name}.json"
    return json.load(open(p)) if os.path.exists(p) else {}


def save(name, d):
    json.dump(d, open(f"{H}/{name}.json", "w"), indent=1)


# ---------------------------------------------------------------- build

def build_one(bench, lang, v, src):
    d = f"{BUILD}/{bench}/{lang}/{v}"
    os.makedirs(d, exist_ok=True)
    if lang == "c":
        shutil.copy(src, f"{d}/p.c")
        rc, log = sh(f"gcc -pipe -D_GNU_SOURCE -O3 -fomit-frame-pointer -march=native -fopenmp "
                     f"-I{ROOT}/inc -I/usr/include/apr-1.0 p.c -o exe "
                     f"-lm -lgmp -lpcre -lpcre2-8 -lapr-1 -lpthread", cwd=d)
        return rc, log, [f"{d}/exe"]
    if lang == "go":
        shutil.copy(src, f"{d}/main.go")
        if not os.path.exists(f"{d}/go.mod"):
            sh("go mod init bg", cwd=d)
        sh("go mod tidy", cwd=d, timeout=300)
        rc, log = sh("go build -o exe .", cwd=d, env={"GOAMD64": "v3"})
        return rc, log, [f"{d}/exe"]
    if lang == "swift":
        shutil.copy(src, f"{d}/main.swift")
        rc, log = sh(f"{SWIFTC} -Ounchecked -wmo -I {ROOT}/swiftmods/gmp -I {ROOT}/swiftmods/apr -Xcc -I/usr/include/apr-1.0 main.swift -o exe", cwd=d, timeout=600)
        return rc, log, [f"{d}/exe"]
    if lang == "csharp":
        shutil.copy(f"{H}/cs_template/bg.csproj", f"{d}/bg.csproj")
        shutil.copy(src, f"{d}/Program.cs")
        rc, log = sh(f"{DOTNET} build -c Release -o out", cwd=d, timeout=600)
        return rc, log[-3000:], [DOTNET, f"{d}/out/bg.dll"]
    if lang == "typescript":
        shutil.copy(src, f"{d}/main.ts")
        # Emit JS even when the checker complains; that's what tsc does by default.
        rc, log = sh(f"tsc --target es2022 --module commonjs --skipLibCheck --typeRoots {H}/ts/node_modules/@types "
                     f"--types node --outDir out main.ts", cwd=d)
        ok = os.path.exists(f"{d}/out/main.js")
        os.makedirs(f"{d}/out/node_modules", exist_ok=True)
        if not os.path.exists(f"{d}/out/node_modules/mpzjs"):
            os.symlink(f"{H}/ts/node_modules/mpzjs", f"{d}/out/node_modules/mpzjs")
        return (0 if ok else 1), log[-3000:], ["node", f"{d}/out/main.js"]
    if lang == "node":
        shutil.copy(src, f"{d}/main.js")
        os.makedirs(f"{d}/node_modules", exist_ok=True)
        if not os.path.exists(f"{d}/node_modules/mpzjs"):
            os.symlink(f"{H}/ts/node_modules/mpzjs", f"{d}/node_modules/mpzjs")
        return 0, "", ["node", f"{d}/main.js"]
    if lang == "python":
        shutil.copy(src, f"{d}/main.py")
        return 0, "", ["python3", f"{d}/main.py"]
    raise ValueError(lang)


def build_rust(todo):
    """All Rust programs share one cargo project; --keep-going builds the rest if one fails."""
    bins = f"{H}/rust/src/bin"
    for (bench, v, src) in todo:
        shutil.copy(src, f"{bins}/{bench}_{v}.rs")
    rc, log = sh("cargo build --release --keep-going", cwd=f"{H}/rust", timeout=3600,
                 env={"RUSTFLAGS": "-C target-cpu=native"})
    open(f"{H}/rust_build.log", "w").write(log)
    res = {}
    for (bench, v, src) in todo:
        exe = f"{H}/rust/target/release/{bench}_{v}"
        res[(bench, v)] = (0 if os.path.exists(exe) else 1, exe)
    return res


def step_build(only_langs=None):
    builds = load("builds")
    rust_todo = []
    for bench in BENCH:
        for lang in LANGS:
            if only_langs and lang not in only_langs:
                continue
            for v, src in variants(bench, lang):
                key = f"{bench}/{lang}/{v}"
                if key in builds and builds[key]["ok"]:
                    continue
                if lang == "rust":
                    rust_todo.append((bench, v, src))
                    continue
                t = time.time()
                rc, log, cmd = build_one(bench, lang, v, src)
                builds[key] = {"ok": rc == 0, "cmd": cmd, "log": log[-1500:] if rc else ""}
                print(f"build {key}: {'ok' if rc == 0 else 'FAIL'} ({time.time()-t:.1f}s)", flush=True)
                save("builds", builds)
    if rust_todo:
        res = build_rust(rust_todo)
        for (bench, v), (rc, exe) in res.items():
            builds[f"{bench}/rust/{v}"] = {"ok": rc == 0, "cmd": [exe], "log": ""}
            print(f"build {bench}/rust/{v}: {'ok' if rc == 0 else 'FAIL'}", flush=True)
        save("builds", builds)


# ---------------------------------------------------------------- run

def run_timed(cmd, args, stdin, timeout):
    """Runs pinned to CPU 0. Returns (seconds, md5 of stdout, rc)."""
    full = ["taskset", "-c", "0"] + cmd + args
    fin = open(f"{INP}/{stdin}", "rb") if stdin else subprocess.DEVNULL
    out_path = f"{H}/last_out"
    with open(out_path, "wb") as fout:
        t = time.perf_counter()
        p = subprocess.Popen(full, stdin=fin, stdout=fout, stderr=subprocess.DEVNULL)
        timer = threading.Timer(timeout, p.kill)
        timer.start()
        rc = p.wait()  # blocking waitpid: no polling granularity
        dt = time.perf_counter() - t
        timed_out = not timer.is_alive()
        timer.cancel()
        if timed_out:
            return None, None, -999
    md5 = hashlib.md5(open(out_path, "rb").read()).hexdigest()
    return dt, md5, rc


def kestrel_cmd(bench, variant=""):
    exe = f"{KS}/build/{bench}{variant}"
    return [exe]


def build_kestrel():
    os.makedirs(f"{KS}/build", exist_ok=True)
    for bench in BENCH:
        for f in sorted(glob.glob(f"{KS}/{bench}*.ks")):
            name = os.path.basename(f)[:-3]
            rc, log = sh([KESTREL, "build", "-O2", f, "-o", f"{KS}/build/{name}"] + KS_LINK.get(bench, []))
            print(f"kestrel build {name}: {'ok' if rc == 0 else 'FAIL ' + log[-500:]}", flush=True)


def expected(bench, small):
    """Reference output md5 comes from the Kestrel program, cross-checked against C #1."""
    full, fstdin, sm, sstdin = BENCH[bench]
    args, stdin = (sm, sstdin) if small else (full, fstdin)
    _, md5, _ = run_timed(kestrel_cmd(bench), args, stdin, 600)
    shutil.copy(f"{H}/last_out", f"{H}/expected_{bench}_{'small' if small else 'full'}.out")
    return md5


def output_ok(bench, md5, small):
    """Exact match, except mandelbrot may differ in a few pixels (float edge cases)."""
    exp = load("expected")[f"{bench}/{'small' if small else 'full'}"]
    if md5 == exp:
        return True
    if bench != "mandelbrot" or md5 is None:
        return False
    a = open(f"{H}/last_out", "rb").read()
    b = open(f"{H}/expected_{bench}_{'small' if small else 'full'}.out", "rb").read()
    return len(a) == len(b) and sum(x != y for x, y in zip(a, b)) <= 16


def step_select(langs=None):
    builds = load("builds")
    sel = load("select")
    exp = load("expected")
    for bench in BENCH:
        if f"{bench}/small" not in exp:
            exp[f"{bench}/small"] = expected(bench, True)
            save("expected", exp)
        _, _, sm, sstdin = BENCH[bench]
        for lang in LANGS:
            if langs and lang not in langs:
                continue
            for v, _ in variants(bench, lang):
                key = f"{bench}/{lang}/{v}"
                if key in sel:
                    continue
                b = builds.get(key)
                if not b or not b["ok"]:
                    sel[key] = {"status": "build-failed"}
                    continue
                tmo = 600 if lang == "python" else 300
                dt, md5, rc = run_timed(b["cmd"], sm, sstdin, tmo)
                if dt is None:
                    st = "timeout"
                elif not output_ok(bench, md5, True):
                    st = f"wrong-output(rc={rc})"
                else:
                    st = "ok"
                sel[key] = {"status": st, "time": dt}
                print(f"select {key}: {st} {dt and round(dt, 3)}", flush=True)
                save("select", sel)


def best_per_lang(sel, skip=()):
    best = {}
    for key, r in sel.items():
        if r["status"] != "ok" or key in skip:
            continue
        bench, lang, v = key.split("/")
        k = (bench, lang)
        if k not in best or r["time"] < best[k][1]:
            best[k] = (v, r["time"])
    return best


def step_full(langs=None, runs=3):
    builds = load("builds")
    sel = load("select")
    exp = load("expected")
    res = load("full")
    # Programs that failed at full size drop out, and the next fastest runs.
    failed = {f"{k.split('/')[0]}/{k.split('/')[1]}/{r['v']}" for k, r in res.items() if r["status"] == "wrong-output"}
    best = best_per_lang(sel, failed)
    for bench in BENCH:
        full, fstdin, _, _ = BENCH[bench]
        if f"{bench}/full" not in exp:
            exp[f"{bench}/full"] = expected(bench, False)
            save("expected", exp)
        entries = [("kestrel", "ks", kestrel_cmd(bench))]
        for lang in LANGS:
            if langs and lang not in langs:
                continue
            if (bench, lang) in best:
                v = best[(bench, lang)][0]
                entries.append((lang, v, builds[f"{bench}/{lang}/{v}"]["cmd"]))
        for lang, v, cmd in entries:
            key = f"{bench}/{lang}"
            if langs and lang not in langs and lang != "kestrel":
                continue
            if key in res and res[key].get("v") == v and res[key]["status"] != "timeout":
                continue
            times = []
            st = "ok"
            for i in range(runs):
                dt, md5, rc = run_timed(cmd, full, fstdin, 1800)
                if dt is None:
                    st = "timeout"
                    break
                if not output_ok(bench, md5, False):
                    st = "wrong-output"
                    break
                times.append(dt)
                if dt > 60:  # one run is enough for slow programs
                    break
            res[key] = {"v": v, "status": st, "times": times, "best": min(times) if times else None}
            print(f"full {key} (#{v}): {st} {[round(t, 3) for t in times]}", flush=True)
            save("full", res)


if __name__ == "__main__":
    step = sys.argv[1]
    langs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
    if step == "build":
        step_build(langs)
    elif step == "kbuild":
        build_kestrel()
    elif step == "select":
        step_select(langs)
    elif step == "full":
        step_full(langs)
