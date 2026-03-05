//! Runtime intrinsics (panic, string operations, etc.)
//!
//! Note: Most intrinsic-like operations in Kestrel are represented as Rvalue
//! variants rather than as a separate Intrinsic enum. They are implemented
//! directly in rvalue.rs:
//!
//! ## String Operations (implemented in rvalue.rs)
//! - `Rvalue::StrPtr` - Extract pointer from string fat pointer
//! - `Rvalue::StrLen` - Extract length from string fat pointer
//!
//! ## Type Casts (implemented in rvalue.rs)
//! - `Rvalue::Cast` - Type conversion operations
//!   - IntWiden: Sign-extend to larger integer
//!   - IntTruncate: Truncate to smaller integer
//!   - IntToFloat: Convert signed integer to float
//!   - FloatToInt: Convert float to signed integer
//!   - FloatWiden: f32 -> f64 promotion
//!   - FloatTruncate: f64 -> f32 demotion
//!   - PtrBitcast: Reinterpret pointer as different pointer type
//!   - RefToImmut: Convert mutable ref to immutable ref
//!
//! ## Pointer Operations (implemented in rvalue.rs)
//! - `Rvalue::RefToPtr` - Reference to pointer
//! - `Rvalue::PtrOffset` - Pointer arithmetic
//!
//! ## TODO: Not yet implemented
//! - panic: print message and abort
//! - IntToString: integer to string conversion (needs runtime support)
