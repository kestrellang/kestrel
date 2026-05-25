use crate::IntBits;

#[derive(Debug, Clone, PartialEq)]
pub struct StructLayout {
    pub size: u64,
    pub align: u64,
    pub field_offsets: Vec<u64>,
}

impl StructLayout {
    pub fn new() -> Self {
        Self { size: 0, align: 1, field_offsets: Vec::new() }
    }

    pub fn scalar(size: u64, align: u64) -> Self {
        Self { size, align, field_offsets: Vec::new() }
    }

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
