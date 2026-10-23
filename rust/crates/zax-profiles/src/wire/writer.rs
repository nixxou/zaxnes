//! The protobuf writer of the profile files. Nested messages are written first and prefixed
//! with their length afterwards, in a single pass.

use super::reader::WireType;

#[derive(Debug)]
pub(crate) struct ProtoWriter {
    buffer: Vec<u8>,
}

impl ProtoWriter {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity.max(16)),
        }
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    pub(crate) fn write_tag(&mut self, field: u32, wire_type: WireType) {
        let wire = match wire_type {
            WireType::Varint => 0,
            WireType::Fixed64 => 1,
            WireType::Length => 2,
            WireType::StartGroup => 3,
            WireType::EndGroup => 4,
            WireType::Fixed32 => 5,
            WireType::Invalid(number) => u64::from(number),
        };
        self.write_varint((u64::from(field) << 3) | wire);
    }

    pub(crate) fn write_varint(&mut self, value: u64) {
        push_varint(&mut self.buffer, value);
    }

    pub(crate) fn write_i32(&mut self, field: u32, value: i32) {
        self.write_tag(field, WireType::Varint);
        // Negative values are sign-extended to ten bytes, as protobuf writes an int32.
        self.write_varint(u64::from_le_bytes(i64::from(value).to_le_bytes()));
    }

    pub(crate) fn write_i64(&mut self, field: u32, value: i64) {
        self.write_tag(field, WireType::Varint);
        self.write_varint(u64::from_le_bytes(value.to_le_bytes()));
    }

    pub(crate) fn write_bool(&mut self, field: u32, value: bool) {
        self.write_tag(field, WireType::Varint);
        self.write_varint(u64::from(value));
    }

    pub(crate) fn write_f32(&mut self, field: u32, value: f32) {
        self.write_tag(field, WireType::Fixed32);
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn write_string(&mut self, field: u32, value: &str) {
        self.write_tag(field, WireType::Length);
        self.write_varint(value.len() as u64);
        self.buffer.extend_from_slice(value.as_bytes());
    }

    /// Opens a length-delimited field; the returned offset goes to [`Self::end_nested`].
    pub(crate) fn begin_nested(&mut self, field: u32) -> usize {
        self.write_tag(field, WireType::Length);
        self.buffer.len()
    }

    /// Closes the field opened at `start`: inserts the length of its body before it.
    pub(crate) fn end_nested(&mut self, start: usize) {
        let body_length = (self.buffer.len() - start) as u64;
        let mut prefix = Vec::with_capacity(10);
        push_varint(&mut prefix, body_length);
        self.buffer.splice(start..start, prefix);
    }

    pub(crate) fn write_group_start(&mut self, field: u32) {
        self.write_tag(field, WireType::StartGroup);
    }

    pub(crate) fn write_group_end(&mut self, field: u32) {
        self.write_tag(field, WireType::EndGroup);
    }
}

fn push_varint(buffer: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        buffer.push(value.to_le_bytes()[0] | 0x80);
        value >>= 7;
    }
    buffer.push(value.to_le_bytes()[0]);
}
