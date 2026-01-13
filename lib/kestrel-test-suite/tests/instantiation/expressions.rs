//! Tests for expression and statement data types.
//!
//! These tests exercise the new Expression, Statement, and CodeBlock types
//! to ensure they work correctly with the semantic tree.

use kestrel_span::Span;
use kestrel_test_suite::*;

mod expression_types {
    use super::*;

    #[test]
    fn expression_struct_creation() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression, LiteralValue};

        // Test creating various expression types directly
        let int_expr = Expression::integer(42, Span::new(0, 0..2));
        assert!(matches!(
            int_expr.kind,
            ExprKind::Literal(LiteralValue::Integer(42))
        ));
        assert!(int_expr.ty.is_int());

        let float_expr = Expression::float(3.14, Span::new(0, 0..4));
        assert!(matches!(
            int_expr.kind,
            ExprKind::Literal(LiteralValue::Integer(_))
        ));
        assert!(float_expr.ty.is_float());

        let string_expr = Expression::string("hello".to_string(), Span::new(0, 0..7));
        assert!(string_expr.ty.is_string());

        let bool_expr = Expression::bool(true, Span::new(0, 0..4));
        assert!(bool_expr.ty.is_bool());

        let unit_expr = Expression::unit(Span::new(0, 0..2));
        assert!(unit_expr.ty.is_unit());
    }

    #[test]
    fn expression_array_creation() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let elements = vec![
            Expression::integer(1, Span::new(0, 1..2)),
            Expression::integer(2, Span::new(0, 4..5)),
            Expression::integer(3, Span::new(0, 7..8)),
        ];
        let element_ty = Ty::int(IntBits::I64, Span::new(0, 0..0));
        let array_expr = Expression::array(elements, element_ty, Span::new(0, 0..10));

        assert!(matches!(array_expr.kind, ExprKind::Array(_)));
        assert!(array_expr.ty.is_array());
    }

    #[test]
    fn expression_tuple_creation() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};

        let elements = vec![
            Expression::integer(1, Span::new(0, 1..2)),
            Expression::string("hello".to_string(), Span::new(0, 4..11)),
        ];
        let tuple_expr = Expression::tuple(elements, Span::new(0, 0..12));

        assert!(matches!(tuple_expr.kind, ExprKind::Tuple(_)));
        assert!(tuple_expr.ty.is_tuple());
    }

    #[test]
    fn expression_grouping_creation() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};

        let inner = Expression::integer(42, Span::new(0, 1..3));
        let grouped = Expression::grouping(inner, Span::new(0, 0..4));

        assert!(matches!(grouped.kind, ExprKind::Grouping(_)));
        // Grouping should preserve the inner type
        assert!(grouped.ty.is_int());
    }

    #[test]
    fn expression_local_ref() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let local_id = LocalId(0);
        let ty = Ty::int(IntBits::I64, Span::new(0, 0..3));
        let local_ref = Expression::local_ref(local_id, ty, true, Span::new(0, 0..1));

        assert!(matches!(local_ref.kind, ExprKind::LocalRef(id) if id == LocalId(0)));
        assert!(local_ref.is_mutable());
    }

    #[test]
    fn expression_symbol_ref() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};
        use kestrel_semantic_tree::ty::{IntBits, Ty};
        use semantic_tree::symbol::SymbolId;

        let symbol_id = SymbolId::new();
        let ty = Ty::int(IntBits::I64, Span::new(0, 0..3));
        let symbol_ref = Expression::symbol_ref(symbol_id, ty, false, Span::new(0, 0..5));

        assert!(matches!(symbol_ref.kind, ExprKind::SymbolRef(_)));
        assert!(!symbol_ref.is_mutable());
    }

    #[test]
    fn expression_overloaded_ref() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};
        use semantic_tree::symbol::SymbolId;

        let candidates = vec![SymbolId::new(), SymbolId::new(), SymbolId::new()];
        let overloaded = Expression::overloaded_ref(candidates, Span::new(0, 0..5));

        assert!(matches!(overloaded.kind, ExprKind::OverloadedRef(ref c) if c.len() == 3));
        // Type should be inferred (unknown until call resolution)
        assert!(overloaded.ty.is_infer());
    }

    #[test]
    fn expression_error() {
        use kestrel_semantic_tree::expr::Expression;

        let error_expr = Expression::error(Span::new(0, 0..5));

        assert!(error_expr.is_error());
        assert!(error_expr.ty.is_error());
    }
}

