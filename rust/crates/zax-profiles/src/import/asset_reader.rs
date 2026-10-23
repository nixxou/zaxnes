//! A bounds-checked reader over the bytes of a bundle or a serialized file, with a switchable
//! byte order.

use super::UnityAssetFormatError;

/// Longest null-terminated text the readers accept.
const MAXIMUM_STRING_LENGTH: usize = 4096;

pub(super) struct AssetReader<'a> {
    bytes: &'a [u8],
    position: usize,
    pub(super) big_endian: bool,
}

impl<'a> AssetReader<'a> {
    pub(super) fn new(bytes: &'a [u8], big_endian: bool) -> Self {
        Self {
            bytes,
            position: 0,
            big_endian,
        }
    }

    pub(super) fn position(&self) -> usize {
        self.position
    }

    pub(super) fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    pub(super) fn read_byte(&mut self) -> Result<u8, UnityAssetFormatError> {
        Ok(self.take(1)?[0])
    }

    pub(super) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], UnityAssetFormatError> {
        self.take(count)
    }

    pub(super) fn skip(&mut self, count: usize) -> Result<(), UnityAssetFormatError> {
        self.take(count).map(|_| ())
    }

    pub(super) fn read_u16(&mut self) -> Result<u16, UnityAssetFormatError> {
        let bytes = self.take_array::<2>()?;
        Ok(if self.big_endian {
            u16::from_be_bytes(bytes)
        } else {
            u16::from_le_bytes(bytes)
        })
    }

    pub(super) fn read_i32(&mut self) -> Result<i32, UnityAssetFormatError> {
        let bytes = self.take_array::<4>()?;
        Ok(if self.big_endian {
            i32::from_be_bytes(bytes)
        } else {
            i32::from_le_bytes(bytes)
        })
    }

    pub(super) fn read_u32(&mut self) -> Result<u32, UnityAssetFormatError> {
        let bytes = self.take_array::<4>()?;
        Ok(if self.big_endian {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        })
    }

    pub(super) fn read_i64(&mut self) -> Result<i64, UnityAssetFormatError> {
        let bytes = self.take_array::<8>()?;
        Ok(if self.big_endian {
            i64::from_be_bytes(bytes)
        } else {
            i64::from_le_bytes(bytes)
        })
    }

    pub(super) fn read_null_terminated_string(&mut self) -> Result<String, UnityAssetFormatError> {
        let start = self.position;
        let rest = &self.bytes[start..];
        let reachable = &rest[..rest.len().min(MAXIMUM_STRING_LENGTH + 1)];
        let Some(length) = reachable.iter().position(|byte| *byte == 0) else {
            return Err(UnityAssetFormatError::new(
                if rest.len() > MAXIMUM_STRING_LENGTH {
                    format!(
                        "a text field is longer than the accepted {MAXIMUM_STRING_LENGTH} bytes."
                    )
                } else {
                    "a text field is not terminated before the end of the file.".to_owned()
                },
            ));
        };
        self.position = start + length + 1;
        Ok(String::from_utf8_lossy(&rest[..length]).into_owned())
    }

    /// Moves to the next multiple of `alignment` bytes from the start of the data.
    pub(super) fn align(&mut self, alignment: usize) -> Result<(), UnityAssetFormatError> {
        let padding = (alignment - (self.position % alignment)) % alignment;
        if padding == 0 {
            return Ok(());
        }
        if padding > self.remaining() {
            // Padding that runs past the end is how a truncated file shows up here; it is the
            // same refusal as a short read.
            return Err(UnityAssetFormatError::new(format!(
                "the file ends inside the padding that aligns position {} on {alignment} bytes.",
                self.position
            )));
        }
        self.position += padding;
        Ok(())
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], UnityAssetFormatError> {
        if count > self.remaining() {
            return Err(UnityAssetFormatError::new(format!(
                "the file ends at byte {}: {count} byte(s) were expected at {}, {} left.",
                self.bytes.len(),
                self.position,
                self.remaining()
            )));
        }
        let slice = &self.bytes[self.position..self.position + count];
        self.position += count;
        Ok(slice)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], UnityAssetFormatError> {
        let slice = self.take(N)?;
        let mut array = [0_u8; N];
        array.copy_from_slice(slice);
        Ok(array)
    }
}
