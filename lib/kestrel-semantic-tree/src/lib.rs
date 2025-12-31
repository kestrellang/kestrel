pub mod attributes;
pub mod behavior;
pub mod error;
pub mod expr;
pub mod language;
pub mod operators;
pub mod pattern;
pub mod stmt;
pub mod symbol;
pub mod ty;

#[cfg(test)]
mod tests {
    use kestrel_span::Span;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