mod statement_types {
    use super::*;
    #[test]
    fn statement_let_creation() {
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 10..12)),
            Span::new(0, 0..1),
        );
        let init = Expression::integer(42, Span::new(0, 10..12));
        let stmt = Statement::binding(pattern, Some(init), Span::new(0, 0..13));

        assert!(stmt.is_binding());
        assert!(!stmt.is_expr());
        assert_eq!(stmt.pattern().and_then(|p| p.local_id()), Some(LocalId(0)));
        assert_eq!(
            stmt.pattern().and_then(|p| p.mutability()),
            Some(Mutability::Immutable)
        );
    }

    #[test]
    fn statement_var_creation() {
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::Ty;

        let pattern = Pattern::local(
            LocalId(1),
            Mutability::Mutable,
            "y".to_string(),
            Ty::string(Span::new(0, 10..17)),
            Span::new(0, 0..1),
        );
        let init = Expression::string("hello".to_string(), Span::new(0, 10..17));
        let stmt = Statement::binding(pattern, Some(init), Span::new(0, 0..18));

        assert!(stmt.is_binding());
        assert!(!stmt.is_expr());
        assert_eq!(stmt.pattern().and_then(|p| p.local_id()), Some(LocalId(1)));
        assert_eq!(
            stmt.pattern().and_then(|p| p.mutability()),
            Some(Mutability::Mutable)
        );
    }

    #[test]
    fn statement_without_initializer() {
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..3)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(pattern, None, Span::new(0, 0..10));

        assert!(stmt.is_binding());
        assert_eq!(stmt.pattern().and_then(|p| p.local_id()), Some(LocalId(0)));
    }

    #[test]
    fn statement_expr_creation() {
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::stmt::Statement;

        let expr = Expression::unit(Span::new(0, 0..2));
        let stmt = Statement::expr(expr, Span::new(0, 0..3));

        assert!(!stmt.is_binding());
        assert!(stmt.is_expr());
        assert!(stmt.pattern().is_none());
    }

    #[test]
    fn statement_span() {
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..3)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(pattern, None, Span::new(0, 5..15));
        assert_eq!(stmt.span, Span::new(0, 5..15));
    }
}

mod code_block_types {
    use super::*;
    #[test]
    fn code_block_empty() {
        use kestrel_semantic_tree::behavior::executable::CodeBlock;

        let block = CodeBlock::empty();

        assert!(block.is_empty());
        assert!(block.statements.is_empty());
        assert!(block.yield_expr().is_none());
    }

    #[test]
    fn code_block_with_statements() {
        use kestrel_semantic_tree::behavior::executable::CodeBlock;
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern1 = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..1)),
            Span::new(0, 0..1),
        );
        let pattern2 = Pattern::local(
            LocalId(1),
            Mutability::Immutable,
            "y".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 11..12)),
            Span::new(0, 11..12),
        );
        let stmt1 = Statement::binding(
            pattern1,
            Some(Expression::integer(1, Span::new(0, 0..1))),
            Span::new(0, 0..10),
        );
        let stmt2 = Statement::binding(
            pattern2,
            Some(Expression::integer(2, Span::new(0, 11..12))),
            Span::new(0, 11..21),
        );

        let block = CodeBlock::new(vec![stmt1, stmt2], None);

        assert!(!block.is_empty());
        assert_eq!(block.statements.len(), 2);
        assert!(block.yield_expr().is_none());
    }

    #[test]
    fn code_block_with_yield() {
        use kestrel_semantic_tree::behavior::executable::CodeBlock;
        use kestrel_semantic_tree::expr::Expression;

        let yield_expr = Expression::integer(42, Span::new(0, 0..2));
        let block = CodeBlock::new(vec![], Some(yield_expr));

        assert!(!block.is_empty());
        assert!(block.statements.is_empty());
        assert!(block.yield_expr().is_some());
    }

    #[test]
    fn code_block_with_statements_and_yield() {
        use kestrel_semantic_tree::behavior::executable::CodeBlock;
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..1)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(
            pattern,
            Some(Expression::integer(1, Span::new(0, 0..1))),
            Span::new(0, 0..10),
        );
        let yield_expr = Expression::integer(42, Span::new(0, 11..13));

        let block = CodeBlock::new(vec![stmt], Some(yield_expr));

        assert!(!block.is_empty());
        assert_eq!(block.statements.len(), 1);
        assert!(block.yield_expr().is_some());
    }
}

