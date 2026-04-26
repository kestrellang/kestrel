//! kestrel-doc: emit a rustdoc-style JSON snapshot of a Kestrel source tree.
//!
//! Loads .ks files from a directory through `Compiler::load_dir`, walks the
//! resulting `World`, and writes one `<module>.json` per module plus an
//! `index.json` summary into the output directory.

use clap::Parser;
use kestrel_compiler::Compiler;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "kestrel-doc",
    about = "Generate JSON documentation data for a Kestrel source tree"
)]
struct Cli {
    /// Directory of .ks sources to document (recursive).
    #[arg(long, default_value = "lang/std")]
    src: PathBuf,

    /// Output directory for the generated JSON files.
    #[arg(long, default_value = "site/public/stdlib")]
    out: PathBuf,
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

    if let Err(e) = std::fs::create_dir_all(&cli.out) {
        eprintln!("error: failed to create {:?}: {}", cli.out, e);
        return ExitCode::FAILURE;
    }

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

    let total_items: usize = index.modules.iter().map(|m| m.item_count).sum();
    eprintln!(
        "wrote {} modules, {} items → {}",
        pages.len(),
        total_items,
        cli.out.display()
    );
    ExitCode::SUCCESS
}

fn write_json<T: serde::Serialize>(path: &std::path::Path, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}
