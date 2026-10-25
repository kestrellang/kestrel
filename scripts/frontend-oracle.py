#!/usr/bin/env python3
"""Differential oracle for front-end rewrites.

Runs two `kestrel` CLI binaries (a frozen *baseline* and the *new* build) over
the whole `.ks` corpus and compares what they produce:

  cst   `kestrel dump cst`          — tree diff after the normalisations below
  diag  `kestrel dump diagnostics`  — diagnostic multiset (severity, code,
                                      message, file:line, label text)
  exec  `kestrel build` + run       — `// test: execution` files only:
                                      build status, exit code, stdout

It never runs the test-suite binary. It does re-implement the suite's
two-way `// ERROR:` annotation check (substring, same line, test file only)
so a diff can be read as "this test would flip pass→fail" — a *simulated*
verdict, reported as such.

Usage
-----
  frontend-oracle.py collect --bin PATH --out DIR [--kinds cst,diag,exec]
                     [--jobs N] [--filter SUBSTR]
  frontend-oracle.py compare BASE_DIR NEW_DIR [--kinds ...] [--verbose]

`collect` is resumable: an output that already exists is not recomputed, so a
baseline only has to be collected once. Run from the repository root (the CLI
finds `lang/std` relative to it).

CST normalisations (known baseline defects, audit H1)
-----------------------------------------------------
The baseline parser drops separators/brackets inside chumsky `separated_by`
lists; the tree builder then swallows the gap — punctuation and trivia — into
an `Error` token. It also emits a call's trailing closure inside the
`ArgumentList` *before* `)`, which rewinds the tree builder and re-emits the
closure's source as another `Error` token (the baseline tree does not
round-trip). So before comparing:

  1. trivia tokens are dropped from both trees;
  2. `Error` tokens are dropped from the baseline tree (they are the
     swallowed gaps — `Error` *nodes* are kept);
  3. synthesized `Missing` nodes are dropped from the baseline tree when the
     new tree has no parse errors (the baseline invents a `Missing(RParen)`
     for a paren-less trailing-closure call);
  4. in the new tree, an `ArgumentList` whose `RParen` precedes trailing
     closure `Argument`s is rewritten to the baseline order (`RParen` last);
  5. punctuation the new tree has but the baseline dropped (found by a linear
     alignment of the two token streams) is removed from the new tree.

Anything still different is reported. Files where the baseline reports parse
errors are listed separately (recovery is expected to differ there).
"""

import argparse
import concurrent.futures as cf
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

CORPUS_ROOTS = ["lib/kestrel-test-suite/testdata", "lang", "examples"]
ANSI = re.compile(r"\x1b\[[0-9;]*m")
TRIVIA = {"Whitespace", "Newline", "LineComment", "BlockComment"}
# Fixed-lexeme tokens the baseline sometimes loses into an `Error` gap
# (separators, brackets, `=` of a default, `and`/`not` of bounds, `:` of a
# where-bound). Identifiers and literals are never dropped by the alignment.
NOT_DROPPABLE = {"Identifier", "String", "RawString", "Char", "Integer", "Float",
                 "Boolean", "Error"}


# ----------------------------------------------------------------------------
# corpus
# ----------------------------------------------------------------------------

def corpus(filter_substr=None):
    files = []
    for root in CORPUS_ROOTS:
        for p in sorted(Path(root).rglob("*.ks")):
            s = str(p)
            if filter_substr and filter_substr not in s:
                continue
            files.append(s)
    return files


