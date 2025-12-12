use kestrel_lexer::lex;
use kestrel_parser::parse_ty_from_source;
use kestrel_syntax_tree::SyntaxNode;
use std::env;
use std::fs;

fn print_syntax_tree(node: &SyntaxNode, indent: usize) {
    let indent_str = "  ".repeat(indent);

    // Print the node kind
    println!("{}Node: {:?}", indent_str, node.kind());

    // Print all children (tokens and nodes)
    for child in node.children_with_tokens() {
        if let Some(child_node) = child.as_node() {
            print_syntax_tree(child_node, indent + 1);
        } else if let Some(token) = child.as_token() {
            println!(
                "{}  Token: {:?} = \"{}\"",
                indent_str,
                token.kind(),
                token.text()
            );
        }
    }
}

fn parse_type_file(path: &str) {
    println!("\n{}", "=".repeat(70));
    println!("Parsing type file: {}", path);
    println!("{}", "=".repeat(70));

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return;
        }
    };

    // Remove comments and empty lines for display
    let code = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    println!("\nSource code:");
    println!("{}", code);
    println!();

    // Lex the code
    let tokens: Vec<_> = lex(&code, 0)
        .filter_map(|t| t.ok())
        .map(|spanned| (spanned.value, spanned.span))
        .collect();

    println!(
        "Tokens: {:?}\n",
        tokens.iter().map(|(t, _)| t).collect::<Vec<_>>()
    );

    // Parse the type
    let ty = parse_ty_from_source(&code, tokens.into_iter());

    println!("Type analysis:");
    println!("  is_unit: {}", ty.is_unit());
    println!("  is_never: {}", ty.is_never());
    println!("  is_tuple: {}", ty.is_tuple());
    println!("  is_function: {}", ty.is_function());
    println!("  is_path: {}", ty.is_path());

    if let Some(segments) = ty.path_segments() {
        println!("  path_segments: {:?}", segments);
    }

    if let Some(count) = ty.tuple_element_count() {
        println!("  tuple_element_count: {}", count);
    }

    println!("\nSyntax Tree:");
    print_syntax_tree(&ty.syntax, 0);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        // Parse files passed as arguments
        for path in &args[1..] {
            parse_type_file(path);
        }
    } else {
        // Parse all test files in tests/ty/
        let test_files = vec![
            "tests/ty/unit.ks",
            "tests/ty/never.ks",
            "tests/ty/path.ks",
            "tests/ty/tuple.ks",
            "tests/ty/function.ks",
        ];

        for file in test_files {
            parse_type_file(file);
        }
    }

    println!("\n{}", "=".repeat(70));
}
