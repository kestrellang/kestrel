//! Function internals: locals, places, values, statements, terminators, blocks.

mod basic_block;
mod immediate;
mod local;
mod place;
mod statement;
mod terminator;
mod type_param;
mod value;

pub use basic_block::*;
pub use immediate::*;
pub use local::*;
pub use place::*;
pub use statement::*;
pub use terminator::*;
pub use type_param::*;
pub use value::*;
