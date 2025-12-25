# Execution Graph Implementation Plan

This document specifies the implementation of `kestrel-execution-graph`, Kestrel's Mid-level Intermediate Representation (MIR). The design is documented in [execution-graph.md](./execution-graph.md).

## Overview

The Execution Graph is a **print-only** IR. It is never parsed - it exists only as in-memory data structures that can be printed for debugging and analysis. The syntax in the design document describes the printed output format, not a source format.

## Crate Structure

```
lib/kestrel-execution-graph/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Re-exports, MirContext
    ├── id.rs                   # Id<T>, Arena<T, V>
    ├── metadata.rs             # Metadata, Prior<T>, Origin
    ├── qualified_name.rs       # QualifiedName
    ├── ty.rs                   # MirTy
    ├── item/
    │   ├── mod.rs
    │   ├── struct_def.rs
    │   ├── field.rs
    │   ├── enum_def.rs
    │   ├── enum_case.rs
    │   ├── protocol_def.rs
    │   ├── associated_type.rs
    │   ├── protocol_method.rs
    │   ├── witness_def.rs
    │   ├── function_def.rs
    │   ├── param.rs
    │   └── static_def.rs
    ├── function/
    │   ├── mod.rs
    │   ├── local.rs
    │   ├── type_param.rs
    │   ├── place.rs
    │   ├── immediate.rs
    │   ├── value.rs
    │   ├── statement.rs
    │   ├── terminator.rs
    │   └── basic_block.rs
    ├── builder/
    │   ├── mod.rs
    │   ├── function.rs
    │   └── block.rs
    └── pass.rs                 # MirPass trait, PassResult
```

## Cargo.toml

```toml
[package]
name = "kestrel-execution-graph"
version.workspace = true
edition.workspace = true

[dependencies]
kestrel-span = { path = "../kestrel-span" }
kestrel-reporting = { path = "../kestrel-reporting" }
downcast-rs.workspace = true
```

---

## Core Infrastructure

### Id and Arena (`src/id.rs`)

```rust
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A typed identifier referencing an item in an arena.
/// 
/// The phantom type `T` is a marker indicating what kind of item this ID references.
/// This provides compile-time safety: you can't accidentally use an `Id<Function>` 
/// where an `Id<Block>` is expected.
pub struct Id<T> {
    raw: u32,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub fn from_raw(raw: u32) -> Self {
        Self { raw, _phantom: PhantomData }
    }
    
    pub fn raw(self) -> u32 {
        self.raw
    }
}

// Manual trait impls to avoid requiring bounds on T

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.raw == other.raw }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.raw.hash(state) }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.raw)
    }
}

/// A typed arena that stores items and hands out `Id<T>` handles.
/// 
/// The type parameter `T` is the marker type for IDs, and `V` is the actual
/// stored value type. For example, `Arena<Function, FunctionDef>` stores
/// `FunctionDef` values and returns `Id<Function>` handles.
#[derive(Debug, Clone)]
pub struct Arena<T, V> {
    items: Vec<V>,
    _phantom: PhantomData<fn() -> T>,
}

impl<T, V> Arena<T, V> {
    pub fn new() -> Self {
        Self { items: Vec::new(), _phantom: PhantomData }
    }
    
    pub fn alloc(&mut self, item: V) -> Id<T> {
        let id = Id::from_raw(self.items.len() as u32);
        self.items.push(item);
        id
    }
    
    pub fn get(&self, id: Id<T>) -> &V {
        &self.items[id.raw() as usize]
    }
    
    pub fn get_mut(&mut self, id: Id<T>) -> &mut V {
        &mut self.items[id.raw() as usize]
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &V)> {
        self.items.iter().enumerate()
            .map(|(i, v)| (Id::from_raw(i as u32), v))
    }
    
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Id<T>, &mut V)> {
        self.items.iter_mut().enumerate()
            .map(|(i, v)| (Id::from_raw(i as u32), v))
    }
    
    pub fn ids(&self) -> impl Iterator<Item = Id<T>> {
        (0..self.items.len() as u32).map(Id::from_raw)
    }
    
    pub fn len(&self) -> usize {
        self.items.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T, V> Default for Arena<T, V> {
    fn default() -> Self { Self::new() }
}

impl<T, V> std::ops::Index<Id<T>> for Arena<T, V> {
    type Output = V;
    fn index(&self, id: Id<T>) -> &V { self.get(id) }
}

impl<T, V> std::ops::IndexMut<Id<T>> for Arena<T, V> {
    fn index_mut(&mut self, id: Id<T>) -> &mut V { self.get_mut(id) }
}

// === Marker types for Id<T> ===

pub struct Function;
pub struct Block;
pub struct Statement;
pub struct Local;
pub struct Param;
pub struct Struct;
pub struct Field;
pub struct Enum;
pub struct EnumCase;
pub struct Protocol;
pub struct AssociatedType;
pub struct ProtocolMethod;
pub struct Witness;
pub struct Static;
pub struct TypeParam;
pub struct Ty;
pub struct QualifiedName;
```

### Metadata (`src/metadata.rs`)

```rust
use crate::id::Id;
use crate::qualified_name::QualifiedNameData;
use kestrel_span::Span;
use std::sync::Arc;

/// Metadata attached to any MIR node.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Source span (preserved through lowering).
    pub span: Option<Span>,
    
    /// Where this item originated from (for cross-type provenance).
    pub origin: Option<Origin>,
    
    /// Debug comments (printed as `// comment`).
    pub comments: Vec<String>,
}

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_span(span: Span) -> Self {
        Self { span: Some(span), ..Self::default() }
    }
    
    pub fn set_span(&mut self, span: Span) -> &mut Self {
        self.span = Some(span);
        self
    }
    
    pub fn set_origin(&mut self, origin: Origin) -> &mut Self {
        self.origin = Some(origin);
        self
    }
    
    pub fn add_comment(&mut self, comment: impl Into<String>) -> &mut Self {
        self.comments.push(comment.into());
        self
    }
}

/// Records same-type transformation history.
/// 
/// When a pass transforms a node, the new node can store the original
/// via `Prior<T>`, enabling debugging and analysis of transformations.
#[derive(Debug, Clone)]
pub struct Prior<T> {
    /// Which pass made this transformation.
    pub pass_name: String,
    
    /// Description of what the transformation did.
    pub description: Option<String>,
    
    /// The original node before transformation.
    pub original: Arc<T>,
}

