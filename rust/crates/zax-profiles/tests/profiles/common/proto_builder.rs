//! A minimal writer of the protobuf binary encoding, for the tests that need to build a profile
//! byte by byte. The production writer is private to `zax-profiles` on purpose, and the
//! synthetic envelopes AMD-062 REQ-PROF-072 asks for have to be malformed in ways no production
//! writer would produce.

#[derive(Debug, Default)]
pub struct ProtoBuilder {
    bytes: Vec<u8>,
}

impl ProtoBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tag(&mut self, field: u32, wire_type: u32) {
        self.varint((u64::from(field) << 3) | u64::from(wire_type));
    }

    pub fn varint(&mut self, mut value: u64) {
        while value >= 0x80 {
            self.bytes.push(value.to_le_bytes()[0] | 0x80);
            value >>= 7;
        }
        self.bytes.push(value.to_le_bytes()[0]);
    }

    pub fn int32(&mut self, field: u32, value: i32) {
        self.tag(field, 0);
        self.varint(u64::from_le_bytes(i64::from(value).to_le_bytes()));
    }

    pub fn string(&mut self, field: u32, value: &str) {
        self.raw_bytes(field, value.as_bytes());
    }

    pub fn raw_bytes(&mut self, field: u32, value: &[u8]) {
        self.tag(field, 2);
        self.varint(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    /// Writes `value` as a nested length-delimited message.
    pub fn message(&mut self, field: u32, value: &ProtoBuilder) {
        self.raw_bytes(field, &value.bytes);
    }

    pub fn group_start(&mut self, field: u32) {
        self.tag(field, 3);
    }

    pub fn group_end(&mut self, field: u32) {
        self.tag(field, 4);
    }

    pub fn int64(&mut self, field: u32, value: i64) {
        self.tag(field, 0);
        self.varint(u64::from_le_bytes(value.to_le_bytes()));
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
