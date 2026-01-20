use crate::compilation::Compilation;
use crate::stdlib::{StdLib, StdLibConfig, StdLibError};
use std::fs;
use std::io;
use std::path::PathBuf;

/// Builder for creating a `Compilation`.
///
/// Use this to add source files from strings or file paths,
/// then call `build()` to compile all sources.
#[derive(Default)]
pub struct CompilationBuilder {
    sources: Vec<(String, String)>, // (name, content) pairs
    stdlib_config: StdLibConfig,
}

impl CompilationBuilder {
    /// Create a new compilation builder.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            stdlib_config: StdLibConfig::default(),
        }
    }

    /// Configure the standard library path.
    pub fn with_std_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdlib_config = self.stdlib_config.with_path(path);
        self
    }

    /// Disable the standard library.
    pub fn without_std(mut self) -> Self {
        self.stdlib_config = StdLibConfig::disabled();
        self
    }

    /// Add a source file from a string.
    ///
    /// # Arguments
    /// * `name` - The name of the file (e.g., "main.ks")
    /// * `source` - The source code content
    ///
    /// # Example
    /// ```no_run
    /// # use kestrel_compiler::CompilationBuilder;
    /// let builder = CompilationBuilder::new()
    ///     .add_source("main.ks", "module Main\nclass Foo {}");
    /// ```
    pub fn add_source(mut self, name: impl Into<String>, source: impl Into<String>) -> Self {
        self.sources.push((name.into(), source.into()));
        self
    }

    /// Add a source file from a file path.
    ///
    /// Reads the file from disk and adds it to the compilation.
    ///
    /// # Arguments
    /// * `path` - The path to the file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read.
    ///
    /// # Example
    /// ```no_run
    /// # use kestrel_compiler::CompilationBuilder;
    /// let builder = CompilationBuilder::new()
    ///     .add_file("src/main.ks")
    ///     .unwrap();
    /// ```
    pub fn add_file(mut self, path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let source = fs::read_to_string(path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.sources.push((name, source));
        Ok(self)
    }

    /// Build the compilation.
    ///
    /// This performs lexing, parsing, and semantic analysis on all source files.
    /// Diagnostics are collected automatically during this process.
    ///
    /// # Example
    /// ```no_run
    /// # use kestrel_compiler::CompilationBuilder;
    /// let compilation = CompilationBuilder::new()
    ///     .add_source("main.ks", "module Main\nclass Foo {}")
    ///     .build();
    ///
    /// if compilation.has_errors() {
    ///     compilation.diagnostics().emit().unwrap();
    /// }
    /// ```
    pub fn build(self) -> Result<Compilation, StdLibError> {
        // Load stdlib first if enabled
        let stdlib_sources = StdLib::load(&self.stdlib_config)?
            .map(|s| s.sources)
            .unwrap_or_default();

        // Combine: stdlib first, then user sources
        let all_sources: Vec<_> = stdlib_sources.into_iter().chain(self.sources).collect();

        Ok(Compilation::from_sources(all_sources, self.stdlib_config.enabled))
    }
}