impl<T> Prior<T> {
    pub fn new(pass_name: impl Into<String>, original: T) -> Self {
        Self {
            pass_name: pass_name.into(),
            description: None,
            original: Arc::new(original),
        }
    }
    
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Cross-type provenance for items that originated from other constructs.
#[derive(Debug, Clone)]
pub enum Origin {
    /// Lowered directly from semantic tree.
    Source { span: Span },
    
    /// Generated as a closure's environment struct.
    ClosureEnv {
        containing_function: Id<crate::id::QualifiedName>,
        closure_span: Span,
    },
    
    /// Generated as a closure's call method.
    ClosureCall {
        env_struct: Id<crate::id::Struct>,
        closure_span: Span,
    },
    
    /// Synthesized by a pass.
    Synthesized {
        pass_name: String,
        reason: String,
    },
}
```

### Qualified Name (`src/qualified_name.rs`)

```rust
use crate::MirContext;
use std::fmt;

/// A fully-qualified name like `std.vec.Vec` or `example.main."closures".0`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedNameData {
    pub segments: Vec<String>,
}

impl QualifiedNameData {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }
    
    pub fn from_parts(parts: &[&str]) -> Self {
        Self { segments: parts.iter().map(|s| s.to_string()).collect() }
    }
    
    pub fn join(&self, segment: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment.into());
        Self { segments }
    }
    
    pub fn name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }
}

impl fmt::Display for QualifiedNameData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 { write!(f, ".")?; }
            write!(f, "{}", segment)?;
        }
        Ok(())
    }
}
```

---

## Type System (`src/ty.rs`)

```rust
use crate::id::{Id, TypeParam, Ty, QualifiedName};
use crate::MirContext;
use std::fmt;

/// MIR type representation.
/// 
/// Types are interned in `MirContext`. Use `Id<Ty>` to reference them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirTy {
    // === Primitives ===
    I8, I16, I32, I64,
    F16, F32, F64,
    Bool,
    Unit,
    Never,
    Str,
    
    // === Pointers and References ===
    Pointer(Id<Ty>),
    Ref(Id<Ty>),
    RefMut(Id<Ty>),
    
    // === Compound ===
    Tuple(Vec<Id<Ty>>),
    
    /// Named type (struct, enum, protocol).
    Named {
        name: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
    },
    
    /// Type parameter reference.
    TypeParam(Id<TypeParam>),
    
    // === Function types ===
    /// Thin function pointer (no environment, FFI-safe).
    FuncThin {
        params: Vec<Id<Ty>>,
        ret: Id<Ty>,
    },
    
    /// Thick callable (has environment, can escape).
    FuncThick {
        params: Vec<Id<Ty>>,
        ret: Id<Ty>,
    },
}

impl MirTy {
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        MirTyDisplay { ty: self, ctx }
    }
}

struct MirTyDisplay<'a> {
    ty: &'a MirTy,
    ctx: &'a MirContext,
}

impl fmt::Display for MirTyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            MirTy::I8 => write!(f, "i8"),
            MirTy::I16 => write!(f, "i16"),
            MirTy::I32 => write!(f, "i32"),
            MirTy::I64 => write!(f, "i64"),
            MirTy::F16 => write!(f, "f16"),
            MirTy::F32 => write!(f, "f32"),
            MirTy::F64 => write!(f, "f64"),
            MirTy::Bool => write!(f, "bool"),
            MirTy::Unit => write!(f, "()"),
            MirTy::Never => write!(f, "!"),
            MirTy::Str => write!(f, "str"),
            
            MirTy::Pointer(inner) => {
                write!(f, "p[{}]", self.ctx.ty(*inner).display(self.ctx))
            }
            MirTy::Ref(inner) => {
                write!(f, "&{}", self.ctx.ty(*inner).display(self.ctx))
            }
            MirTy::RefMut(inner) => {
                write!(f, "&var {}", self.ctx.ty(*inner).display(self.ctx))
            }
            
            MirTy::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", self.ctx.ty(*elem).display(self.ctx))?;
                }
                write!(f, ")")
            }
            
            MirTy::Named { name, type_args } => {
                write!(f, "{}", self.ctx.name(*name))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", self.ctx.ty(*arg).display(self.ctx))?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            
            MirTy::TypeParam(id) => {
                write!(f, "{}", self.ctx.type_param(*id).name)
            }
            
            MirTy::FuncThin { params, ret } => {
                write!(f, "func(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", self.ctx.ty(*p).display(self.ctx))?;
                }
                write!(f, ") -> {}", self.ctx.ty(*ret).display(self.ctx))
            }
            
            MirTy::FuncThick { params, ret } => {
                write!(f, "func escaping(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", self.ctx.ty(*p).display(self.ctx))?;
                }
                write!(f, ") -> {}", self.ctx.ty(*ret).display(self.ctx))
            }
        }
    }
}
```

---

## Item Definitions (`src/item/`)

### StructDef (`src/item/struct_def.rs`)

```rust
use crate::id::{Id, Struct, Field, TypeParam, QualifiedName};
use crate::metadata::{Metadata, Prior};
use crate::MirContext;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct StructDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<StructDef>>,
    pub name: Id<QualifiedName>,
    pub type_params: Vec<Id<TypeParam>>,
    pub fields: Vec<Id<Field>>,
    pub fields_by_name: HashMap<String, Id<Field>>,
}

impl StructDef {
    pub fn new(name: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name,
            type_params: Vec::new(),
            fields: Vec::new(),
            fields_by_name: HashMap::new(),
        }
    }
    
    pub fn add_field(&mut self, name: String, id: Id<Field>) {
        self.fields_by_name.insert(name, id);
        self.fields.push(id);
    }
    
    pub fn field_by_name(&self, name: &str) -> Option<Id<Field>> {
        self.fields_by_name.get(name).copied()
    }
    
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        StructDefDisplay { def: self, ctx }
    }
}

struct StructDefDisplay<'a> {
    def: &'a StructDef,
    ctx: &'a MirContext,
}

