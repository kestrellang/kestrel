//! Attribute parsing
//!
//! This module is the single source of truth for attribute parsing.
//! Attributes use the `@name(args)` syntax and can be attached to declarations.

use chumsky::prelude::*;
use kestrel_lexer::Token;

use crate::common::data::{AttributeArgData, AttributeArgValue, AttributeArgsData, AttributeData};
use crate::common::parsers::{identifier, skip_trivia, token};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};

// =============================================================================
// Attribute Argument Value Parsers
// =============================================================================

/// Parser for string literal in attribute arguments
fn string_literal_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgValue, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        just(Token::String).map_with(|_, e| AttributeArgValue::String(to_kestrel_span(e.span()))),
    )
}

/// Parser for integer literal in attribute arguments
fn integer_literal_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgValue, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        just(Token::Integer).map_with(|_, e| AttributeArgValue::Integer(to_kestrel_span(e.span()))),
    )
}

/// Parser for float literal in attribute arguments
fn float_literal_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgValue, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        just(Token::Float).map_with(|_, e| AttributeArgValue::Float(to_kestrel_span(e.span()))),
    )
}

/// Parser for boolean literal in attribute arguments
fn bool_literal_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgValue, ParserExtra<'tokens>> + Clone {
    skip_trivia().ignore_then(
        just(Token::Boolean).map_with(|_, e| AttributeArgValue::Bool(to_kestrel_span(e.span()))),
    )
}

/// Parser for implicit member access in attribute arguments: `.option`
fn implicit_member_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgValue, ParserExtra<'tokens>> + Clone {
    token(Token::Dot)
        .then(identifier())
        .map(|(dot_span, name_span)| AttributeArgValue::ImplicitMember {
            dot_span,
            name_span,
        })
}

/// Parser for path in attribute arguments: `SomeType` or `Module.Type`
fn path_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgValue, ParserExtra<'tokens>> + Clone {
    identifier()
        .separated_by(token(Token::Dot))
        .at_least(1)
        .collect::<Vec<_>>()
        .map(AttributeArgValue::Path)
}

/// Parser for attribute argument value
///
/// Accepts: string, integer, float, boolean, implicit member access, or path
fn attribute_arg_value_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgValue, ParserExtra<'tokens>> + Clone {
    string_literal_parser()
        .or(float_literal_parser()) // Float before integer to handle "3.14" correctly
        .or(integer_literal_parser())
        .or(bool_literal_parser())
        .or(implicit_member_parser())
        .or(path_parser())
}

// =============================================================================
// Attribute Argument Parser
// =============================================================================

/// Parser for a single attribute argument
///
/// Syntax: `value` or `label: value`
fn attribute_arg_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgData, ParserExtra<'tokens>> + Clone {
    // Try labeled argument first: `label: value`
    let labeled = identifier()
        .then(token(Token::Colon))
        .then(attribute_arg_value_parser())
        .map(|((label, colon), value)| AttributeArgData {
            label: Some(label),
            colon: Some(colon),
            value,
        });

    // Unlabeled argument: just `value`
    let unlabeled = attribute_arg_value_parser().map(|value| AttributeArgData {
        label: None,
        colon: None,
        value,
    });

    labeled.or(unlabeled)
}