mod executable_behavior {
    use super::*;
    #[test]
    fn executable_behavior_creation() {
        use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
        use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
        use semantic_tree::behavior::Behavior;

        let block = CodeBlock::empty();
        let behavior = ExecutableBehavior::new(block);

        assert_eq!(behavior.kind(), KestrelBehaviorKind::Executable);
        assert!(behavior.body().is_empty());
    }

    #[test]
    fn executable_behavior_with_body() {
        use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..1)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(
            pattern,
            Some(Expression::integer(1, Span::new(0, 0..1))),
            Span::new(0, 0..10),
        );
        let yield_expr = Expression::integer(42, Span::new(0, 11..13));
        let block = CodeBlock::new(vec![stmt], Some(yield_expr));

        let behavior = ExecutableBehavior::new(block);

        assert_eq!(behavior.body().statements.len(), 1);
        assert!(behavior.body().yield_expr().is_some());
    }

    #[test]
    fn executable_behavior_mutable_body() {
        use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let block = CodeBlock::empty();
        let mut behavior = ExecutableBehavior::new(block);

        // Initially empty
        assert!(behavior.body().is_empty());

        // Add a statement via mutable access
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..1)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(
            pattern,
            Some(Expression::integer(1, Span::new(0, 0..1))),
            Span::new(0, 0..10),
        );
        behavior.body_mut().statements.push(stmt);

        assert_eq!(behavior.body().statements.len(), 1);
    }
}

mod literal_value_equality {
    use super::*;
    #[test]
    fn literal_values_equal() {
        use kestrel_semantic_tree::expr::LiteralValue;

        assert_eq!(LiteralValue::Unit, LiteralValue::Unit);
        assert_eq!(LiteralValue::Integer(42), LiteralValue::Integer(42));
        assert_eq!(LiteralValue::Float(3.14), LiteralValue::Float(3.14));
        assert_eq!(
            LiteralValue::String("hello".to_string()),
            LiteralValue::String("hello".to_string())
        );
        assert_eq!(LiteralValue::Bool(true), LiteralValue::Bool(true));
    }

    #[test]
    fn literal_values_not_equal() {
        use kestrel_semantic_tree::expr::LiteralValue;

        assert_ne!(LiteralValue::Integer(42), LiteralValue::Integer(43));
        assert_ne!(LiteralValue::Bool(true), LiteralValue::Bool(false));
        assert_ne!(
            LiteralValue::String("hello".to_string()),
            LiteralValue::String("world".to_string())
        );
    }
}

mod cloning {
    use super::*;
    #[test]
    fn expression_clone_preserves_type_and_span() {
        use kestrel_semantic_tree::expr::Expression;

        let expr = Expression::integer(42, Span::new(0, 0..2));
        let cloned = expr.clone();

        assert!(cloned.ty.is_int());
        assert_eq!(cloned.span, Span::new(0, 0..2));
        assert_eq!(cloned.span, expr.span);
    }

    #[test]
    fn expression_clone_with_different_types() {
        use kestrel_semantic_tree::expr::Expression;

        let string_expr = Expression::string("test".to_string(), Span::new(0, 5..10));
        let cloned_string = string_expr.clone();
        assert!(cloned_string.ty.is_string());
        assert_eq!(cloned_string.span, Span::new(0, 5..10));

        let bool_expr = Expression::bool(false, Span::new(0, 0..5));
        let cloned_bool = bool_expr.clone();
        assert!(cloned_bool.ty.is_bool());
        assert_eq!(cloned_bool.span, Span::new(0, 0..5));

        let unit_expr = Expression::unit(Span::new(0, 10..12));
        let cloned_unit = unit_expr.clone();
        assert!(cloned_unit.ty.is_unit());
    }

