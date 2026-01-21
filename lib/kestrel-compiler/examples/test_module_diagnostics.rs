use kestrel_compiler::Compilation;

fn main() {
    println!("=== Testing Module Validation Diagnostics ===\n");

    // Test 1: No module declaration
    println!("Test 1: No Module Declaration");
    println!("{}", "-".repeat(50));
    let no_module_source = r#"
public class Test {}
"#;
    let compilation = Compilation::builder()
        .add_source("no_module.ks", no_module_source)
        .build()
        .expect("failed to build compilation");

    if compilation.has_errors() {
        println!("✓ Error detected as expected!");
        compilation.diagnostics().emit().unwrap();
    }

    println!("\n");

    // Test 2: Module not first
    println!("Test 2: Module Not First");
    println!("{}", "-".repeat(50));
    let module_not_first = r#"
public class Test {}
module MyModule
"#;
    let compilation2 = Compilation::builder()
        .add_source("module_not_first.ks", module_not_first)
        .build()
        .expect("failed to build compilation");

    if compilation2.has_errors() {
        println!("✓ Error detected as expected!");
        compilation2.diagnostics().emit().unwrap();
    }

    println!("\n");

    // Test 3: Multiple modules
    println!("Test 3: Multiple Module Declarations");
    println!("{}", "-".repeat(50));
    let multiple_modules = r#"
module First
module Second
module Third
"#;
    let compilation3 = Compilation::builder()
        .add_source("multiple_modules.ks", multiple_modules)
        .build()
        .expect("failed to build compilation");

    if compilation3.has_errors() {
        println!("✓ Error detected as expected!");
        compilation3.diagnostics().emit().unwrap();
    }

    println!("\n=== All Tests Complete ===");
}