/// Parser for attribute arguments list: `(arg1, arg2, ...)`
fn attribute_args_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeArgsData, ParserExtra<'tokens>> + Clone {
    token(Token::LParen)
        .then(
            attribute_arg_parser()
                .separated_by(token(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then(token(Token::RParen))
        .map(|((lparen_span, args), rparen_span)| AttributeArgsData {
            lparen_span,
            args,
            rparen_span,
        })
}

// =============================================================================
// Attribute Parser
// =============================================================================

/// Parser for a single attribute: `@name` or `@name(args)`
pub fn attribute_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, AttributeData, ParserExtra<'tokens>> + Clone {
    token(Token::At)
        .then(identifier())
        .then(attribute_args_parser().or_not())
        .map(|((at_span, name_span), args)| AttributeData {
            at_span,
            name_span,
            args,
        })
}

/// Parser for a list of attributes (zero or more)
///
/// This parser is used before declaration parsers to collect any attributes.
pub fn attribute_list_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Vec<AttributeData>, ParserExtra<'tokens>> + Clone {
    attribute_parser().repeated().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{create_input, prepare_tokens};
    use kestrel_lexer::lex;

    /// Helper to parse attributes from source
    fn parse_attributes(source: &str) -> Vec<AttributeData> {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let prepared = prepare_tokens(tokens.into_iter());
        let input = create_input(&prepared, source.len());

        attribute_list_parser()
            .parse(input)
            .into_result()
            .unwrap_or_default()
    }

    #[test]
    fn test_simple_attribute() {
        let attrs = parse_attributes("@dummy");
        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].args.is_none());
    }

    #[test]
    fn test_attribute_with_empty_parens() {
        let attrs = parse_attributes("@dummy()");
        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].args.is_some());
        assert_eq!(attrs[0].args.as_ref().unwrap().args.len(), 0);
    }

    #[test]
    fn test_attribute_with_string_arg() {
        let attrs = parse_attributes("@dummy(\"hello\")");
        assert_eq!(attrs.len(), 1);
        let args = attrs[0].args.as_ref().unwrap();
        assert_eq!(args.args.len(), 1);
        assert!(matches!(args.args[0].value, AttributeArgValue::String(_)));
    }

    #[test]
    fn test_attribute_with_integer_arg() {
        let attrs = parse_attributes("@dummy(42)");
        assert_eq!(attrs.len(), 1);
        let args = attrs[0].args.as_ref().unwrap();
        assert_eq!(args.args.len(), 1);
        assert!(matches!(args.args[0].value, AttributeArgValue::Integer(_)));
    }

    #[test]
    fn test_attribute_with_labeled_arg() {
        let attrs = parse_attributes("@available(iOS: 15)");
        assert_eq!(attrs.len(), 1);
        let args = attrs[0].args.as_ref().unwrap();
        assert_eq!(args.args.len(), 1);
        assert!(args.args[0].label.is_some());
    }

    #[test]
    fn test_attribute_with_multiple_args() {
        let attrs = parse_attributes("@available(iOS: 15, macOS: 12)");
        assert_eq!(attrs.len(), 1);
        let args = attrs[0].args.as_ref().unwrap();
        assert_eq!(args.args.len(), 2);
    }

    #[test]
    fn test_multiple_attributes() {
        let attrs = parse_attributes("@dummy @available(iOS: 15)");
        assert_eq!(attrs.len(), 2);
    }

    #[test]
    fn test_attribute_with_implicit_member() {
        let attrs = parse_attributes("@dummy(.option)");
        assert_eq!(attrs.len(), 1);
        let args = attrs[0].args.as_ref().unwrap();
        assert_eq!(args.args.len(), 1);
        assert!(matches!(
            args.args[0].value,
            AttributeArgValue::ImplicitMember { .. }
        ));
    }

    #[test]
    fn test_attribute_with_bool_arg() {
        let attrs = parse_attributes("@dummy(true)");
        assert_eq!(attrs.len(), 1);
        let args = attrs[0].args.as_ref().unwrap();
        assert_eq!(args.args.len(), 1);
        assert!(matches!(args.args[0].value, AttributeArgValue::Bool(_)));
    }

    #[test]
    fn test_attribute_with_path_arg() {
        let attrs = parse_attributes("@dummy(SomeType)");
        assert_eq!(attrs.len(), 1);
        let args = attrs[0].args.as_ref().unwrap();
        assert_eq!(args.args.len(), 1);
        assert!(matches!(args.args[0].value, AttributeArgValue::Path(_)));
    }
}
