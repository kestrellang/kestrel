//! Item lowering - converts semantic tree items to MIR.

mod function;
mod item;
mod struct_lowerer;

pub use function::lower_function;
pub use item::lower_item;
pub use struct_lowerer::lower_struct;