impl fmt::Display for StructDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "struct {}", self.ctx.name(self.def.name))?;
        
        if !self.def.type_params.is_empty() {
            write!(f, "[")?;
            for (i, tp) in self.def.type_params.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                write!(f, "{}", self.ctx.type_param(*tp).name)?;
            }
            write!(f, "]")?;
        }
        
        writeln!(f, " {{")?;
        
        for field_id in &self.def.fields {
            let field = &self.ctx.fields[*field_id];
            writeln!(f, "    {}: {}", field.name, self.ctx.ty(field.ty).display(self.ctx))?;
        }
        
        write!(f, "}}")
    }
}
```

### Field (`src/item/field.rs`)

```rust
use crate::id::{Id, Ty};
use crate::metadata::{Metadata, Prior};

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<FieldDef>>,
    pub name: String,
    pub ty: Id<Ty>,
}
```

### FunctionDef (`src/item/function_def.rs`)

```rust
use crate::id::{Id, Function, Block, Local, Param, TypeParam, QualifiedName, Ty};
use crate::metadata::{Metadata, Prior};
use crate::MirContext;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<FunctionDef>>,
    pub name: Id<QualifiedName>,
    pub type_params: Vec<Id<TypeParam>>,
    pub params: Vec<Id<Param>>,
    pub params_by_name: HashMap<String, Id<Param>>,
    pub ret: Id<Ty>,
    pub where_clause: Option<WhereClause>,
    pub locals: Vec<Id<Local>>,
    pub locals_by_name: HashMap<String, Id<Local>>,
    pub blocks: Vec<Id<Block>>,
    pub entry_block: Option<Id<Block>>,
}

impl FunctionDef {
    pub fn new(name: Id<QualifiedName>, ret: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name,
            type_params: Vec::new(),
            params: Vec::new(),
            params_by_name: HashMap::new(),
            ret,
            where_clause: None,
            locals: Vec::new(),
            locals_by_name: HashMap::new(),
            blocks: Vec::new(),
            entry_block: None,
        }
    }
    
    pub fn add_local(&mut self, name: String, id: Id<Local>) {
        self.locals_by_name.insert(name, id);
        self.locals.push(id);
    }
    
    pub fn local_by_name(&self, name: &str) -> Option<Id<Local>> {
        self.locals_by_name.get(name).copied()
    }
    
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        FunctionDefDisplay { def: self, ctx }
    }
}

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub constraints: Vec<WhereConstraint>,
}

#[derive(Debug, Clone)]
pub enum WhereConstraint {
    /// `T: Protocol`
    Implements {
        type_param: Id<TypeParam>,
        protocol: Id<QualifiedName>,
    },
    /// `T.Item = i64`
    TypeEquals {
        base: Id<TypeParam>,
        associated: String,
        equals: Id<Ty>,
    },
}

struct FunctionDefDisplay<'a> {
    def: &'a FunctionDef,
    ctx: &'a MirContext,
}

impl fmt::Display for FunctionDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // func name[T, U](params...) -> Ret
        write!(f, "func {}", self.ctx.name(self.def.name))?;
        
        if !self.def.type_params.is_empty() {
            write!(f, "[")?;
            for (i, tp) in self.def.type_params.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                write!(f, "{}", self.ctx.type_param(*tp).name)?;
            }
            write!(f, "]")?;
        }
        
        write!(f, "(")?;
        for (i, param_id) in self.def.params.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            let param = &self.ctx.params[*param_id];
            write!(f, "{}: {}", param.name, self.ctx.ty(param.ty).display(self.ctx))?;
        }
        write!(f, ") -> {}", self.ctx.ty(self.def.ret).display(self.ctx))?;
        
        // where clause
        if let Some(wc) = &self.def.where_clause {
            write!(f, "\nwhere ")?;
            for (i, constraint) in wc.constraints.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                match constraint {
                    WhereConstraint::Implements { type_param, protocol } => {
                        write!(f, "{}: {}", 
                            self.ctx.type_param(*type_param).name,
                            self.ctx.name(*protocol))?;
                    }
                    WhereConstraint::TypeEquals { base, associated, equals } => {
                        write!(f, "{}.{} = {}",
                            self.ctx.type_param(*base).name,
                            associated,
                            self.ctx.ty(*equals).display(self.ctx))?;
                    }
                }
            }
        }
        
        writeln!(f, "\n{{")?;
        
        // locals
        if !self.def.locals.is_empty() {
            writeln!(f, "    locals:")?;
            for local_id in &self.def.locals {
                let local = &self.ctx.locals[*local_id];
                writeln!(f, "        %{}: {}", local.name, self.ctx.ty(local.ty).display(self.ctx))?;
            }
            writeln!(f)?;
        }
        
        // blocks
        for (i, block_id) in self.def.blocks.iter().enumerate() {
            let block = &self.ctx.blocks[*block_id];
            write!(f, "    bb{}:\n{}", i, block.display(self.ctx, "        "))?;
            if i < self.def.blocks.len() - 1 {
                writeln!(f)?;
            }
        }
        
        write!(f, "}}")
    }
}
```

### Param (`src/item/param.rs`)

```rust
use crate::id::{Id, Local, Ty};
use crate::metadata::{Metadata, Prior};

#[derive(Debug, Clone)]
pub struct ParamDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<ParamDef>>,
    pub name: String,
    pub local: Id<Local>,
    pub ty: Id<Ty>,
}
```

---

## Function Internals (`src/function/`)

### TypeParam (`src/function/type_param.rs`)

```rust
use crate::id::{Id, Function, Struct, Enum, Protocol};
use crate::metadata::{Metadata, Prior};

#[derive(Debug, Clone)]
pub struct TypeParamDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<TypeParamDef>>,
    pub name: String,
    pub owner: TypeParamOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeParamOwner {
    Function(Id<Function>),
    Struct(Id<Struct>),
    Enum(Id<Enum>),
    Protocol(Id<Protocol>),
}
```

### Local (`src/function/local.rs`)

```rust
use crate::id::{Id, Ty};
use crate::metadata::{Metadata, Prior};

#[derive(Debug, Clone)]
pub struct LocalDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<LocalDef>>,
    pub name: String,
    pub ty: Id<Ty>,
}
```

### Place (`src/function/place.rs`)

```rust
use crate::id::{Id, Local};
use crate::metadata::Metadata;
use crate::MirContext;
use std::fmt;

/// A place is a memory location that can be read, written, or referenced.
#[derive(Debug, Clone)]
pub struct Place {
    pub meta: Metadata,
    pub inline_name: Option<String>,
    pub kind: PlaceKind,
}

#[derive(Debug, Clone)]
pub enum PlaceKind {
    /// A local variable: `%x`
    Local(Id<Local>),
    
