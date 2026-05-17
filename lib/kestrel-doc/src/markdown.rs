//! Markdown printer for `ModulePage` — renders one `<module>.md` per
//! page, suitable for ingestion by Context7, static-site builders, or
//! any LLM-facing doc consumer that takes raw markdown.
//!
//! Mirrors the JSON shape exactly: each `Item` becomes a heading whose
//! level reflects nesting depth; signatures are fenced as `kestrel`;
//! the item's authored `///` doc body is emitted verbatim except that
//! its own `#` headings (e.g. `# Examples`, `# Safety`) are demoted so
//! they nest under the item heading instead of competing with the
//! module-level structure.

use std::fmt::Write;

use crate::{Item, MemberGroup, ModulePage};

/// Heading level used for top-level items in a module page.
/// Submodules list and the page title sit above this.
const ITEM_LEVEL: usize = 2;

/// Render a single module page to markdown. The output starts with
/// `# <module path>` so each file is a self-contained document.
pub fn render(page: &ModulePage) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {}", page.path);
    out.push('\n');

    if !page.submodules.is_empty() {
        let _ = writeln!(out, "## Submodules");
        out.push('\n');
        for sub in &page.submodules {
            let _ = writeln!(out, "- [`{sub}`]({sub}.md)");
        }
        out.push('\n');
    }

    for item in &page.items {
        write_item(&mut out, item, ITEM_LEVEL);
    }

    out
}

fn write_item(out: &mut String, item: &Item, level: usize) {
    let hashes = heading(level);
    let _ = writeln!(out, "{hashes} {} `{}`", item.kind, item.name);
    out.push('\n');

    if !item.signature.is_empty() {
        let _ = writeln!(out, "```kestrel");
        let _ = writeln!(out, "{}", item.signature);
        let _ = writeln!(out, "```");
        out.push('\n');
    }

    if !item.doc.is_empty() {
        // The authored doc body assumes it sits at document root —
        // `# Examples` means "Examples section of this item". To keep
        // that semantic, demote so item-doc `#` becomes one level below
        // the item heading itself.
        let body = demote_headings(&item.doc, level);
        out.push_str(body.trim_end());
        out.push_str("\n\n");
    }

    if let Some(src) = &item.source_path {
        let _ = writeln!(out, "_Defined in `{src}`._");
        out.push('\n');
    }

    for group in &item.member_groups {
        write_group(out, group, level + 1);
    }
}

fn write_group(out: &mut String, group: &MemberGroup, level: usize) {
    let hashes = heading(level);
    let label = match (group.kind.as_str(), group.label.as_deref()) {
        ("direct", _) => "Members".to_string(),
        ("protocol", Some(name)) => format!("Implements `{name}`"),
        ("protocol", None) => "Implements".to_string(),
        (other, Some(name)) => format!("{other}: {name}"),
        (other, None) => other.to_string(),
    };
    let _ = writeln!(out, "{hashes} {label}");
    out.push('\n');

    for member in &group.members {
        write_item(out, member, level + 1);
    }
}

fn heading(level: usize) -> String {
    "#".repeat(level.clamp(1, 6))
}

/// Add `extra` `#` characters to any line that begins with `#`, capped
/// at h6. Skips lines inside fenced code blocks so example code that
/// happens to use `#` for comments or shell prompts is left alone.
fn demote_headings(body: &str, extra: usize) -> String {
    if extra == 0 {
        return body.to_string();
    }
    let mut out = String::with_capacity(body.len());
    let mut in_fence = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }
        if !in_fence && trimmed.starts_with('#') {
            let existing = trimmed.chars().take_while(|&c| c == '#').count();
            // Only treat as a heading if it looks like one (`# foo`,
            // not `#foo` — preserves things like `#[attr]` if they
            // ever appear in a doc body).
            let after = trimmed[existing..].chars().next();
            if !matches!(after, Some(' ') | Some('\n') | None) {
                out.push_str(line);
                continue;
            }
            let total = (existing + extra).min(6);
            let indent = &line[..line.len() - trimmed.len()];
            let rest = &trimmed[existing..];
            out.push_str(indent);
            for _ in 0..total {
                out.push('#');
            }
            out.push_str(rest);
        } else {
            out.push_str(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demotes_headings_outside_fences() {
        let input = "# Examples\n\n```\n# not a heading\n```\n\n# Safety\n";
        let got = demote_headings(input, 2);
        assert_eq!(
            got,
            "### Examples\n\n```\n# not a heading\n```\n\n### Safety\n"
        );
    }

    #[test]
    fn caps_at_h6() {
        let got = demote_headings("##### deep\n", 5);
        assert_eq!(got, "###### deep\n");
    }

    #[test]
    fn renders_minimal_page() {
        let page = ModulePage {
            path: "std.example".into(),
            name: "example".into(),
            submodules: vec![],
            items: vec![Item {
                kind: "func".into(),
                name: "bump".into(),
                anchor: "function-bump".into(),
                signature: "func bump(Int) -> Int".into(),
                doc: "Increments by one.\n\n# Examples\n\n```\nbump(1)\n```".into(),
                source_path: Some("lang/std/example.ks".into()),
                member_groups: vec![],
            }],
        };
        let md = render(&page);
        assert!(md.contains("# std.example"));
        assert!(md.contains("## func `bump`"));
        assert!(md.contains("```kestrel\nfunc bump(Int) -> Int\n```"));
        assert!(md.contains("### Examples"));
        assert!(md.contains("_Defined in `lang/std/example.ks`._"));
    }
}