def header(path):
    """Parse the test header (`// test:`, `// stdlib:`, `// include:`...)."""
    cfg = {"test": "diagnostics", "stdlib": True, "include": [],
           "expect_exit": 0, "expect_stdout": None, "stdout_contains": None,
           "skip": None}
    try:
        text = Path(path).read_text()
    except Exception:
        return cfg
    for line in text.splitlines():
        t = line.strip()
        if t and not t.startswith("//"):
            break
        if not t.startswith("//"):
            continue
        body = t[2:].strip()
        if ":" not in body:
            continue
        k, v = body.split(":", 1)
        k, v = k.strip().lower(), v.strip()
        if k == "test":
            cfg["test"] = v.lower()
        elif k == "stdlib":
            cfg["stdlib"] = v.lower() != "false"
        elif k == "include":
            cfg["include"].append(v)
        elif k == "expect-exit":
            try:
                cfg["expect_exit"] = int(v)
            except ValueError:
                pass
        elif k == "expect-stdout":
            cfg["expect_stdout"] = decode_escapes(v)
        elif k == "stdout-contains":
            cfg["stdout_contains"] = decode_escapes(v)
        elif k == "skip":
            cfg["skip"] = v
    return cfg


def decode_escapes(s):
    out, i = [], 0
    table = {"n": "\n", "t": "\t", "r": "\r", "\\": "\\", "0": "\0"}
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            nxt = s[i + 1]
            out.append(table.get(nxt, "\\" + nxt))
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


def std_flags(path, cfg):
    # The stdlib's own files are compiled standalone (no second copy of std).
    if path.startswith("lang/std/") or not cfg["stdlib"]:
        return ["--no-std"]
    return []


def file_args(path, cfg):
    d = os.path.dirname(path)
    return [path] + [os.path.join(d, inc) for inc in cfg["include"]]


# ----------------------------------------------------------------------------
# collect
# ----------------------------------------------------------------------------