    /// Field access: `<place>.field`
    Field {
        parent: Box<Place>,
        name: String,
    },
    
    /// Tuple index: `<place>.0`
    Index {
        parent: Box<Place>,
        index: usize,
    },
    
    /// Enum downcast: `<place>.SomeCase` (valid after switch)
    Downcast {
        parent: Box<Place>,
        variant: String,
    },
    
    /// Dereference: `deref <place>`
    Deref(Box<Place>),
}

impl Place {
    pub fn local(id: Id<Local>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Local(id),
        }
    }
    
    pub fn field(self, name: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Field {
                parent: Box::new(self),
                name: name.into(),
            },
        }
    }
    
    pub fn index(self, index: usize) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Index {
                parent: Box::new(self),
                index,
            },
        }
    }
    
    pub fn downcast(self, variant: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Downcast {
                parent: Box::new(self),
                variant: variant.into(),
            },
        }
    }
    
    pub fn deref(self) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Deref(Box::new(self)),
        }
    }
    
    pub fn with_inline_name(mut self, name: impl Into<String>) -> Self {
        self.inline_name = Some(name.into());
        self
    }
    
    /// Get the root local of this place.
    pub fn root_local(&self) -> Id<Local> {
        match &self.kind {
            PlaceKind::Local(id) => *id,
            PlaceKind::Field { parent, .. } => parent.root_local(),
            PlaceKind::Index { parent, .. } => parent.root_local(),
            PlaceKind::Downcast { parent, .. } => parent.root_local(),
            PlaceKind::Deref(parent) => parent.root_local(),
        }
    }
    
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        PlaceDisplay { place: self, ctx }
    }
}

struct PlaceDisplay<'a> {
    place: &'a Place,
    ctx: &'a MirContext,
}

impl fmt::Display for PlaceDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.place.kind {
            PlaceKind::Local(id) => {
                write!(f, "%{}", self.ctx.local(*id).name)
            }
            PlaceKind::Field { parent, name } => {
                write!(f, "{}.{}", parent.display(self.ctx), name)
            }
            PlaceKind::Index { parent, index } => {
                write!(f, "{}.{}", parent.display(self.ctx), index)
            }
            PlaceKind::Downcast { parent, variant } => {
                write!(f, "{}.{}", parent.display(self.ctx), variant)
            }
            PlaceKind::Deref(inner) => {
                write!(f, "(deref {})", inner.display(self.ctx))
            }
        }
    }
}
```

### Immediate (`src/function/immediate.rs`)

```rust
use crate::id::{Id, QualifiedName, Ty};
use crate::metadata::Metadata;
use crate::MirContext;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Immediate {
    pub meta: Metadata,
    pub inline_name: Option<String>,
    pub kind: ImmediateKind,
}

#[derive(Debug, Clone)]
pub enum ImmediateKind {
    // === Literals ===
    IntLiteral { bits: IntBits, value: i128 },
    FloatLiteral { bits: FloatBits, value: f64 },
    BoolLiteral(bool),
    StringLiteral(String),
    Unit,
    
    // === Function references ===
    FunctionRef {
        name: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
    },
    
    // === Witness method lookup ===
    WitnessMethod {
        protocol: Id<QualifiedName>,
        method: String,
        for_type: Id<Ty>,
    },
    
    // === Null pointer ===
    NullPtr(Id<Ty>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBits { I8, I16, I32, I64 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatBits { F16, F32, F64 }

impl Immediate {
    pub fn i8(value: i8) -> Self {
        Self::int(IntBits::I8, value as i128)
    }
    
    pub fn i16(value: i16) -> Self {
        Self::int(IntBits::I16, value as i128)
    }
    
    pub fn i32(value: i32) -> Self {
        Self::int(IntBits::I32, value as i128)
    }
    
    pub fn i64(value: i64) -> Self {
        Self::int(IntBits::I64, value as i128)
    }
    
    pub fn int(bits: IntBits, value: i128) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::IntLiteral { bits, value },
        }
    }
    
    pub fn f32(value: f32) -> Self {
        Self::float(FloatBits::F32, value as f64)
    }
    
    pub fn f64(value: f64) -> Self {
        Self::float(FloatBits::F64, value)
    }
    
    pub fn float(bits: FloatBits, value: f64) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::FloatLiteral { bits, value },
        }
    }
    
    pub fn bool(value: bool) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::BoolLiteral(value),
        }
    }
    
    pub fn string(value: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::StringLiteral(value.into()),
        }
    }
    
    pub fn unit() -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::Unit,
        }
    }
    
    pub fn function_ref(name: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: ImmediateKind::FunctionRef { name, type_args: Vec::new() },
        }
    }
    
    pub fn with_inline_name(mut self, name: impl Into<String>) -> Self {
        self.inline_name = Some(name.into());
        self
    }
    
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        ImmediateDisplay { imm: self, ctx }
    }
}

struct ImmediateDisplay<'a> {
    imm: &'a Immediate,
    ctx: &'a MirContext,
}

impl fmt::Display for ImmediateDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.imm.kind {
            ImmediateKind::IntLiteral { bits, value } => {
                let prefix = match bits {
                    IntBits::I8 => "i8",
                    IntBits::I16 => "i16",
                    IntBits::I32 => "i32",
                    IntBits::I64 => "i64",
                };
                write!(f, "{}.literal {}", prefix, value)
            }
            ImmediateKind::FloatLiteral { bits, value } => {
                let prefix = match bits {
                    FloatBits::F16 => "f16",
                    FloatBits::F32 => "f32",
                    FloatBits::F64 => "f64",
                };
                write!(f, "{}.literal {}", prefix, value)
            }
            ImmediateKind::BoolLiteral(b) => write!(f, "{}", b),
            ImmediateKind::StringLiteral(s) => write!(f, "str.literal {:?}", s),
            ImmediateKind::Unit => write!(f, "()"),
            ImmediateKind::FunctionRef { name, type_args } => {
                write!(f, "{}", self.ctx.name(*name))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", self.ctx.ty(*arg).display(self.ctx))?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            ImmediateKind::WitnessMethod { protocol, method, for_type } => {
                write!(f, "witness_method {}.{} for {}",
                    self.ctx.name(*protocol),
                    method,
                    self.ctx.ty(*for_type).display(self.ctx))
            }
            ImmediateKind::NullPtr(ty) => {
                write!(f, "ptr.null[{}]", self.ctx.ty(*ty).display(self.ctx))
            }
        }
    }
}
```

### Value (`src/function/value.rs`)

```rust
use crate::function::{Place, Immediate};
use crate::MirContext;
use std::fmt;

