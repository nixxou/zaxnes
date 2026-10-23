//! The protobuf reader of the profile files: varints, fixed 32-bit floats, length-delimited
//! fields and groups, over a byte slice.

use super::ProfileFormatError;

/// Wire types of the protobuf encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WireType {
    Varint,
    Fixed64,
    Length,
    StartGroup,
    EndGroup,
    Fixed32,
    /// 6 and 7, which no message uses.
    Invalid(u8),
}

impl WireType {
    fn of_tag(tag: u64) -> Self {
        match tag & 7 {
            0 => Self::Varint,
            1 => Self::Fixed64,
            2 => Self::Length,
            3 => Self::StartGroup,
            4 => Self::EndGroup,
            5 => Self::Fixed32,
            other => Self::Invalid(other.to_le_bytes()[0]),
        }
    }

    fn number(self) -> u8 {
        match self {
            Self::Varint => 0,
            Self::Fixed64 => 1,
            Self::Length => 2,
            Self::StartGroup => 3,
            Self::EndGroup => 4,
            Self::Fixed32 => 5,
            Self::Invalid(number) => number,
        }
    }
}

pub(crate) struct ProtoReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> ProtoReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// End of the data, the limit of the root message.
    pub(crate) fn end(&self) -> usize {
        self.data.len()
    }

    /// Reads the next tag of a message that ends at `limit`, or none at its end.
    pub(crate) fn try_read_tag(
        &mut self,
        limit: usize,
    ) -> Result<Option<(u32, WireType)>, ProfileFormatError> {
        if self.position >= limit {
            return Ok(None);
        }
        let tag = self.read_varint()?;
        let field = self.field_of(tag)?;
        Ok(Some((field, WireType::of_tag(tag))))
    }

    pub(crate) fn read_varint(&mut self) -> Result<u64, ProfileFormatError> {
        let mut result = 0_u64;
        for index in 0..10 {
            let current = *self.data.get(self.position).ok_or_else(|| {
                ProfileFormatError::new("The file ends in the middle of a varint.")
            })?;
            self.position += 1;
            result |= u64::from(current & 0x7F) << (7 * index);
            if current & 0x80 == 0 {
                return Ok(result);
            }
        }
        Err(ProfileFormatError::new(format!(
            "A varint at offset {} is longer than ten bytes.",
            self.position
        )))
    }

    pub(crate) fn read_i64(&mut self) -> Result<i64, ProfileFormatError> {
        Ok(i64::from_le_bytes(self.read_varint()?.to_le_bytes()))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32, ProfileFormatError> {
        // Int32 fields are varints of the sign-extended value; the low 32 bits are the value
        // (and a varint written with more bits is truncated, as the reference reader does).
        let [b0, b1, b2, b3, ..] = self.read_varint()?.to_le_bytes();
        Ok(i32::from_le_bytes([b0, b1, b2, b3]))
    }

    pub(crate) fn read_bool(&mut self) -> Result<bool, ProfileFormatError> {
        Ok(self.read_varint()? != 0)
    }

    pub(crate) fn read_f32(&mut self) -> Result<f32, ProfileFormatError> {
        let bytes = self.take(4)?;
        Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Reads a length prefix and returns the offset where the field ends.
    pub(crate) fn read_length_limit(&mut self) -> Result<usize, ProfileFormatError> {
        let length = self.read_varint()?;
        let available = self.data.len() - self.position;
        match usize::try_from(length) {
            Ok(length) if length <= available => Ok(self.position + length),
            _ => Err(ProfileFormatError::new(format!(
                "A length-delimited field at offset {} claims {length} bytes, past the end of the file.",
                self.position
            ))),
        }
    }

    pub(crate) fn read_string(&mut self) -> Result<String, ProfileFormatError> {
        let limit = self.read_length_limit()?;
        // Invalid UTF-8 becomes U+FFFD, as the reference decoder does.
        let value = String::from_utf8_lossy(&self.data[self.position..limit]).into_owned();
        self.position = limit;
        Ok(value)
    }

    pub(crate) fn skip(&mut self, field: u32, wire_type: WireType) -> Result<(), ProfileFormatError> {
        match wire_type {
            WireType::Varint => {
                self.read_varint()?;
            }
            WireType::Fixed64 => {
                self.take(8)?;
            }
            WireType::Length => self.position = self.read_length_limit()?,
            WireType::Fixed32 => {
                self.take(4)?;
            }
            WireType::StartGroup => self.skip_group(field)?,
            WireType::EndGroup | WireType::Invalid(_) => {
                return Err(ProfileFormatError::new(format!(
                    "Wire type {} of field {field} at offset {} is not valid here.",
                    wire_type.number(),
                    self.position
                )));
            }
        }
        Ok(())
    }

    pub(crate) fn expect_group_end(&mut self, field: u32) -> Result<(), ProfileFormatError> {
        let tag = self.read_varint()?;
        if u32::try_from(field_number(tag)) != Ok(field) || WireType::of_tag(tag) != WireType::EndGroup {
            return Err(ProfileFormatError::new(format!(
                "Expected the end of group {field} at offset {}.",
                self.position
            )));
        }
        Ok(())
    }

    /// Reads the next tag inside group `group_field`; none at the group's end tag.
    pub(crate) fn try_read_group_tag(
        &mut self,
        group_field: u32,
    ) -> Result<Option<(u32, WireType)>, ProfileFormatError> {
        let tag = self.read_varint()?;
        let wire_type = WireType::of_tag(tag);
        if wire_type == WireType::EndGroup {
            return if u32::try_from(field_number(tag)) == Ok(group_field) {
                Ok(None)
            } else {
                Err(ProfileFormatError::new(format!(
                    "Group {group_field} ends with the tag of group {} at offset {}.",
                    field_number(tag),
                    self.position
                )))
            };
        }
        Ok(Some((self.field_of(tag)?, wire_type)))
    }

    fn skip_group(&mut self, field: u32) -> Result<(), ProfileFormatError> {
        while let Some((inner, wire_type)) = self.try_read_group_tag(field)? {
            self.skip(inner, wire_type)?;
        }
        Ok(())
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], ProfileFormatError> {
        if self.data.len() - self.position < count {
            return Err(ProfileFormatError::new(format!(
                "The file ends {count} bytes too early at offset {}.",
                self.position
            )));
        }
        let bytes = &self.data[self.position..self.position + count];
        self.position += count;
        Ok(bytes)
    }

    /// The field number of `tag`, which must be positive.
    fn field_of(&self, tag: u64) -> Result<u32, ProfileFormatError> {
        let field = field_number(tag);
        u32::try_from(field)
            .ok()
            .filter(|field| *field > 0)
            .ok_or_else(|| {
                ProfileFormatError::new(format!(
                    "Field number {field} is not valid at offset {}.",
                    self.position
                ))
            })
    }
}

/// The field number of a tag as the reference reads it: the low 32 bits of `tag >> 3`, signed.
fn field_number(tag: u64) -> i32 {
    let [b0, b1, b2, b3, ..] = (tag >> 3).to_le_bytes();
    i32::from_le_bytes([b0, b1, b2, b3])
}
