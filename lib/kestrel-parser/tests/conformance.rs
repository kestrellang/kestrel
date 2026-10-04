//! Every error-free parse conforms to `kestrel.ungram`.
//!
//! The typed views in `kestrel_syntax_tree::ast` are generated from the
//! grammar, so a tree that does not match it is one the views misread.

mod common;

use common::parse;
use kestrel_syntax_tree::validate::validate;

#[test]
fn corpus_conforms_to_grammar() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    let mut failures = Vec::new();
    for dir in [
        "../../lang",
        "../kestrel-test-suite/testdata",
        "../../examples",
    ] {
        for path in walk(&manifest.join(dir)) {
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let result = parse(&source);
            if !result.errors.is_empty() {
                continue;
            }
            checked += 1;
            for v in validate(&result.tree()) {
                let at = usize::from(v.range.start());
                let line = source[..at].matches('\n').count() + 1;
                failures.push(format!("{}:{line}: {v}", path.display()));
            }
        }
    }
    assert!(checked > 1000, "corpus not found");
    assert!(
        failures.is_empty(),
        "{} violations (first 40):\n{}",
        failures.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else if path.extension().is_some_and(|e| e == "ks") {
            out.push(path);
        }
    }
    out.sort();
    out
}