def run(cmd, timeout):
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout)
        return p.returncode, p.stdout.decode("utf8", "replace"), \
            p.stderr.decode("utf8", "replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "", ""


def out_path(out, kind, path):
    return Path(out) / kind / (path + ".txt")


def collect_one(binary, out, kind, path):
    dest = out_path(out, kind, path)
    if dest.exists():
        return
    cfg = header(path)
    dest.parent.mkdir(parents=True, exist_ok=True)
    if kind == "cst":
        code, so, se = run([binary, "dump", "cst", path], 120)
        text = f"exit: {code}\n--- stderr\n{ANSI.sub('', se)}--- stdout\n{so}"
    elif kind == "diag":
        cmd = [binary] + std_flags(path, cfg) + ["dump", "diagnostics"] + file_args(path, cfg)
        code, so, se = run(cmd, 300)
        text = f"exit: {code}\n{ANSI.sub('', se)}"
    elif kind == "exec":
        if cfg["test"] != "execution" or not path.startswith("lib/"):
            return
        with tempfile.TemporaryDirectory() as td:
            exe = os.path.join(td, "a.out")
            cmd = [binary] + std_flags(path, cfg) + ["build", "-o", exe] + file_args(path, cfg)
            bcode, _, bse = run(cmd, 600)
            if bcode != 0:
                text = f"build: {bcode}\n"
            else:
                rcode, rso, _ = run([exe], 60)
                text = f"build: 0\nrun: {rcode}\n--- stdout\n{rso}"
    else:
        raise ValueError(kind)
    tmp = dest.with_suffix(".tmp")
    tmp.write_text(text)
    tmp.replace(dest)


def cmd_collect(args):
    files = corpus(args.filter)
    kinds = args.kinds.split(",")
    binary = os.path.abspath(args.bin)
    jobs = [(k, f) for k in kinds for f in files]
    done = 0
    with cf.ThreadPoolExecutor(args.jobs) as ex:
        futs = [ex.submit(collect_one, binary, args.out, k, f) for k, f in jobs]
        for fut in cf.as_completed(futs):
            fut.result()
            done += 1
            if done % 200 == 0:
                print(f"  {done}/{len(jobs)}", file=sys.stderr, flush=True)
    print(f"collected {len(jobs)} outputs into {args.out}")


# ----------------------------------------------------------------------------
# CST parsing + normalisation
# ----------------------------------------------------------------------------

LINE = re.compile(r'^( *)([A-Za-z_]+)@(\d+)\.\.(\d+)(?: (".*"))?$')


class Node:
    __slots__ = ("kind", "children", "text")

    def __init__(self, kind, text=None):
        self.kind = kind
        self.children = [] if text is None else None
        self.text = text

    @property
    def is_token(self):
        return self.children is None


def parse_cst(dump):
    """Parse the Debug rendering of a rowan tree. Returns (root, stderr, exit)."""
    exit_line, rest = dump.split("\n", 1)
    _, rest = rest.split("--- stderr\n", 1)
    stderr, stdout = rest.split("--- stdout\n", 1)
    root, stack = None, []
    for line in stdout.splitlines():
        m = LINE.match(line)
        if not m:
            continue
        depth = len(m.group(1)) // 2
        node = Node(m.group(2), m.group(5))
        while len(stack) > depth:
            stack.pop()
        if stack:
            stack[-1].children.append(node)
        else:
            root = node
        if not node.is_token:
            stack.append(node)
    return root, stderr, exit_line


def parse_error_count(stderr):
    # Parse errors are the only diagnostics `dump cst` prints, and they are
    # uncoded (`error:` / later `error[P...]:`).
    return len(re.findall(r"^error(?:\[E8\d\d\])?:", stderr, re.M))


def strip(node, drop_error_tokens, drop_missing):
    if node.is_token:
        return node
    kids = []
    for c in node.children:
        if c.is_token:
            if c.kind in TRIVIA:
                continue
            if drop_error_tokens and c.kind == "Error":
                continue
            # Baseline-only: a separator whose span was computed by byte
            # arithmetic covers the wrong text (`Comma "E"`) — it is not a
            # token of the source at all.
            if drop_error_tokens and c.kind == "Comma" and c.text != '","':
                continue
            kids.append(c)
        else:
            if drop_missing and c.kind == "Missing":
                continue
            kids.append(strip(c, drop_error_tokens, drop_missing))
    node.children = kids
    return node


def fix_double_optional(node):
    """Baseline emits `T??` with the one `??` token twice (once per nested
    TyOptional, as `Question`); the new tree has it once, as
    `QuestionQuestion`, on the outer node."""
    if node.is_token:
        return
    for c in node.children:
        fix_double_optional(c)
    if node.kind != "TyOptional" or not node.children:
        return
    last = node.children[-1]
    if not (last.is_token and last.kind == "Question" and last.text == '"??"'):
        return
    last.kind = "QuestionQuestion"
    inner_ty = node.children[0]
    if inner_ty.is_token or not inner_ty.children:
        return
    inner = inner_ty.children[0]
    if (not inner.is_token and inner.kind == "TyOptional" and inner.children
            and inner.children[-1].is_token and inner.children[-1].text == '"??"'):
        inner.children.pop()


def flatten_binary(node):
    """Phase 2: the parser applies precedence, the baseline left-folded every
    operator chain. Compare chains as flat `operand op operand …` lists."""
    if node.is_token:
        return
    for c in node.children:
        flatten_binary(c)
    if node.kind != "Expression" or len(node.children) != 1:
        return
    inner = node.children[0]
    if inner.is_token or inner.kind != "ExprBinary":
        return
    flat = []
    for c in inner.children:
        if (not c.is_token and c.kind == "Expression" and len(c.children) == 1
                and not c.children[0].is_token and c.children[0].kind == "BinaryChain"):
            flat.extend(c.children[0].children)
        else:
            flat.append(c)
    chain = Node("BinaryChain")
    chain.children = flat
    node.children = [chain]


def collapse_interpolation(node, is_base):
    """Phase 2: interpolated strings are structured in the new tree; the
    baseline had one `String` token. Compare both as an opaque literal."""
    if node.is_token:
        return
    for i, c in enumerate(node.children):
        if not c.is_token and c.kind == "ExprInterpolatedString":
            s = Node("ExprString")
            s.children = [Node("String", '"<interpolated>"')]
            node.children[i] = s
        elif (not c.is_token and c.kind == "ExprString" and c.children
              and c.children[0].is_token and "\\\\(" in c.children[0].text):
            c.children[0].text = '"<interpolated>"'
        else:
            collapse_interpolation(c, is_base)


def unwrap_ty_paren(node):
    """Phase 3: grouping parens are `Ty > TyParen > ( Ty )`; the baseline
    left the parens as bare tokens next to the inner `Ty`."""
    if node.is_token:
        return
    kids = []
    for c in node.children:
        unwrap_ty_paren(c)
        if (not c.is_token and c.kind == "Ty" and len(c.children) == 1
                and not c.children[0].is_token and c.children[0].kind == "TyParen"):
            kids.extend(c.children[0].children)
        else:
            kids.append(c)
    node.children = kids


def single_body_expression(node):
    """Phase 3: `func f() = e` is `FunctionBody(= Expression)`; the baseline
    wrapped the expression twice."""
    if node.is_token:
        return
    for c in node.children:
        single_body_expression(c)
    if node.kind != "FunctionBody":
        return
    for i, c in enumerate(node.children):
        if (not c.is_token and c.kind == "Expression" and len(c.children) == 1
                and not c.children[0].is_token and c.children[0].kind == "Expression"):
            node.children[i] = c.children[0]


def drop_fixed_lexemes(node):
    if node.is_token:
        return
    node.children = [c for c in node.children
                     if not (c.is_token and c.kind not in NOT_DROPPABLE)]
    for c in node.children:
        drop_fixed_lexemes(c)


def reorder_trailing_closures(node):
    """New tree: ArgumentList(`(` args `)` closure...) → baseline order."""
    if node.is_token:
        return
    for c in node.children:
        reorder_trailing_closures(c)
    if node.kind != "ArgumentList":
        return
    idx = [i for i, c in enumerate(node.children) if c.is_token and c.kind == "RParen"]
    if not idx:
        return
    i = idx[-1]
    if i == len(node.children) - 1:
        return
    rparen = node.children.pop(i)
    node.children.append(rparen)


def tokens(node, acc):
    if node.is_token:
        acc.append(node)
    else:
        for c in node.children:
            tokens(c, acc)
    return acc


def align_and_drop(base, new):
    """Remove from `new` punctuation the baseline lost. Returns mismatch info."""
    bt, nt = tokens(base, []), tokens(new, [])
    drop, i, j = set(), 0, 0
    while j < len(nt):
        # The dump truncates long token text, so a baseline interpolated
        # string may not show its `\(`: accept any String there.
        if (i < len(bt) and nt[j].text == '"<interpolated>"' and bt[i].kind == "String"):
            bt[i].text = nt[j].text
        if i < len(bt) and same_token(bt[i], nt[j]):
            # `]]` in new vs `]` in base: the baseline kept the *outer*
            # bracket, so drop the inner (first) one.
            ambiguous = (nt[j].kind not in NOT_DROPPABLE and j + 1 < len(nt)
                         and same_token(bt[i], nt[j + 1])
                         and not (i + 1 < len(bt) and same_token(bt[i + 1], nt[j + 1])))
            if not ambiguous:
                i += 1
                j += 1
                continue
        if nt[j].kind not in NOT_DROPPABLE:
            drop.add(id(nt[j]))
            j += 1
            continue
        break
    remove_ids(new, drop)
    if j < len(nt) or i < len(bt):
        b = bt[i] if i < len(bt) else None
        n = nt[j] if j < len(nt) else None
        return f"token stream diverges at base[{i}]={fmt_tok(b)} new[{j}]={fmt_tok(n)}"
    return None


def same_token(a, b):
    # The baseline sometimes spans a token over its leading trivia
    # (`Arrow " ->"`): compare the text without surrounding whitespace.
    return a.kind == b.kind and a.text.strip('"').strip() == b.text.strip('"').strip()


def fmt_tok(t):
    return "EOF" if t is None else f"{t.kind}{t.text}"


def remove_ids(node, ids):
    if node.is_token:
        return
    node.children = [c for c in node.children if id(c) not in ids]
    for c in node.children:
        remove_ids(c, ids)


def render(node, depth=0, out=None):
    out = [] if out is None else out
    pad = "  " * depth
    if node.is_token:
        text = '"' + node.text.strip('"').strip() + '"'
        out.append(f"{pad}{node.kind} {text}")
    else:
        out.append(f"{pad}{node.kind}")
        for c in node.children:
            render(c, depth + 1, out)
    return out


def count_kind(node, kind, tokens_only=None):
    n = 0
    if node.kind == kind and (tokens_only is None or node.is_token == tokens_only):
        n += 1
    if not node.is_token:
        for c in node.children:
            n += count_kind(c, kind, tokens_only)
    return n


def tree_text(node):
    if node.is_token:
        t = node.text[1:-1]
        return t.encode("utf8").decode("unicode_escape", "replace") if "\\" in t else t
    return "".join(tree_text(c) for c in node.children)


def compare_cst(path, btxt, ntxt):
    """Returns (status, detail). status in identical/different/error-file."""
    broot, berr, _ = parse_cst(btxt)
    nroot, nerr, _ = parse_cst(ntxt)
    if broot is None or nroot is None:
        return "different", "missing tree"
    bpe, npe = parse_error_count(berr), parse_error_count(nerr)
    notes = []
    # Losslessness: the new tree's text must be exactly the source.
    m = re.search(r"^SourceFile@0\.\.(\d+)", ntxt, re.M)
    size = len(Path(path).read_bytes())
    if not m or int(m.group(1)) != size:
        notes.append(f"new tree does not round-trip: SourceFile range {m and m.group(1)} vs {size} bytes")
    # Invariant: no parse errors ⇒ no Error tokens or nodes.
    if npe == 0 and count_kind(nroot, "Error") > 0:
        notes.append(f"new tree has {count_kind(nroot, 'Error')} Error elements but no parse errors")
    strip(broot, True, True)
    strip(nroot, False, False)
    fix_double_optional(broot)
    reorder_trailing_closures(nroot)
    unwrap_ty_paren(nroot)
    single_body_expression(broot)
    collapse_interpolation(broot, True)
    collapse_interpolation(nroot, False)
    flatten_binary(broot)
    flatten_binary(nroot)
    mismatch = align_and_drop(broot, nroot)
    b, n = render(broot), render(nroot)
    status = "identical" if b == n and not notes else "different"
    if status == "different" and not notes:
        # Lenient level: same tree once every keyword/punctuation token is
        # ignored — only the *placement* of tokens the baseline lost differs.
        drop_fixed_lexemes(broot)
        drop_fixed_lexemes(nroot)
        if render(broot) == render(nroot):
            status = "punct-placement"
    if b != n:
        import difflib
        diff = list(difflib.unified_diff(b, n, "base", "new", n=3, lineterm=""))
        notes.append((mismatch or "") + "\n" + "\n".join(diff[:60]))
    if bpe or npe:
        status = "parse-errors:" + status
    return status, "\n".join(notes)


# ----------------------------------------------------------------------------
# diagnostics
# ----------------------------------------------------------------------------

HDR = re.compile(r"^(error|warning|note|help|bug)(?:\[([A-Z]\d+)\])?: (.*)$")
LOC = re.compile(r"^\s*┌─ (.*):(\d+):(\d+)$")
CARET = re.compile(r"^\s*(\d+\s*)?│.*?[\^]+\s?(.*)$")


def parse_diags(text):
    """Split rendered codespan output into diagnostic records."""
    lines = text.splitlines()[1:]
    diags, cur = [], None
    for line in lines:
        m = HDR.match(line)
        if m:
            cur = {"sev": m.group(1), "code": m.group(2) or "", "msg": m.group(3),
                   "file": "", "line": 0, "label": "", "notes": []}
            diags.append(cur)
            continue
        if cur is None:
            continue
        m = LOC.match(line)
        if m and not cur["file"]:
            cur["file"], cur["line"] = m.group(1), int(m.group(2))
            continue
        m = CARET.match(line)
        if m and not cur["label"] and "^" in line:
            cur["label"] = m.group(2).strip()
            continue
        s = line.strip()
        if s.startswith("= "):
            cur["notes"].append(s)
    return diags


def diag_key(d):
    return (d["sev"], d["code"], d["msg"], d["file"], d["line"], d["label"],
            tuple(d["notes"]))


def fmt_diag(d):
    code = f"[{d['code']}]" if d["code"] else ""
    lab = f" «{d['label']}»" if d["label"] else ""
    return f"{d['sev']}{code} {d['file']}:{d['line']}: {d['msg']}{lab}"


def is_uncoded_error(d):
    # Baseline parse errors are uncoded, but so are some name-resolution
    # errors (`undefined name`): this is a hint, not a classification.
    return d["sev"] == "error" and (d["code"] == "" or d["code"].startswith("P"))


# ----------------------------------------------------------------------------
# simulated test verdict (mirrors kestrel-test-suite diagnostic_matcher.rs)
# ----------------------------------------------------------------------------

ANN = re.compile(r"//\s*(ERROR|WARN)")


def annotations(path):
    anns = []
    for no, line in enumerate(Path(path).read_text().splitlines(), 1):
        idx = 0
        while True:
            k = line.find("//", idx)
            if k < 0:
                break
            after = line[k + 2:].lstrip()
            idx = k + 2
            if after.startswith("ERROR("):
                code = after[6:].split(")", 1)[0]
                anns.append((no, "code", code))
                break
            if after.startswith("ERROR"):
                rest = after[5:]
                msg = rest[1:].strip() if rest.startswith(":") else None
                anns.append((no, "error", msg or None))
                break
            if after.startswith("WARN"):
                rest = after[4:]
                rest = rest[3:] if rest.startswith("ING") else rest
                msg = rest[1:].strip() if rest.startswith(":") else None
                anns.append((no, "warn", msg or None))
                break
    return anns


def verdict(path, diags):
    anns = annotations(path)
    mine = [d for d in diags if os.path.normpath(d["file"]) == os.path.normpath(path)]

    def full(d):
        m = d["msg"]
        if d["label"] and d["label"] != d["msg"]:
            m = f"{m}: {d['label']}"
        return m.lower()

    def match(a, d):
        if a[0] != d["line"]:
            return False
        if a[1] == "code":
            return d["sev"] == "error" and d["code"] == a[2]
        sev = "error" if a[1] == "error" else "warning"
        if d["sev"] != sev:
            return False
        return a[2] is None or a[2].lower() in full(d)

    used = [False] * len(mine)
    for a in anns:
        hit = False
        for i, d in enumerate(mine):
            if match(a, d):
                hit = used[i] = True
        if not hit:
            return False
    return all(used[i] or mine[i]["sev"] not in ("error", "warning") for i in range(len(mine)))


# ----------------------------------------------------------------------------
# compare
# ----------------------------------------------------------------------------

def read(p):
    try:
        return Path(p).read_text()
    except FileNotFoundError:
        return None


def output_text(binary, kind, path):
    """Recompute one output (same format as `collect`) without caching."""
    with tempfile.TemporaryDirectory() as td:
        dest = Path(td) / "out"
        collect_one(os.path.abspath(binary), str(dest), kind, path)
        return read(out_path(str(dest), kind, path))


def diag_signature(text):
    return frozenset(Counter(diag_key(d) for d in parse_diags(text)).items())


def diag_signature_no_line(text):
    return frozenset(Counter((d["sev"], d["code"], d["msg"], d["file"], d["label"])
                             for d in parse_diags(text)).items())


def recheck(args, kind, path):
    """Re-run both binaries `args.recheck` times. The compiler has some
    pre-existing nondeterminism (diagnostic sets that vary run to run), so a
    file is `flaky` — not a difference — when some new run reproduces some
    baseline run exactly."""
    base_texts = [output_text(args.base_bin, kind, path) for _ in range(args.recheck)]
    new_texts = [output_text(args.new_bin, kind, path) for _ in range(args.recheck)]
    sig = diag_signature if kind == "diag" else (lambda t: t)
    base_runs, new_runs = {sig(t) for t in base_texts}, {sig(t) for t in new_texts}
    if base_runs & new_runs:
        return "flaky" if len(base_runs) > 1 or len(new_runs) > 1 else "identical-on-rerun"
    # Some diagnostics land on whichever of several equivalent sites the
    # checker reaches first (`could not infer type` on one of many `self`s),
    # so the *line* varies run to run. When the baseline itself varies, also
    # accept a match that ignores lines.
    if kind == "diag" and len(base_runs) > 1:
        base_l = {diag_signature_no_line(t) for t in base_texts}
        if base_l & {diag_signature_no_line(t) for t in new_texts}:
            return "flaky-lines"
    return None


def cmd_compare(args):
    kinds = args.kinds.split(",")
    files = corpus(args.filter)
    report = []
    for kind in kinds:
        stats, details = Counter(), []
        flips = []
        for f in files:
            b = read(out_path(args.base, kind, f))
            n = read(out_path(args.new, kind, f))
            if b is None or n is None:
                if b is not None or n is not None:
                    stats["missing"] += 1
                continue
            if kind == "cst":
                st, det = compare_cst(f, b, n)
                stats[st] += 1
                if not st.endswith("identical"):
                    details.append((f, st, det))
            elif kind == "diag":
                bd, nd = parse_diags(b), parse_diags(n)
                bc = Counter(diag_key(d) for d in bd)
                nc = Counter(diag_key(d) for d in nd)
                cfg = header(f)
                if f.startswith("lib/") and cfg["test"] == "diagnostics":
                    vb, vn = verdict(f, bd), verdict(f, nd)
                    if vb != vn:
                        flips.append((f, vb, vn))
                if bc == nc:
                    stats["identical"] += 1
                    continue
                if args.recheck:
                    rc = recheck(args, kind, f)
                    if rc:
                        stats[rc] += 1
                        details.append((f, rc, ""))
                        continue
                only_parse = all(is_uncoded_error(dict(zip(
                    ("sev", "code", "msg", "file", "line", "label", "notes"), k)))
                    for k in (bc - nc) + (nc - bc))
                st = "different(uncoded-only)" if only_parse else "different"
                stats[st] += 1
                lines = [f"- {fmt_diag(dict(zip(('sev','code','msg','file','line','label','notes'), k)))}"
                         for k in (bc - nc).elements()]
                lines += [f"+ {fmt_diag(dict(zip(('sev','code','msg','file','line','label','notes'), k)))}"
                          for k in (nc - bc).elements()]
                details.append((f, st, "\n".join(lines[:40])))
            elif kind == "exec":
                if b == n:
                    stats["identical"] += 1
                elif args.recheck and (rc := recheck(args, kind, f)):
                    stats[rc] += 1
                    details.append((f, rc, ""))
                else:
                    stats["different"] += 1
                    details.append((f, "different", f"base:\n{b[:400]}\nnew:\n{n[:400]}"))
        report.append(f"== {kind}: " + ", ".join(f"{k}={v}" for k, v in sorted(stats.items())))
        if flips:
            report.append(f"   simulated diagnostics-test verdict flips: {len(flips)}")
            for f, vb, vn in flips:
                report.append(f"     {'PASS' if vb else 'FAIL'} -> {'PASS' if vn else 'FAIL'}  {f}")
        for f, st, det in details:
            report.append(f"-- [{kind}] {st}: {f}")
            if args.verbose and det:
                report.append("\n".join("     " + l for l in det.splitlines()))
    print("\n".join(report))


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("collect")
    c.add_argument("--bin", required=True)
    c.add_argument("--out", required=True)
    c.add_argument("--kinds", default="cst,diag")
    c.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    c.add_argument("--filter")
    k = sub.add_parser("compare")
    k.add_argument("base")
    k.add_argument("new")
    k.add_argument("--kinds", default="cst,diag")
    k.add_argument("--filter")
    k.add_argument("--verbose", "-v", action="store_true")
    k.add_argument("--recheck", type=int, default=0,
                   help="re-run differing diag/exec files N times with both binaries")
    k.add_argument("--base-bin", help="baseline binary (for --recheck)")
    k.add_argument("--new-bin", help="new binary (for --recheck)")
    a = ap.parse_args()
    {"collect": cmd_collect, "compare": cmd_compare}[a.cmd](a)


if __name__ == "__main__":
    main()
