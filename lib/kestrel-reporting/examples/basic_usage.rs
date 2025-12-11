use kestrel_reporting::{DiagnosticContext, IntoDiagnostic, Diagnostic, Label, Severity};
use kestrel_span::Span;

/// Example error type that implements IntoDiagnostic
struct SyntaxError {
    message: String,
    span: Span,
}

impl IntoDiagnostic for SyntaxError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(&self.message)
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("syntax error here")
            ])
    }
}

/// Example warning type
struct UnusedVariable {
    name: String,
    span: Span,
}

impl IntoDiagnostic for UnusedVariable {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message(format!("unused variable `{}`", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("not used in this scope")
            ])
            .with_notes(vec![
                format!("consider prefixing with an underscore: `_{}`", self.name)
            ])
    }
}

fn main() {
    // Create a diagnostic context
    let mut ctx = DiagnosticContext::new();

    // Add source files
    let source1 = "fn main() {\n    let x = 5\n}\n";
    let file_id = ctx.add_file("example.kes".to_string(), source1.to_string());

    // Example 1: Throw a syntax error using the IntoDiagnostic trait
    let error = SyntaxError {
        message: "expected `;` after expression".to_string(),
        span: Span::new(file_id, 23..24),
    };
    ctx.throw(error);

    // Example 2: Add a warning about unused variable
    let warning = UnusedVariable {
        name: "x".to_string(),
        span: Span::new(file_id, 20..21),
    };
    ctx.throw(warning);

    // Example 3: Manually create a diagnostic for more complex cases
    let complex_diagnostic = Diagnostic::error()
        .with_message("type mismatch")
        .with_labels(vec![
            Label::primary(file_id, 16..17)
                .with_message("expected `String`, found `i32`"),
            Label::secondary(file_id, 20..21)
                .with_message("this has type `i32`"),
        ])
        .with_notes(vec![
            "help: you can convert an integer to a string using `.to_string()`".to_string()
        ]);
    ctx.add_diagnostic(complex_diagnostic);

    // Check if there are errors
    println!("Has errors: {}", ctx.has_errors());
    println!("Total diagnostics: {}\n", ctx.len());

    // Emit all diagnostics to stderr
    if let Err(e) = ctx.emit() {
        eprintln!("Failed to emit diagnostics: {}", e);
    }
}