    #[test]
    fn statement_clone_preserves_binding_properties() {
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..2)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(
            pattern,
            Some(Expression::integer(42, Span::new(0, 0..2))),
            Span::new(0, 0..10),
        );
        let cloned = stmt.clone();

        assert!(cloned.is_binding());
        assert_eq!(
            cloned.pattern().and_then(|p| p.local_id()),
            Some(LocalId(0))
        );
        assert_eq!(
            cloned.pattern().and_then(|p| p.mutability()),
            Some(Mutability::Immutable)
        );
        assert_eq!(cloned.span, Span::new(0, 0..10));
    }

    #[test]
    fn statement_clone_with_mutable_variable() {
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::Ty;

        let pattern = Pattern::local(
            LocalId(5),
            Mutability::Mutable,
            "y".to_string(),
            Ty::string(Span::new(0, 0..6)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(
            pattern,
            Some(Expression::string("hello".to_string(), Span::new(0, 0..6))),
            Span::new(0, 0..15),
        );
        let cloned = stmt.clone();

        assert!(cloned.is_binding());
        assert_eq!(
            cloned.pattern().and_then(|p| p.mutability()),
            Some(Mutability::Mutable)
        );
        assert_eq!(
            cloned.pattern().and_then(|p| p.local_id()),
            Some(LocalId(5))
        );
    }

    #[test]
    fn code_block_clone_preserves_structure() {
        use kestrel_semantic_tree::behavior::executable::CodeBlock;
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            Ty::int(IntBits::I64, Span::new(0, 0..1)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(
            pattern,
            Some(Expression::integer(1, Span::new(0, 0..1))),
            Span::new(0, 0..10),
        );
        let yield_expr = Expression::integer(42, Span::new(0, 11..13));
        let block = CodeBlock::new(vec![stmt], Some(yield_expr));
        let cloned = block.clone();

        assert_eq!(cloned.statements.len(), 1);
        assert!(cloned.yield_expr().is_some());
        assert!(!cloned.is_empty());
    }

    #[test]
    fn code_block_clone_empty_block() {
        use kestrel_semantic_tree::behavior::executable::CodeBlock;

        let block = CodeBlock::empty();
        let cloned = block.clone();

        assert!(cloned.is_empty());
        assert!(cloned.statements.is_empty());
        assert!(cloned.yield_expr().is_none());
    }

    #[test]
    fn executable_behavior_clone_empty() {
        use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};

        let block = CodeBlock::empty();
        let behavior = ExecutableBehavior::new(block);
        let cloned = behavior.clone();

        assert!(cloned.body().is_empty());
        assert_eq!(cloned.body().statements.len(), 0);
    }

    #[test]
    fn executable_behavior_clone_with_body() {
        use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
        use kestrel_semantic_tree::expr::Expression;
        use kestrel_semantic_tree::pattern::{Mutability, Pattern};
        use kestrel_semantic_tree::stmt::Statement;
        use kestrel_semantic_tree::symbol::local::LocalId;
        use kestrel_semantic_tree::ty::Ty;

        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "z".to_string(),
            Ty::string(Span::new(0, 0..5)),
            Span::new(0, 0..1),
        );
        let stmt = Statement::binding(
            pattern,
            Some(Expression::string("hi".to_string(), Span::new(0, 0..5))),
            Span::new(0, 0..10),
        );
        let block = CodeBlock::new(vec![stmt], Some(Expression::unit(Span::new(0, 11..13))));
        let behavior = ExecutableBehavior::new(block);
        let cloned = behavior.clone();

        assert_eq!(cloned.body().statements.len(), 1);
        assert!(cloned.body().yield_expr().is_some());
    }
}

