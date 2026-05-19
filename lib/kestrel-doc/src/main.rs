//! kestrel-doc: emit a rustdoc-style snapshot of a Kestrel source tree.
//!
//! Loads .ks files from a directory through `Compiler::load_dir`, walks
//! the resulting `World`, and emits documentation in one or both of:
//!   - JSON: one `<module>.json` per module plus `index.json` (website).
//!   - Markdown: one `<module>.md` per module (Context7 / LLM tools).
//!
//! The tool itself is project-agnostic — pass `--src`, `--out`, and
//! `--md-out` to point it at any Kestrel source tree. For the
//! kestrel-lang stdlib specifically, the `stdlib-docs` skill wraps this
//! with the right paths.

use clap::{Parser, ValueEnum};
use kestrel_compiler::Compiler;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Format {
    Json,
    Markdown,
    Both,
}

#[derive(Parser)]
#[command(
    name = "kestrel-doc",
    about = "Generate documentation data for a Kestrel source tree"
)]
struct Cli {
    /// Directory of .ks sources to document (recursive).
    #[arg(long, default_value = ".")]
    src: PathBuf,

    /// Output directory for the JSON snapshot (rustdoc-style, one
    /// `<module>.json` per module plus an `index.json`).
    #[arg(long, default_value = "doc/json")]
    out: PathBuf,

    /// Output directory for the markdown snapshot (Context7/LLM-
    /// friendly, one `<module>.md` per module). Kept separate from
    /// `--out` so the two formats can land in different trees.
    #[arg(long, default_value = "doc/markdown")]
    md_out: PathBuf,

    /// Output format. `json` is the rustdoc-style snapshot; `markdown`
    /// is the Context7/LLM-friendly form. Defaults to emitting both.
    #[arg(long, value_enum, default_value_t = Format::Both)]
    format: Format,

    /// Emit a single bundled `docs.json` instead of one file per module.
    /// The output contains `{"index": …, "modules": {"path": …, …}}`.
    /// Useful for uploading package docs to the registry in one shot.
    #[arg(long, default_value_t = false)]
    bundle: bool,
}

#[derive(Serialize)]
struct BundledDocs {
    index: kestrel_doc::ModuleIndex,
    modules: HashMap<String, kestrel_doc::ModulePage>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if !cli.src.is_dir() {
        eprintln!("error: --src {:?} is not a directory", cli.src);
        return ExitCode::FAILURE;
    }

    let mut compiler = Compiler::new();
    compiler.load_dir(&cli.src);

    let (index, pages) = kestrel_doc::extract(compiler.world(), compiler.root());

    let want_json = matches!(cli.format, Format::Json | Format::Both);
    let want_md = matches!(cli.format, Format::Markdown | Format::Both);

    if want_json {
        if let Err(e) = std::fs::create_dir_all(&cli.out) {
            eprintln!("error: failed to create {:?}: {}", cli.out, e);
            return ExitCode::FAILURE;
        }

        if cli.bundle {
            let bundled = BundledDocs {
                index: index.clone(),
                modules: pages.iter().map(|p| (p.path.clone(), p.clone())).collect(),
            };
            let bundle_path = cli.out.join("docs.json");
            if let Err(e) = write_json(&bundle_path, &bundled) {
                eprintln!("error: writing {:?}: {}", bundle_path, e);
                return ExitCode::FAILURE;
            }
        } else {
            let index_path = cli.out.join("index.json");
            if let Err(e) = write_json(&index_path, &index) {
                eprintln!("error: writing {:?}: {}", index_path, e);
                return ExitCode::FAILURE;
            }
            for page in &pages {
                let page_path = cli.out.join(format!("{}.json", page.path));
                if let Err(e) = write_json(&page_path, page) {
                    eprintln!("error: writing {:?}: {}", page_path, e);
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    if want_md {
        if let Err(e) = std::fs::create_dir_all(&cli.md_out) {
            eprintln!("error: failed to create {:?}: {}", cli.md_out, e);
            return ExitCode::FAILURE;
        }
        for page in &pages {
            let page_path = cli.md_out.join(format!("{}.md", page.path));
            let md = kestrel_doc::markdown::render(page);
            if let Err(e) = std::fs::write(&page_path, md) {
                eprintln!("error: writing {:?}: {}", page_path, e);
                return ExitCode::FAILURE;
            }
        }
    }

    let total_items: usize = index.modules.iter().map(|m| m.item_count).sum();
    let dest = match cli.format {
        Format::Json => cli.out.display().to_string(),
        Format::Markdown => cli.md_out.display().to_string(),
        Format::Both => format!("{} + {}", cli.out.display(), cli.md_out.display()),
    };
    eprintln!(
        "wrote {} modules, {} items ({:?}) → {}",
        pages.len(),
        total_items,
        cli.format,
        dest,
    );
    ExitCode::SUCCESS
}

fn write_json<T: serde::Serialize>(path: &std::path::Path, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}