/// A value is either a place or an immediate.
#[derive(Debug, Clone)]
pub enum Value {
    Place(Place),
    Immediate(Immediate),
}

impl Value {
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        ValueDisplay { value: self, ctx }
    }
}

impl From<Place> for Value {
    fn from(p: Place) -> Self { Value::Place(p) }
}

impl From<Immediate> for Value {
    fn from(i: Immediate) -> Self { Value::Immediate(i) }
}

struct ValueDisplay<'a> {
    value: &'a Value,
    ctx: &'a MirContext,
}

impl fmt::Display for ValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value {
            Value::Place(p) => write!(f, "{}", p.display(self.ctx)),
            Value::Immediate(i) => write!(f, "{}", i.display(self.ctx)),
        }
    }
}
```

### Statement (`src/function/statement.rs`)

```rust
use crate::id::{Id, QualifiedName, Ty};
use crate::function::{Place, Value};
use crate::metadata::{Metadata, Prior};
use crate::MirContext;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Statement {
    pub meta: Metadata,
    pub priors: Vec<Prior<Statement>>,
    pub kind: StatementKind,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    /// `<place> = <rvalue>`
    Assign { dest: Place, rvalue: Rvalue },
    
    /// `call func(args...)` (unit return, no assignment)
    Call { callee: Callee, args: Vec<Value> },
}

/// The right-hand side of an assignment.
#[derive(Debug, Clone)]
pub enum Rvalue {
    /// `move <place>`
    Move(Place),
    
    /// `copy <place>`
    Copy(Place),
    
    /// `ref <place>`
    Ref(Place),
    
    /// `ref var <place>`
    RefMut(Place),
    
    /// `<immediate>`
    Use(crate::function::Immediate),
    
    /// Binary operation
    BinaryOp { op: BinOp, lhs: Value, rhs: Value },
    
    /// Unary operation
    UnaryOp { op: UnOp, operand: Value },
    
    /// `construct Type { field: value, ... }`
    Construct {
        ty: Id<Ty>,
        fields: Vec<(String, Value)>,
    },
    
    /// `call func(args...)` with return value
    Call { callee: Callee, args: Vec<Value> },
    
    /// Type cast
    Cast { kind: CastKind, operand: Value, target: Id<Ty> },
    
    // === String operations ===
    StrPtr(Value),
    StrLen(Value),
    StrFromParts { ptr: Value, len: Value },
    
    // === Pointer operations ===
    PtrOffset { ptr: Value, offset: Value },
    PtrToRef(Value),
    PtrToRefMut(Value),
    RefToPtr(Value),
    
    // === Callable operations ===
    FuncToEscaping(Id<QualifiedName>),
    ApplyPartial { func: Id<QualifiedName>, captures: Vec<Value> },
}

/// What's being called.
#[derive(Debug, Clone)]
pub enum Callee {
    /// Direct call: `call path.to.func(...)`
    Direct {
        name: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
    },
    
    /// Thin function pointer: `call %fn_ptr(...)`
    Thin(Place),
    
    /// Thick callable: `call escaping %closure(...)`
    Thick(Place),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Integer (signed)
    AddSigned, SubSigned, MulSigned, DivSigned, RemSigned,
    // Integer (unsigned)
    AddUnsigned, SubUnsigned, MulUnsigned, DivUnsigned, RemUnsigned,
    // Float
    FAdd, FSub, FMul, FDiv,
    // Bitwise
    And, Or, Xor, Shl, ShrSigned, ShrUnsigned,
    // Comparison (integer)
    Eq, Ne, LtSigned, LeSigned, GtSigned, GeSigned,
    LtUnsigned, LeUnsigned, GtUnsigned, GeUnsigned,
    // Comparison (float)
    FEq, FNe, FLt, FLe, FGt, FGe,
    // Boolean
    BoolAnd, BoolOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    FNeg,
    Not,
    BoolNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    IntToFloat,
    FloatToInt,
    IntWiden,
    IntTruncate,
    FloatWiden,
    FloatTruncate,
    PtrBitcast,
    RefToImmut,
}

impl Statement {
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        StatementDisplay { stmt: self, ctx }
    }
}

struct StatementDisplay<'a> {
    stmt: &'a Statement,
    ctx: &'a MirContext,
}

impl fmt::Display for StatementDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.stmt.kind {
            StatementKind::Assign { dest, rvalue } => {
                write!(f, "{} = {}", 
                    dest.display(self.ctx),
                    rvalue.display(self.ctx))
            }
            StatementKind::Call { callee, args } => {
                write!(f, "call {}", callee.display(self.ctx))?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg.display(self.ctx))?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Rvalue {
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        RvalueDisplay { rvalue: self, ctx }
    }
}

struct RvalueDisplay<'a> {
    rvalue: &'a Rvalue,
    ctx: &'a MirContext,
}

impl fmt::Display for RvalueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.rvalue {
            Rvalue::Move(p) => write!(f, "move {}", p.display(self.ctx)),
            Rvalue::Copy(p) => write!(f, "copy {}", p.display(self.ctx)),
            Rvalue::Ref(p) => write!(f, "ref {}", p.display(self.ctx)),
            Rvalue::RefMut(p) => write!(f, "ref var {}", p.display(self.ctx)),
            Rvalue::Use(i) => write!(f, "{}", i.display(self.ctx)),
            Rvalue::BinaryOp { op, lhs, rhs } => {
                write!(f, "{} {}, {}", op.as_str(), lhs.display(self.ctx), rhs.display(self.ctx))
            }
            Rvalue::UnaryOp { op, operand } => {
                write!(f, "{} {}", op.as_str(), operand.display(self.ctx))
            }
            Rvalue::Construct { ty, fields } => {
                write!(f, "construct {} {{ ", self.ctx.ty(*ty).display(self.ctx))?;
                for (i, (name, value)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", name, value.display(self.ctx))?;
                }
                write!(f, " }}")
            }
            Rvalue::Call { callee, args } => {
                write!(f, "call {}", callee.display(self.ctx))?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg.display(self.ctx))?;
                }
                write!(f, ")")
            }
            // ... other cases
            _ => write!(f, "<rvalue>"),
        }
    }
}

impl Callee {
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        CalleeDisplay { callee: self, ctx }
    }
}

