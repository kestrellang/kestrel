use kestrel_compiler::Compilation;

fn main() {
    println!("=== Kestrel Compiler Example ===\n");

    // Example 1: Simple compilation with valid code
    println!("Example 1: Valid code");
    println!("{}", "-".repeat(50));

    let source = r#"
module Math.Vector

public class Vector2D {
    public fn length() {}
}

public class Vector3D {
    public fn normalize() {}
}
"#;

    let compilation = Compilation::builder()
        .add_source("vector.ks", source)
        .build();

    if compilation.has_errors() {
        println!("Compilation failed!");
        compilation.diagnostics().emit().unwrap();
    } else {
        println!("✓ Compilation successful!");
        println!("  Compiled {} file(s)", compilation.source_files().len());

        for file in compilation.source_files() {
            println!("\n  File: {}", file.name());
            println!("  Source length: {} bytes", file.source().len());
        }

        // Access the unified semantic model
        if let Some(semantic_model) = compilation.semantic_model() {
            println!("\n  Semantic model: {} top-level symbols",
                     semantic_model.root().metadata().children().len());
        }
    }

    println!("\n");

    // Example 2: Multiple files
    println!("Example 2: Multiple files");
    println!("{}", "-".repeat(50));

    let main_source = r#"
module Main

import Math.Vector

public class Application {
    public fn run() {}
}
"#;

    let utils_source = r#"
module Utils

public class Helper {
    public fn assist() {}
}
"#;

    let compilation = Compilation::builder()
        .add_source("main.ks", main_source)
        .add_source("utils.ks", utils_source)
        .build();

    if compilation.has_errors() {
        println!("Compilation failed!");
        compilation.diagnostics().emit().unwrap();
    } else {
        println!("✓ Compilation successful!");
        println!("  Compiled {} file(s)", compilation.source_files().len());

        for file in compilation.source_files() {
            println!("\n  File: {}", file.name());
        }
    }

    println!("\n");

    // Example 3: Code with errors
    println!("Example 3: Code with errors");
    println!("{}", "-".repeat(50));

    let invalid_source = r#"
module Test

public class Bad {
    @invalid_token@
}
"#;

    let compilation = Compilation::builder()
        .add_source("bad.ks", invalid_source)
        .build();

    if compilation.has_errors() {
        println!("Compilation failed as expected!");
        println!("Total diagnostics: {}\n", compilation.diagnostics().len());
        compilation.diagnostics().emit().unwrap();
    } else {
        println!("✓ No errors detected");
    }

    println!("\n=== End of Examples ===");
}
