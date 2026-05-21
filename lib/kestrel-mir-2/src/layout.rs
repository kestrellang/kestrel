use crate::IntBits;

#[derive(Debug, Clone, PartialEq)]
pub struct StructLayout {
    pub size: u64,
    pub align: u64,
    pub field_offsets: Vec<u64>,
}

impl StructLayout {
    pub fn new() -> Self {
        Self {
            size: 0,
            align: 1,
            field_offsets: Vec::new(),
        }
    }

    pub fn scalar(size: u64, align: u64) -> Self {
        Self {
            size,
            align,
            field_offsets: Vec::new(),
        }
    }

    /// Sequentially append a field, returning its byte offset.
    pub fn append_field(&mut self, field_layout: StructLayout) -> u64 {
        let field_align = field_layout.align;
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

    /// Round size up to alignment boundary.
    pub fn pad_to_align(&mut self) {
        if self.align > 0 {
            let padding = (self.align - (self.size % self.align)) % self.align;
            self.size += padding;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_layout_scalar() {
        let layout = StructLayout::scalar(8, 8);
        assert_eq!(layout.size, 8);
        assert_eq!(layout.align, 8);
        assert!(layout.field_offsets.is_empty());
    }

    #[test]
    fn struct_layout_append_fields() {
        let mut layout = StructLayout::new();
        let off0 = layout.append_field(StructLayout::scalar(1, 1)); // i8
        let off1 = layout.append_field(StructLayout::scalar(8, 8)); // i64 (needs padding)
        layout.pad_to_align();
        assert_eq!(off0, 0);
        assert_eq!(off1, 8);
        assert_eq!(layout.size, 16);
        assert_eq!(layout.align, 8);
        assert_eq!(layout.field_offsets, vec![0, 8]);
    }

    #[test]
    fn struct_layout_no_padding_needed() {
        let mut layout = StructLayout::new();
        layout.append_field(StructLayout::scalar(8, 8));
        layout.append_field(StructLayout::scalar(8, 8));
        layout.pad_to_align();
        assert_eq!(layout.size, 16);
        assert_eq!(layout.align, 8);
        assert_eq!(layout.field_offsets, vec![0, 8]);
    }

    #[test]
    fn struct_layout_trailing_padding() {
        let mut layout = StructLayout::new();
        layout.append_field(StructLayout::scalar(8, 8)); // i64
        layout.append_field(StructLayout::scalar(1, 1)); // i8
        layout.pad_to_align();
        assert_eq!(layout.size, 16);
        assert_eq!(layout.align, 8);
    }

    #[test]
    fn struct_layout_three_fields() {
        let mut layout = StructLayout::new();
        layout.append_field(StructLayout::scalar(4, 4)); // i32
        layout.append_field(StructLayout::scalar(1, 1)); // i8
        layout.append_field(StructLayout::scalar(2, 2)); // i16
        layout.pad_to_align();
        assert_eq!(layout.field_offsets, vec![0, 4, 6]);
        assert_eq!(layout.size, 8);
        assert_eq!(layout.align, 4);
    }
}