struct CalleeDisplay<'a> {
    callee: &'a Callee,
    ctx: &'a MirContext,
}

impl fmt::Display for CalleeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.callee {
            Callee::Direct { name, type_args } => {
                write!(f, "{}", self.ctx.name(*name))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", self.ctx.ty(*arg).display(self.ctx))?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            Callee::Thin(p) => write!(f, "{}", p.display(self.ctx)),
            Callee::Thick(p) => write!(f, "escaping {}", p.display(self.ctx)),
        }
    }
}

impl BinOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinOp::AddSigned => "i64.add.signed",
            BinOp::SubSigned => "i64.sub.signed",
            BinOp::MulSigned => "i64.mul.signed",
            BinOp::DivSigned => "i64.div.signed",
            BinOp::RemSigned => "i64.rem.signed",
            BinOp::FAdd => "f64.add",
            BinOp::FSub => "f64.sub",
            BinOp::FMul => "f64.mul",
            BinOp::FDiv => "f64.div",
            BinOp::Eq => "i64.eq",
            BinOp::Ne => "i64.ne",
            BinOp::LtSigned => "i64.lt.signed",
            BinOp::LeSigned => "i64.le.signed",
            BinOp::GtSigned => "i64.gt.signed",
            BinOp::GeSigned => "i64.ge.signed",
            BinOp::BoolAnd => "bool.and",
            BinOp::BoolOr => "bool.or",
            // ... etc
            _ => "<binop>",
        }
    }
}

impl UnOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnOp::Neg => "i64.neg",
            UnOp::FNeg => "f64.neg",
            UnOp::Not => "i64.not",
            UnOp::BoolNot => "bool.not",
        }
    }
}
```

### Terminator (`src/function/terminator.rs`)

```rust
use crate::id::{Id, Block};
use crate::function::{Place, Value};
use crate::metadata::Metadata;
use crate::MirContext;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Terminator {
    pub meta: Metadata,
    pub kind: TerminatorKind,
}

#[derive(Debug, Clone)]
pub enum TerminatorKind {
    /// `return <value>`
    Return(Value),
    
    /// `jump bb`
    Jump(Id<Block>),
    
    /// `branch if <cond>, bb_true else bb_false`
    Branch {
        condition: Value,
        then_block: Id<Block>,
        else_block: Id<Block>,
    },
    
    /// `switch <place> { Case => bb, ... }`
    Switch {
        discriminant: Place,
        cases: Vec<(String, Id<Block>)>,
    },
    
    /// `panic "message"`
    Panic(String),
    
    /// `unreachable`
    Unreachable,
}

impl Terminator {
    pub fn display<'a>(&'a self, ctx: &'a MirContext, blocks: &'a [Id<Block>]) -> impl fmt::Display + 'a {
        TerminatorDisplay { term: self, ctx, blocks }
    }
}

struct TerminatorDisplay<'a> {
    term: &'a Terminator,
    ctx: &'a MirContext,
    blocks: &'a [Id<Block>],
}

impl fmt::Display for TerminatorDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let block_index = |id: Id<Block>| -> usize {
            self.blocks.iter().position(|&b| b == id).unwrap_or(0)
        };
        
        match &self.term.kind {
            TerminatorKind::Return(v) => {
                write!(f, "return {}", v.display(self.ctx))
            }
            TerminatorKind::Jump(target) => {
                write!(f, "jump bb{}", block_index(*target))
            }
            TerminatorKind::Branch { condition, then_block, else_block } => {
                write!(f, "branch if {}, bb{} else bb{}",
                    condition.display(self.ctx),
                    block_index(*then_block),
                    block_index(*else_block))
            }
            TerminatorKind::Switch { discriminant, cases } => {
                writeln!(f, "switch {} {{", discriminant.display(self.ctx))?;
                for (case_name, target) in cases {
                    writeln!(f, "    {} => bb{}", case_name, block_index(*target))?;
                }
                write!(f, "}}")
            }
            TerminatorKind::Panic(msg) => {
                write!(f, "panic {:?}", msg)
            }
            TerminatorKind::Unreachable => {
                write!(f, "unreachable")
            }
        }
    }
}
```

### BasicBlock (`src/function/basic_block.rs`)

```rust
use crate::id::{Id, Statement, Block};
use crate::function::Terminator;
use crate::metadata::{Metadata, Prior};
use crate::MirContext;
use std::fmt;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub meta: Metadata,
    pub priors: Vec<Prior<BasicBlock>>,
    pub statements: Vec<Id<Statement>>,
    pub terminator: Option<Terminator>,
}

impl BasicBlock {
    pub fn new() -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            statements: Vec::new(),
            terminator: None,
        }
    }
    
    pub fn display<'a>(&'a self, ctx: &'a MirContext, indent: &'a str) -> impl fmt::Display + 'a {
        BasicBlockDisplay { block: self, ctx, indent }
    }
}

impl Default for BasicBlock {
    fn default() -> Self { Self::new() }
}

struct BasicBlockDisplay<'a> {
    block: &'a BasicBlock,
    ctx: &'a MirContext,
    indent: &'a str,
}

impl fmt::Display for BasicBlockDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stmt_id in &self.block.statements {
            let stmt = &self.ctx.statements[*stmt_id];
            writeln!(f, "{}{}", self.indent, stmt.display(self.ctx))?;
        }
        
        if let Some(term) = &self.block.terminator {
            // Note: block list not available here, would need adjustment
            writeln!(f, "{}{:?}", self.indent, term.kind)?;
        }
        
        Ok(())
    }
}
```

---

## MirContext (`src/lib.rs`)

```rust
pub mod id;
pub mod metadata;
pub mod qualified_name;
pub mod ty;
pub mod item;
pub mod function;
pub mod builder;
pub mod pass;

pub use id::*;
pub use metadata::*;
pub use qualified_name::*;
pub use ty::*;
pub use item::*;
pub use function::*;
pub use builder::*;
pub use pass::*;

use std::collections::HashMap;
use std::fmt;

/// The central context holding all MIR data.
#[derive(Debug, Clone, Default)]
pub struct MirContext {
    // === Top-level items ===
    pub structs: Arena<Struct, StructDef>,
    pub enums: Arena<Enum, EnumDef>,
    pub protocols: Arena<Protocol, ProtocolDef>,
    pub witnesses: Arena<Witness, WitnessDef>,
    pub functions: Arena<Function, FunctionDef>,
    pub statics: Arena<Static, StaticDef>,
    
