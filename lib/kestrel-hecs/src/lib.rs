//! kestrel-hecs: Hierarchical Entity Component System with incremental queries.
//!
//! A standalone ECS designed for incremental compilation. Provides:
//! - **Entities** as lightweight handles into the world
//! - **Components** stored in typed columns (struct-of-arrays)
//! - **Queries** with automatic dependency tracking and memoization
//! - **Change detection** via fingerprinting with early cutoff (backdating)
//! - **Accumulators** for side-effect values (diagnostics, warnings, etc.)
//!
//! # Usage
//!
//! ```
//! use kestrel_hecs::{World, QueryFn, QueryContext};
//!
//! // Define components (any Clone + 'static type)
//! #[derive(Clone)]
//! struct Name(String);
//!
//! // Define queries
//! #[derive(Clone, PartialEq, Eq, Hash)]
//! struct GetName { entity: kestrel_hecs::Entity }
//!
//! impl QueryFn for GetName {
//!     type Output = Option<String>;
//!     fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
//!         ctx.get::<Name>(self.entity).map(|n| n.0.clone())
//!     }
//! }
//!
//! // Use the world
//! let mut world = World::new();
//! world.begin_revision();
//! let e = world.spawn();
//! world.set(e, Name("Alice".into()));
//!
//! let ctx = world.query_context();
//! assert_eq!(ctx.query(GetName { entity: e }), Some("Alice".into()));
//! ```

pub mod accumulator;
pub mod change;
pub mod component;
pub mod entity;
pub mod fingerprint;
pub mod query;
pub mod world;

// Re-export primary types at crate root
pub use entity::Entity;
pub use fingerprint::Fingerprint;
pub use query::{QueryContext, QueryFn, QueryKey};
pub use world::{Revision, World};
