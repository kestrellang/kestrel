#!/usr/bin/env python3
"""Extract a structural map of the Kestrel compiler into JSON.

Everything here is derived from the source tree (plus `cargo metadata`), so
the map can be regenerated for any commit:

    python3 scripts/archmap/extract.py [repo_root] > archmap.json

What it collects:
  * crates      - workspace members, LOC, docs, internal deps (cargo metadata)
  * modules     - every .rs file with LOC and its first `//!` line
  * queries     - every `impl QueryFn for X`, its Output, docs, and the queries
                  it calls (directly in `execute`, or via same-crate helpers)
  * components  - types read/written through `get::<T>` / `has::<T>` / `.set(e, T`
  * analyzers   - Body/Decl/CompilationCheck impls with their diagnostic codes
  * mir_stages  - the `kestrel_mir::passes::Stage` enum, in pipeline order
  * traces      - `ktrace!` categories and where they are used

The parser is regex + brace matching over comment/string-stripped text. It is
a heuristic: the "indirect" query edges follow helper functions by name
within a crate, so treat them as "probably calls", not proof.
"""

import json
import os
import re
import subprocess
import sys
from collections import defaultdict

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else ".")

# Hand-curated: which pipeline stage each crate belongs to. This is the only
# non-derived input; a crate missing here shows up under "unassigned" so the
# drift is visible in the site rather than silently dropped.
STAGES = [
    ("frontend", "Lex & parse", ["kestrel-span", "kestrel-lexer", "kestrel-parser", "kestrel-syntax-tree"]),
    ("decls", "Declarations (hECS)", ["kestrel-hecs", "kestrel-ast", "kestrel-ast-builder"]),
    ("names", "Name resolution", ["kestrel-name-res"]),
    ("hir", "HIR lowering", ["kestrel-hir", "kestrel-hir-lower"]),
    ("types", "Type inference & semantics", ["kestrel-type-infer", "kestrel-semantics", "kestrel-entail", "kestrel-copy-fold"]),
    ("analyze", "Analysis", ["kestrel-analyze", "kestrel-pattern-matching"]),
    ("mir", "MIR & mono", ["kestrel-mir-lower", "kestrel-mir"]),
    ("codegen", "Codegen", ["kestrel-codegen", "kestrel-codegen-cranelift", "kestrel-codegen-llvm"]),
    ("driver", "Driver & tools", ["kestrel", "kestrel-compiler", "kestrel-compiler-driver", "kestrel-lsp", "kestrel-doc", "kestrel-reporting", "kestrel-debug", "kestrel-test-suite"]),
]