mod nested_expressions {
    use super::*;
    #[test]
    fn deeply_nested_array() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};
        use kestrel_semantic_tree::ty::{IntBits, Ty};

        // Create [[1, 2], [3, 4]]
        let inner1 = vec![
            Expression::integer(1, Span::new(0, 0..1)),
            Expression::integer(2, Span::new(0, 3..4)),
        ];
        let inner2 = vec![
            Expression::integer(3, Span::new(0, 0..1)),
            Expression::integer(4, Span::new(0, 3..4)),
        ];

        let element_ty = Ty::int(IntBits::I64, Span::new(0, 0..0));
        let array1 = Expression::array(inner1, element_ty.clone(), Span::new(0, 0..5));
        let array2 = Expression::array(inner2, element_ty.clone(), Span::new(0, 7..12));

        assert!(array1.ty.is_array());
        assert!(array2.ty.is_array());

        let outer_element_ty = Ty::array(element_ty, Span::new(0, 0..0));
        let outer = Expression::array(vec![array1, array2], outer_element_ty, Span::new(0, 0..13));

        assert!(outer.ty.is_array());
        assert!(matches!(outer.kind, ExprKind::Array(_)));
        assert_eq!(outer.span, Span::new(0, 0..13));
    }

    #[test]
    fn nested_tuple_in_array() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};

        // Create [(1, "a"), (2, "b")]
        let tuple1 = Expression::tuple(
            vec![
                Expression::integer(1, Span::new(0, 1..2)),
                Expression::string("a".to_string(), Span::new(0, 4..7)),
            ],
            Span::new(0, 0..8),
        );
        let tuple2 = Expression::tuple(
            vec![
                Expression::integer(2, Span::new(0, 1..2)),
                Expression::string("b".to_string(), Span::new(0, 4..7)),
            ],
            Span::new(0, 10..18),
        );

        assert!(tuple1.ty.is_tuple());
        assert!(tuple2.ty.is_tuple());
        assert_eq!(tuple1.span, Span::new(0, 0..8));
        assert_eq!(tuple2.span, Span::new(0, 10..18));

        let element_ty = tuple1.ty.clone();
        let array = Expression::array(vec![tuple1, tuple2], element_ty, Span::new(0, 0..20));

        assert!(array.ty.is_array());
        assert!(matches!(array.kind, ExprKind::Array(_)));
        assert_eq!(array.span, Span::new(0, 0..20));
    }

    #[test]
    fn nested_grouping() {
        use kestrel_semantic_tree::expr::{ExprKind, Expression};

        // Create (((42)))
        let inner = Expression::integer(42, Span::new(0, 3..5));
        assert!(inner.ty.is_int());
        assert_eq!(inner.span, Span::new(0, 3..5));

        let g1 = Expression::grouping(inner, Span::new(0, 2..6));
        assert!(g1.ty.is_int());
        assert!(matches!(g1.kind, ExprKind::Grouping(_)));

        let g2 = Expression::grouping(g1, Span::new(0, 1..7));
        assert!(g2.ty.is_int());
        assert_eq!(g2.span, Span::new(0, 1..7));

        let g3 = Expression::grouping(g2, Span::new(0, 0..8));
        // All groupings should preserve the Int type
        assert!(g3.ty.is_int());
        assert_eq!(g3.span, Span::new(0, 0..8));
        assert!(matches!(g3.kind, ExprKind::Grouping(_)));
    }
}

/// Integration tests that compile actual Kestrel code
mod integration {
    use super::*;
    use super::*;

    #[test]
    fn functions_with_varied_literal_return_types() {
        Test::new(
            r#"module Test
            func answer() -> Int { 42 }
            func greeting() -> String { "hello" }
            func nothing() { () }
            func flag() -> Bool { true }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("answer")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("greeting")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("nothing")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("flag")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        );
    }

    #[test]
    fn functions_with_aggregate_expressions() {
        Test::new(
            r#"module Test
            func pair() -> (Int, Int) { (1, 2) }
            func numbers() -> [Int] { [1, 2, 3] }
            func complex() -> [(Int, Int)] { [(1, 2), (3, 4)] }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("pair").is(SymbolKind::Function))
        .expect(Symbol::new("numbers").is(SymbolKind::Function))
        .expect(Symbol::new("complex").is(SymbolKind::Function));
    }

    #[test]
    fn struct_with_method() {
        Test::new(
            r#"module Test
            struct Point {
                func origin() -> (Int, Int) { (0, 0) }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Point").is(SymbolKind::Struct))
        .expect(
            Symbol::new("Point.origin")
                .is(SymbolKind::Function)
                .has(Behavior::IsInstanceMethod(true)),
        );
    }
}
