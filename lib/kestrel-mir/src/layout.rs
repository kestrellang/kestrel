use crate::IntBits;

/// How a struct or enum declared its layout (`@layout(...)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LayoutKind {
    /// No `@layout`: unspecified. Today declaration order with natural
    /// alignment, the same as `C`, but a later release may reorder fields.
    #[default]
    Default,
    /// `@layout(.C)`: C's rules. On a payload-less enum, an `Int32`
    /// discriminant.
    C,
    /// `@layout(.Packed)`: declaration order, every field at alignment 1, no
    /// padding.
    Packed,
}

/// `@layout` and `@align`, set by MIR lowering on a struct or enum
/// definition. The only input to a layout besides its field types: every
/// layout goes through [`StructLayout::with_attrs`] / `build_enum_layout`, so
/// the attributes are read in one place and the backends cannot disagree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LayoutAttrs {
    pub kind: LayoutKind,
    /// `@align(N)`: the type's alignment is at least `N`.
    pub min_align: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLayout {
    pub size: u64,
    pub align: u64,
    pub field_offsets: Vec<u64>,
    /// The last field is unsized (`RcBoxStorage[rawany P]`): `size`/`align`
    /// cover the sized prefix only, `field_offsets` has no entry for the
    /// tail, and the tail's offset is computed at runtime from the
    /// payload's alignment (existentials plan Phase 9).
    pub unsized_tail: bool,
    /// `@layout(.Packed)`: fields may sit at misaligned offsets, so code
    /// generation accesses them with alignment 1.
    pub packed: bool,
    /// Raised alignment to apply when the layout is finished.
    min_align: Option<u64>,
}

impl StructLayout {
    /// An empty layout with natural alignment (tuples, enum payloads).
    pub fn new() -> Self {
        Self::with_attrs(LayoutAttrs::default())
    }

    /// An empty layout for a struct declared with `attrs`. Append the fields,
    /// then [`Self::finish`].
    pub fn with_attrs(attrs: LayoutAttrs) -> Self {
        Self {
            size: 0,
            align: 1,
            field_offsets: Vec::new(),
            unsized_tail: false,
            packed: attrs.kind == LayoutKind::Packed,
            min_align: attrs.min_align,
        }
    }

    pub fn scalar(size: u64, align: u64) -> Self {
        Self {
            size,
            align,
            ..Self::new()
        }
    }

    pub fn append_field(&mut self, field_layout: StructLayout) -> u64 {
        // A packed struct places every field at alignment 1.
        let field_align = if self.packed { 1 } else { field_layout.align };
        let field_size = field_layout.size;
        if field_align > 0 {
            self.align = self.align.max(field_align);
            let padding = (field_align - (self.size % field_align)) % field_align;
            self.size += padding;
        }
        let offset = self.size;
        self.field_offsets.push(offset);
        self.size += field_size;
        offset
    }

    pub fn pad_to_align(&mut self) {
        if self.align > 0 {
            let padding = (self.align - (self.size % self.align)) % self.align;
            self.size += padding;
        }
    }

    /// Apply `@align` and pad the size to the alignment, so arrays of the
    /// type stay aligned. An unsized tail is not padded: its offset is
    /// computed at runtime.
    pub fn finish(mut self) -> Self {
        if let Some(min) = self.min_align {
            self.align = self.align.max(min);
        }
        if !self.unsized_tail {
            self.pad_to_align();
        }
        self
    }
}

impl Default for StructLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumLayout {
    pub size: u64,
    pub align: u64,
    pub discriminant_width: IntBits,
    pub payload_offset: u64,
    pub variant_layouts: Vec<StructLayout>,
}