def sh(*args):
    return subprocess.run(args, cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def rel(path):
    return os.path.relpath(path, ROOT)


# ---------------------------------------------------------------- source text

_STRIP_RE = re.compile(
    r'//[^\n]*'                      # line comments (incl. doc comments)
    r'|/\*.*?\*/'                    # block comments
    r'|r(#*)".*?"\1'                 # raw strings
    r'|b?"(?:\\.|[^"\\])*"'          # strings
    r"|b?'(?:\\.|[^'\\])'",          # char literals ('{' etc.; lifetimes don't match)
    re.S,
)


def strip(src):
    """Blank out comments and literals, keeping offsets and newlines intact."""
    return _STRIP_RE.sub(lambda m: re.sub(r'[^\n]', ' ', m.group(0)), src)


def block_end(code, open_idx):
    """Index just past the `}` matching the `{` at open_idx."""
    depth = 0
    for i in range(open_idx, len(code)):
        c = code[i]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                return i + 1
    return len(code)


def line_of(src, idx):
    return src.count('\n', 0, idx) + 1


def doc_above(lines, line_no):
    """Collect `///` lines directly above 1-based line_no (skipping attributes)."""
    out = []
    i = line_no - 2
    while i >= 0:
        s = lines[i].strip()
        if s.startswith('#['):
            i -= 1
            continue
        if s.startswith('///'):
            out.append(s[3:].strip())
            i -= 1
            continue
        break
    return ' '.join(reversed(out)).strip()


def module_doc(src, max_lines=None):
    """Leading `//!` block as text (first paragraph if max_lines is None)."""
    out = []
    for line in src.splitlines():
        s = line.strip()
        if s.startswith('//!'):
            out.append(s[3:].strip())
        elif s == '' and not out:
            continue
        else:
            break
    if max_lines is None:
        para = []
        for l in out:
            if not l and para:
                break
            if l and not l.startswith('#'):
                para.append(l)
            elif l.startswith('#') and not para:
                continue
        return ' '.join(para)
    return '\n'.join(out[:max_lines])


# ---------------------------------------------------------------- crates

def load_crates():
    meta = json.loads(sh('cargo', 'metadata', '--no-deps', '--format-version', '1', '--offline'))
    names = {p['name'] for p in meta['packages']}
    crates = {}
    for p in meta['packages']:
        d = os.path.dirname(p['manifest_path'])
        deps = sorted({x['name'] for x in p['dependencies'] if x['name'] in names and x.get('kind') is None})
        dev = sorted({x['name'] for x in p['dependencies'] if x['name'] in names and x.get('kind') == 'dev'} - set(deps))
        ext = sorted({x['name'] for x in p['dependencies'] if x['name'] not in names and x.get('kind') is None})
        crates[p['name']] = {
            'name': p['name'], 'dir': rel(d), 'deps': deps, 'dev_deps': dev, 'external_deps': ext,
        }
    return crates


def rs_files(crate_dir):
    base = os.path.join(ROOT, crate_dir)
    roots = ['src', 'tests', 'benches'] if crate_dir != '.' else ['src']
    for r in roots:
        for dp, dns, fns in os.walk(os.path.join(base, r)):
            dns[:] = sorted(x for x in dns if x not in ('target', 'fixtures'))
            for f in sorted(fns):
                if f.endswith('.rs'):
                    yield os.path.join(dp, f)


# ---------------------------------------------------------------- per-file scan

FN_RE = re.compile(r'\bfn\s+([a-z_][a-z0-9_]*)\s*(?:<[^{;]*?>)?\s*\(')
IMPL_Q_RE = re.compile(r'\bimpl\s*(?:<[^{]*?>)?\s*(?:[\w:]+::)?QueryFn\s+for\s+([A-Z]\w*)')
QCALL_RE = re.compile(r'\bquery\s*\(\s*&?\s*(?:[a-z_]\w*::)*([A-Z]\w*)')
CALL_RE = re.compile(r'\b([a-z_][a-z0-9_]*)\s*(?:::<[^>]*>)?\s*\(')
COMP_READ_RE = re.compile(r'\.(get|has|get_ref|try_get|get_or_default|get_cloned)[a-z_]*::<\s*(?:[a-z_]\w*::)*([A-Z]\w*)')
COMP_SET_RE = re.compile(r'\.set\s*\(\s*[\w.()&*]+\s*,\s*(?:[a-z_]\w*::)*([A-Z]\w*)')
TYPE_DEF_RE = re.compile(r'^\s*pub(?:\([^)]*\))?\s+(struct|enum|type)\s+([A-Z]\w*)', re.M)
TRACE_RE = re.compile(r'ktrace!\s*\(\s*"([\w-]+)"')
CHECK_RE = re.compile(r'\bimpl\s+(BodyCheck|DeclCheck|CompilationCheck)\s+for\s+([A-Z]\w*)')
DESC_RE = re.compile(r'id:\s*"([A-Z]\d+)",\s*name:\s*"([\w]+)",\s*default_severity:\s*Severity::(\w+),\s*category:\s*Category::(\w+)')
ANALYZER_ID_RE = re.compile(r'AnalyzerId::(\w+)')
# Display names for each crate's non-query code in the graph.
ACTOR_LABELS = {
    'kestrel-mir-lower': 'MIR lowering', 'kestrel-lsp': 'LSP handlers', 'kestrel-type-infer': 'Inference internals',
    'kestrel-analyze': 'Analyzer framework', 'kestrel-compiler': 'Compiler pipeline', 'kestrel-compiler-driver': 'Driver',
    'kestrel-pattern-matching': 'Exhaustiveness checking', 'kestrel-doc': 'Doc generator',
}


def short_name(crate):
    return crate.replace('kestrel-', '')


AMBIGUOUS_FNS = {'new', 'execute', 'describe', 'default', 'from', 'into', 'clone', 'fmt', 'eq', 'hash', 'drop'}
KEYWORDS = {'if', 'while', 'for', 'match', 'loop', 'return', 'fn', 'let', 'move', 'in', 'as', 'some', 'ok', 'err', 'none', 'self', 'super', 'crate', 'where', 'impl', 'mut', 'ref'}


def scan_file(path, crate):
    src = open(path, encoding='utf-8', errors='replace').read()
    code = strip(src)
    lines = src.splitlines()
    rp = rel(path)
    is_test = '/tests/' in rp or rp.endswith('/tests.rs') or '_tests.rs' in rp
    # Treat everything after a top-level `#[cfg(test)]` module as test code.
    m = re.search(r'#\[cfg\(test\)\]\s*mod\s+\w+\s*\{', src)
    test_from = m.start() if m else len(code)

    fns = []
    for m in FN_RE.finditer(code):
        brace = code.find('{', m.end())
        semi = code.find(';', m.end())
        if brace < 0 or (0 <= semi < brace):
            continue  # trait method declaration without body
        end = block_end(code, brace)
        body = code[brace:end]
        fns.append({
            'name': m.group(1), 'file': rp, 'line': line_of(src, m.start()), 'start': brace, 'end': end,
            'calls': {c for c in CALL_RE.findall(body) if c not in KEYWORDS and c != m.group(1)},
            'qcalls': [(q, line_of(src, brace + qm.start())) for qm in QCALL_RE.finditer(body) for q in [qm.group(1)]],
            'test': is_test or m.start() > test_from,
        })

    impls = []
    for m in IMPL_Q_RE.finditer(code):
        brace = code.find('{', m.end())
        end = block_end(code, brace)
        out = re.search(r'type\s+Output\s*=\s*([^;]+);', code[brace:end])
        impls.append({'name': m.group(1), 'start': brace, 'end': end, 'line': line_of(src, m.start()),
                      'test': is_test or m.start() > test_from,
                      'output': re.sub(r'\s+', ' ', out.group(1)).strip() if out else None})

    return {
        'path': rp, 'crate': crate, 'src': src, 'code': code, 'lines': lines, 'loc': len(lines),
        'doc': module_doc(src), 'test': is_test, 'fns': fns, 'impls': impls,
        'test_from': 0 if is_test else test_from,
    }


def enclosing(fns, idx):
    best = None
    for f in fns:
        if f['start'] <= idx < f['end'] and (best is None or f['start'] > best['start']):
            best = f
    return best


# ---------------------------------------------------------------- main

def main():
    crates = load_crates()
    files = []
    for name, c in crates.items():
        for p in rs_files(c['dir']):
            files.append(scan_file(p, name))
    by_crate = defaultdict(list)
    for f in files:
        by_crate[f['crate']].append(f)

    # Type definitions: name -> [(crate, file, line, kind, doc)]
    typedefs = defaultdict(list)
    for f in files:
        for m in TYPE_DEF_RE.finditer(f['code'][:f['test_from']]):
            ln = line_of(f['src'], m.start())
            typedefs[m.group(2)].append({'crate': f['crate'], 'file': f['path'], 'line': ln,
                                         'kind': m.group(1), 'doc': doc_above(f['lines'], ln)})

    # ---- crates
    stage_of = {c: sid for sid, _, cs in STAGES for c in cs}
    arch_md = open(os.path.join(ROOT, 'docs/contributing/architecture.md'), encoding='utf-8').read()
    crate_out = []
    for name, c in sorted(crates.items()):
        fs = by_crate[name]
        d = os.path.join(ROOT, c['dir'])
        docs_dir = os.path.join(d, 'docs')
        docs = sorted(rel(os.path.join(docs_dir, x)) for x in os.listdir(docs_dir) if x.endswith('.md')) if os.path.isdir(docs_dir) else []
        agents = [rel(os.path.join(dp, 'AGENTS.md')) for dp, _, fns_ in os.walk(d) if 'AGENTS.md' in fns_ and '/target' not in dp] if c['dir'] != '.' else []
        lib = next((f for f in fs if f['path'].endswith(('src/lib.rs', 'src/main.rs'))), None)
        crate_out.append({
            **c,
            'stage': stage_of.get(name, 'unassigned'),
            'doc': lib['doc'] if lib else '',
            'loc': sum(f['loc'] for f in fs if not f['test']),
            'test_loc': sum(f['loc'] for f in fs if f['test']),
            'files': sum(1 for f in fs if not f['test']),
            'docs': docs, 'agents': sorted(agents),
            # Drift signal: is the crate named in the top-level architecture doc?
            'in_architecture_md': re.search(r'`%s`' % re.escape(name), arch_md) is not None,
        })

    # ---- modules
    modules = [{'path': f['path'], 'crate': f['crate'], 'loc': f['loc'], 'doc': f['doc'], 'test': f['test']} for f in files]

    # ---- queries
    fn_index = defaultdict(list)  # (crate, fn name) -> fns
    fn_crate = {f['path']: f['crate'] for f in files}
    for f in files:
        for fn in f['fns']:
            if not fn['test']:
                fn_index[(f['crate'], fn['name'])].append(fn)

    queries = {}
    for f in files:
        f['impls'] = [im for im in f['impls'] if not im['test']]
        for im in f['impls']:
            defs = [t for t in typedefs.get(im['name'], []) if t['crate'] == f['crate']]
            d = defs[0] if defs else None
            queries[im['name']] = {
                'name': im['name'], 'crate': f['crate'], 'file': f['path'], 'line': im['line'],
                'def_line': d['line'] if d else None, 'def_file': d['file'] if d else None,
                'output': im['output'], 'doc': d['doc'] if d else '',
                'calls': {}, 'callers': [], 'sites': [],
            }

    def helper_defs(crate, name):
        """Same-crate fns a call to `name` could reach. Names defined many
        times (new, execute, lower, ...) are ambiguous, so they're not followed."""
        defs = fn_index.get((crate, name), [])
        return defs if 0 < len(defs) <= 2 and name not in AMBIGUOUS_FNS else []

    def reachable_qcalls(crate, seeds, depth=64):
        """Query calls reachable from a set of fns through same-crate helpers."""
        seen, out, frontier = set(), [], list(seeds)
        for _ in range(depth):
            nxt = []
            for fn in frontier:
                key = (fn['file'], fn['start'])
                if key in seen:
                    continue
                seen.add(key)
                out.extend((q, fn['file'], ln, fn['name']) for q, ln in fn['qcalls'])
                for c in fn['calls']:
                    nxt.extend(helper_defs(crate, c))
            frontier = nxt
        return out

    for f in files:
        for im in f['impls']:
            q = queries[im['name']]
            inside = [fn for fn in f['fns'] if im['start'] <= fn['start'] < im['end']]
            # Direct: calls lexically inside the impl block.
            for fn in inside:
                for callee, ln in fn['qcalls']:
                    if callee in queries:
                        q['calls'].setdefault(callee, {'kind': 'direct', 'file': f['path'], 'line': ln})
            # Indirect: through helper fns the impl calls.
            helpers = [h for fn in inside for c in fn['calls'] for h in helper_defs(f['crate'], c)
                       if not (h['file'] == f['path'] and im['start'] <= h['start'] < im['end'])]
            for callee, file, ln, via in reachable_qcalls(f['crate'], helpers):
                if callee in queries and callee != im['name']:
                    q['calls'].setdefault(callee, {'kind': 'indirect', 'file': file, 'line': ln, 'via': via})

    # Every call site, attributed to its enclosing fn / query.
    for f in files:
        for fn in f['fns']:
            owner = next((im['name'] for im in f['impls'] if im['start'] <= fn['start'] < im['end']), None)
            for callee, ln in fn['qcalls']:
                if callee in queries:
                    queries[callee]['sites'].append({'crate': f['crate'], 'file': f['path'], 'line': ln,
                                                     'fn': fn['name'], 'query': owner, 'test': fn['test']})
    for q in queries.values():
        for callee in q['calls']:
            queries[callee]['callers'].append(q['name'])
        q['calls'] = [{'query': k, **v} for k, v in sorted(q['calls'].items())]
        q['callers'] = sorted(set(q['callers']))
        q['sites'].sort(key=lambda s: (s['file'], s['line']))
        q['used_by_crates'] = sorted({s['crate'] for s in q['sites'] if not s['test']})

    # ---- components
    comp = defaultdict(lambda: {'reads': defaultdict(int), 'writes': defaultdict(int)})
    for f in files:
        if f['test']:
            continue
        live = f['code'][:f['test_from']]
        for m in COMP_READ_RE.finditer(live):
            comp[m.group(2)]['reads'][f['crate']] += 1
        for m in COMP_SET_RE.finditer(live):
            comp[m.group(1)]['writes'][f['crate']] += 1
    components = []
    for name, v in sorted(comp.items()):
        defs = typedefs.get(name, [])
        if not defs:
            continue  # not a workspace type (e.g. a generic param or std type)
        d = defs[0]
        components.append({
            'name': name, 'crate': d['crate'], 'file': d['file'], 'line': d['line'], 'kind': d['kind'], 'doc': d['doc'],
            'reads': dict(sorted(v['reads'].items())), 'writes': dict(sorted(v['writes'].items())),
            'total': sum(v['reads'].values()) + sum(v['writes'].values()),
        })

    # ---- analyzers
    analyzers = []
    for f in files:
        for m in CHECK_RE.finditer(f['code'][:f['test_from']]):
            desc = [{'id': a, 'name': b, 'severity': c, 'category': d} for a, b, c, d in DESC_RE.findall(f['src'])]
            ids = ANALYZER_ID_RE.findall(f['code'])
            analyzers.append({
                'name': m.group(2), 'kind': m.group(1), 'crate': f['crate'], 'file': f['path'],
                'line': line_of(f['src'], m.start()), 'id': ids[0] if ids else None,
                'doc': module_doc(f['src']), 'diagnostics': desc,
                # Analyzers run behind trait objects, so record the queries their file calls.
                'queries': sorted({q for fn in f['fns'] if not fn['test'] for q, _ in fn['qcalls'] if q in queries}),
            })
    analyzers.sort(key=lambda a: (a['kind'], a['name']))

    # ---- actors: code that calls queries without being one
    # A plain fn counts as "inside" a query when some query's execute can
    # reach it through same-crate helpers; those calls are already query
    # edges. Everything else is grouped into an actor: one per analyzer, and
    # one per crate for the rest (MIR lowering, LSP handlers, solver, ...).
    reachable = set()
    for f in files:
        for im in f['impls']:
            frontier = [fn for fn in f['fns'] if im['start'] <= fn['start'] < im['end']]
            frontier += [h for fn in frontier for c in fn['calls'] for h in helper_defs(f['crate'], c)]
            while frontier:
                fn = frontier.pop()
                key = (fn['file'], fn['start'])
                if key in reachable:
                    continue
                reachable.add(key)
                frontier.extend(h for c in fn['calls'] for h in helper_defs(fn_crate[fn['file']], c))
    analyzer_of_file = {a['file']: a for a in analyzers}
    actors = {}
    for f in files:
        for fn in f['fns']:
            if fn['test'] or not fn['qcalls'] or (fn['file'], fn['start']) in reachable:
                continue
            if any(im['start'] <= fn['start'] < im['end'] for im in f['impls']):
                continue
            an = analyzer_of_file.get(f['path'])
            aid = '@' + (an['name'] if an else f['crate'])
            act = actors.setdefault(aid, {
                'id': aid, 'crate': f['crate'], 'kind': 'analyzer' if an else 'plain',
                'label': an['name'].replace('Analyzer', '') if an else ACTOR_LABELS.get(f['crate'], short_name(f['crate']) + ' (other code)'),
                'analyzer_kind': an['kind'] if an else None, 'calls': {}, 'fns': set(),
            })
            act['fns'].add(fn['name'])
            for q, ln in fn['qcalls']:
                if q in queries:
                    act['calls'].setdefault(q, []).append({'file': f['path'], 'line': ln, 'fn': fn['name']})
    actors = [
        {**a, 'fns': sorted(a['fns']), 'calls': [{'query': q, 'sites': v} for q, v in sorted(a['calls'].items())]}
        for a in sorted(actors.values(), key=lambda a: (a['crate'], a['kind'], a['label']))
        if a['calls']
    ]

    # ---- MIR stages (variant order == pipeline order)
    mir_stages = []
    pf = next((f for f in files if f['path'].endswith('kestrel-mir/src/passes/mod.rs')), None)
    if pf:
        m = re.search(r'pub enum Stage\s*\{', pf['code'])
        if m:
            body_start = m.end()
            body = pf['src'][body_start:block_end(pf['code'], m.end() - 1)]
            doc = []
            for line in body.splitlines():
                s = line.strip()
                if s.startswith('///'):
                    doc.append(s[3:].strip())
                elif re.match(r'^[A-Z]\w*,?$', s):
                    mir_stages.append({'name': s.rstrip(','), 'doc': ' '.join(doc)})
                    doc = []

    # ---- ktrace categories
    traces = defaultdict(lambda: defaultdict(int))
    for f in files:
        for cat in TRACE_RE.findall(f['src']):
            traces[cat][f['crate']] += 1
    traces = [{'category': k, 'crates': dict(sorted(v.items())), 'total': sum(v.values())} for k, v in sorted(traces.items())]

    # ---- top-level docs
    top_docs = sorted(rel(os.path.join(ROOT, 'docs/contributing', x)) for x in os.listdir(os.path.join(ROOT, 'docs/contributing')) if x.endswith('.md'))

    out = {
        'meta': {
            'commit': sh('git', 'rev-parse', 'HEAD'),
            'commit_short': sh('git', 'rev-parse', '--short=8', 'HEAD'),
            'subject': sh('git', 'log', '-1', '--format=%s'),
            'date': sh('git', 'log', '-1', '--format=%cI'),
            'describe': sh('git', 'describe', '--tags', '--always'),
            'tags_at_head': sh('git', 'tag', '--points-at', 'HEAD').split(),
        },
        'stages': [{'id': sid, 'title': t, 'crates': cs} for sid, t, cs in STAGES],
        'crates': crate_out,
        'modules': modules,
        'queries': sorted(queries.values(), key=lambda q: (q['crate'], q['name'])),
        'components': components,
        'analyzers': analyzers,
        'actors': actors,
        'mir_stages': mir_stages,
        'traces': traces,
        'top_docs': top_docs,
    }
    json.dump(out, sys.stdout, indent=1)


if __name__ == '__main__':
    main()