    // === Struct/enum children ===
    pub fields: Arena<Field, FieldDef>,
    pub enum_cases: Arena<EnumCase, EnumCaseDef>,
    
    // === Protocol children ===
    pub associated_types: Arena<AssociatedType, AssociatedTypeDef>,
    pub protocol_methods: Arena<ProtocolMethod, ProtocolMethodDef>,
    
    // === Function children ===
    pub blocks: Arena<Block, BasicBlock>,
    pub statements: Arena<Statement, item::Statement>,
    pub locals: Arena<Local, LocalDef>,
    pub params: Arena<Param, ParamDef>,
    
    // === Type system ===
    pub type_params: Arena<TypeParam, TypeParamDef>,
    types: Arena<Ty, MirTy>,
    type_lookup: HashMap<MirTy, Id<Ty>>,
    
    // === Names ===
    names: Arena<QualifiedName, QualifiedNameData>,
    name_lookup: HashMap<QualifiedNameData, Id<QualifiedName>>,
}

impl MirContext {
    pub fn new() -> Self {
        Self::default()
    }
    
    // === Type interning ===
    
    pub fn intern_type(&mut self, ty: MirTy) -> Id<Ty> {
        if let Some(&id) = self.type_lookup.get(&ty) {
            return id;
        }
        let id = self.types.alloc(ty.clone());
        self.type_lookup.insert(ty, id);
        id
    }
    
    pub fn ty(&self, id: Id<Ty>) -> &MirTy {
        &self.types[id]
    }
    
    // === Name interning ===
    
    pub fn intern_name(&mut self, name: QualifiedNameData) -> Id<QualifiedName> {
        if let Some(&id) = self.name_lookup.get(&name) {
            return id;
        }
        let id = self.names.alloc(name.clone());
        self.name_lookup.insert(name, id);
        id
    }
    
    pub fn name(&self, id: Id<QualifiedName>) -> &QualifiedNameData {
        &self.names[id]
    }
    
    // === Convenience accessors ===
    
    pub fn function(&self, id: Id<Function>) -> &FunctionDef {
        &self.functions[id]
    }
    
    pub fn function_mut(&mut self, id: Id<Function>) -> &mut FunctionDef {
        &mut self.functions[id]
    }
    
    pub fn block(&self, id: Id<Block>) -> &BasicBlock {
        &self.blocks[id]
    }
    
    pub fn block_mut(&mut self, id: Id<Block>) -> &mut BasicBlock {
        &mut self.blocks[id]
    }
    
    pub fn statement(&self, id: Id<Statement>) -> &item::Statement {
        &self.statements[id]
    }
    
    pub fn statement_mut(&mut self, id: Id<Statement>) -> &mut item::Statement {
        &mut self.statements[id]
    }
    
    pub fn local(&self, id: Id<Local>) -> &LocalDef {
        &self.locals[id]
    }
    
    pub fn type_param(&self, id: Id<TypeParam>) -> &TypeParamDef {
        &self.type_params[id]
    }
    
    // === Builders ===
    
    pub fn add_function(&mut self, name: Id<QualifiedName>, ret: Id<Ty>) -> FunctionBuilder<'_> {
        let def = FunctionDef::new(name, ret);
        let id = self.functions.alloc(def);
        FunctionBuilder { ctx: self, id }
    }
    
    pub fn function_builder(&mut self, id: Id<Function>) -> FunctionBuilder<'_> {
        FunctionBuilder { ctx: self, id }
    }
    
    // === Display ===
    
    pub fn display(&self) -> impl fmt::Display + '_ {
        MirContextDisplay { ctx: self }
    }
}

struct MirContextDisplay<'a> {
    ctx: &'a MirContext,
}

impl fmt::Display for MirContextDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (_, def) in self.ctx.structs.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }
        
        for (_, def) in self.ctx.functions.iter() {
            writeln!(f, "{}\n", def.display(self.ctx))?;
        }
        
        Ok(())
    }
}
```

---

## Builders (`src/builder/`)

### FunctionBuilder (`src/builder/function.rs`)

```rust
use crate::*;

pub struct FunctionBuilder<'ctx> {
    pub(crate) ctx: &'ctx mut MirContext,
    pub(crate) id: Id<Function>,
}

impl<'ctx> FunctionBuilder<'ctx> {
    pub fn id(&self) -> Id<Function> {
        self.id
    }
    
    pub fn def(&self) -> &FunctionDef {
        &self.ctx.functions[self.id]
    }
    
    pub fn def_mut(&mut self) -> &mut FunctionDef {
        &mut self.ctx.functions[self.id]
    }
    
    pub fn type_param(&mut self, name: impl Into<String>) -> Id<TypeParam> {
        let tp = TypeParamDef {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.into(),
            owner: TypeParamOwner::Function(self.id),
        };
        let tp_id = self.ctx.type_params.alloc(tp);
        self.def_mut().type_params.push(tp_id);
        tp_id
    }
    
    pub fn param(&mut self, name: impl Into<String>, ty: Id<Ty>) -> Id<Local> {
        let name = name.into();
        
        let local = LocalDef {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.clone(),
            ty,
        };
        let local_id = self.ctx.locals.alloc(local);
        
        let param = ParamDef {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.clone(),
            local: local_id,
            ty,
        };
        let param_id = self.ctx.params.alloc(param);
        
        let def = self.def_mut();
        def.params.push(param_id);
        def.params_by_name.insert(name.clone(), param_id);
        def.locals.push(local_id);
        def.locals_by_name.insert(name, local_id);
        
        local_id
    }
    
    pub fn local(&mut self, name: impl Into<String>, ty: Id<Ty>) -> Id<Local> {
        let name = name.into();
        
        let local = LocalDef {
            meta: Metadata::new(),
            priors: Vec::new(),
            name: name.clone(),
            ty,
        };
        let local_id = self.ctx.locals.alloc(local);
        
        let def = self.def_mut();
        def.locals.push(local_id);
        def.locals_by_name.insert(name, local_id);
        
        local_id
    }
    
    pub fn add_block(&mut self) -> BlockBuilder<'_> {
        let block = BasicBlock::new();
        let block_id = self.ctx.blocks.alloc(block);
        
        let def = self.def_mut();
        if def.entry_block.is_none() {
            def.entry_block = Some(block_id);
        }
        def.blocks.push(block_id);
        
        BlockBuilder { ctx: self.ctx, id: block_id }
    }
}
```

### BlockBuilder (`src/builder/block.rs`)

```rust
use crate::*;

pub struct BlockBuilder<'ctx> {
    pub(crate) ctx: &'ctx mut MirContext,
    pub(crate) id: Id<Block>,
}

impl<'ctx> BlockBuilder<'ctx> {
    pub fn id(&self) -> Id<Block> {
        self.id
    }
    
    pub fn block(&self) -> &BasicBlock {
        &self.ctx.blocks[self.id]
    }
    
    pub fn block_mut(&mut self) -> &mut BasicBlock {
        &mut self.ctx.blocks[self.id]
    }
    
    pub fn add_statement(&mut self, kind: StatementKind) -> Id<Statement> {
        let stmt = Statement {
            meta: Metadata::new(),
            priors: Vec::new(),
            kind,
        };
        let stmt_id = self.ctx.statements.alloc(stmt);
        self.block_mut().statements.push(stmt_id);
        stmt_id
    }
    
    pub fn assign(&mut self, dest: Place, rvalue: Rvalue) -> Id<Statement> {
        self.add_statement(StatementKind::Assign { dest, rvalue })
    }
    
    pub fn call(&mut self, callee: Callee, args: Vec<Value>) -> Id<Statement> {
        self.add_statement(StatementKind::Call { callee, args })
    }
    
    // === Terminators ===
    
    pub fn terminate(&mut self, kind: TerminatorKind) {
        self.block_mut().terminator = Some(Terminator {
            meta: Metadata::new(),
            kind,
        });
    }
    
    pub fn ret(&mut self, value: impl Into<Value>) {
        self.terminate(TerminatorKind::Return(value.into()));
    }
    
    pub fn ret_unit(&mut self) {
        self.ret(Immediate::unit());
    }
    
    pub fn jump(&mut self, target: Id<Block>) {
        self.terminate(TerminatorKind::Jump(target));
    }
    
    pub fn branch(&mut self, cond: impl Into<Value>, then_block: Id<Block>, else_block: Id<Block>) {
        self.terminate(TerminatorKind::Branch {
            condition: cond.into(),
            then_block,
            else_block,
        });
    }
    
    pub fn switch(&mut self, discriminant: Place, cases: Vec<(String, Id<Block>)>) {
        self.terminate(TerminatorKind::Switch { discriminant, cases });
    }
    
    pub fn panic(&mut self, message: impl Into<String>) {
        self.terminate(TerminatorKind::Panic(message.into()));
    }
    
    pub fn unreachable(&mut self) {
        self.terminate(TerminatorKind::Unreachable);
    }
}
```

---

## Pass System (`src/pass.rs`)

```rust
use crate::{MirContext, Id, Function};
use kestrel_reporting::Diagnostic;

/// Result of running a pass.
#[derive(Debug, Default)]
pub struct PassResult {
    /// Diagnostics (errors, warnings) produced by the pass.
    pub diagnostics: Vec<Diagnostic>,
    
    /// Whether the pass made any modifications.
    pub modified: bool,
}

impl PassResult {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_diagnostic(mut self, diag: Diagnostic) -> Self {
        self.diagnostics.push(diag);
        self
    }
    
    pub fn set_modified(&mut self) {
        self.modified = true;
    }
}

/// A transformation pass over the entire MIR.
pub trait MirPass {
    /// Human-readable name (used in Prior records).
    fn name(&self) -> &'static str;
    
    /// Run the pass, potentially modifying the context.
    fn run(&mut self, ctx: &mut MirContext) -> PassResult;
}

/// A pass that operates on individual functions.
pub trait FunctionPass {
    /// Human-readable name.
    fn name(&self) -> &'static str;
    
    /// Run the pass on a single function.
    fn run_on_function(&mut self, ctx: &mut MirContext, func: Id<Function>) -> PassResult;
}

/// Adapter to run a FunctionPass as a MirPass.
impl<T: FunctionPass> MirPass for T {
    fn name(&self) -> &'static str {
        FunctionPass::name(self)
    }
    
    fn run(&mut self, ctx: &mut MirContext) -> PassResult {
        let mut result = PassResult::new();
        let func_ids: Vec<_> = ctx.functions.ids().collect();
        
        for func_id in func_ids {
            let func_result = self.run_on_function(ctx, func_id);
            result.diagnostics.extend(func_result.diagnostics);
            result.modified |= func_result.modified;
        }
        
        result
    }
}

/// Manages and runs a sequence of passes.
pub struct PassManager {
    passes: Vec<Box<dyn MirPass>>,
}

impl PassManager {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }
    
    pub fn add<P: MirPass + 'static>(&mut self, pass: P) -> &mut Self {
        self.passes.push(Box::new(pass));
        self
    }
    
    pub fn run(&mut self, ctx: &mut MirContext) -> Vec<Diagnostic> {
        let mut all_diagnostics = Vec::new();
        
        for pass in &mut self.passes {
            let result = pass.run(ctx);
            all_diagnostics.extend(result.diagnostics);
        }
        
        all_diagnostics
    }
}

impl Default for PassManager {
    fn default() -> Self { Self::new() }
}
```

---

## Example Usage

```rust
use kestrel_execution_graph::*;

fn main() {
    let mut ctx = MirContext::new();
    
    // Intern types
    let i64_ty = ctx.intern_type(MirTy::I64);
    let unit_ty = ctx.intern_type(MirTy::Unit);
    
    // Intern name
    let name = ctx.intern_name(QualifiedNameData::from_parts(&["example", "add"]));
    
    // Build function
    let mut func = ctx.add_function(name, i64_ty);
    
    let x = func.param("x", i64_ty);
    let y = func.param("y", i64_ty);
    let result = func.local("result", i64_ty);
    
    let mut bb = func.add_block();
    
    // %result = i64.add.signed %x, %y
    bb.assign(
        Place::local(result),
        Rvalue::BinaryOp {
            op: BinOp::AddSigned,
            lhs: Place::local(x).into(),
            rhs: Place::local(y).into(),
        },
    );
    
    // return %result
    bb.ret(Place::local(result));
    
    // Print
    println!("{}", ctx.display());
}
```

Output:
```
func example.add(x: i64, y: i64) -> i64
{
    locals:
        %x: i64
        %y: i64
        %result: i64

    bb0:
        %result = i64.add.signed %x, %y
        return %result
}
```
