//! Savestate block encoding (port of `src/Zax.Nes/State`): a plain, fixed-width,
//! little-endian format, so that a block is reproducible byte for byte from the same machine
//! state (REQ-ARCH-012, REQ-CORE-090).

use std::fmt;

/// Typed cause of a failed "restore state" (REQ-CORE-091, AREQ-CORE-120; AMD-002 answer to
/// Q-001). The host picks the text it shows from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateLoadFailure {
    /// The block is invalid or truncated ("savestate corrompu").
    CorruptState,
    /// The block was written by another core, which REQ-CORE-120 refuses with the literal
    /// message `Save state used a different core.`
    ForeignCore,
}

/// A savestate block that is invalid or truncated, or written by another core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidState {
    /// Typed cause of the failure.
    pub failure: StateLoadFailure,
    /// Technical message naming the cause.
    pub message: String,
}

impl InvalidState {
    /// A corrupt or truncated block, described by `message`.
    #[must_use]
    pub fn corrupt(message: impl Into<String>) -> Self {
        Self {
            failure: StateLoadFailure::CorruptState,
            message: message.into(),
        }
    }
}

impl fmt::Display for InvalidState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for InvalidState {}

/// Append-only little-endian writer used to build a savestate block (REQ-CORE-090).
#[derive(Debug, Clone, Default)]
pub struct StateWriter {
    buffer: Vec<u8>,
}

impl StateWriter {
    /// Creates a writer with an initial capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Creates a writer that appends to `buffer` after emptying it: a buffer of an earlier
    /// state is reused without a new allocation.
    #[must_use]
    pub fn reusing(mut buffer: Vec<u8>) -> Self {
        buffer.clear();
        Self { buffer }
    }

    /// Number of bytes written so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether nothing has been written yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Writes one byte.
    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    /// Writes a boolean as one byte, 0 or 1.
    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    /// Writes a signed byte.
    pub fn write_i8(&mut self, value: i8) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 16-bit unsigned integer.
    pub fn write_u16(&mut self, value: u16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 32-bit unsigned integer, as the bits of the signed integer the format uses.
    pub fn write_u32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 16-bit signed integer.
    pub fn write_i16(&mut self, value: i16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 32-bit signed integer.
    pub fn write_i32(&mut self, value: i32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 64-bit signed integer.
    pub fn write_i64(&mut self, value: i64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 64-bit unsigned integer.
    pub fn write_u64(&mut self, value: u64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a double.
    pub fn write_f64(&mut self, value: f64) {
        self.write_u64(value.to_bits());
    }

    /// Writes a size or an offset as the 32-bit signed integer the format stores. They are
    /// bounded by the size of a ROM or of a buffer, far below 2 GiB; anything larger is
    /// clamped to `i32::MAX` and refused by the reader's own checks.
    pub fn write_usize(&mut self, value: usize) {
        self.write_i32(i32::try_from(value).unwrap_or(i32::MAX));
    }

    /// Writes raw bytes, without a length prefix.
    pub fn write_raw(&mut self, value: &[u8]) {
        self.buffer.extend_from_slice(value);
    }

    /// Writes a 32-bit length followed by the bytes.
    pub fn write_blob(&mut self, value: &[u8]) {
        self.write_usize(value.len());
        self.write_raw(value);
    }

    /// Writes a 32-bit byte count followed by the UTF-8 bytes of `value`.
    pub fn write_string(&mut self, value: &str) {
        self.write_blob(value.as_bytes());
    }

    /// The bytes written so far.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Consumes the writer and returns its bytes.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }
}

/// Little-endian reader over a savestate block, counterpart of [`StateWriter`].
///
/// It starts at an arbitrary offset inside a larger slice and reports how many bytes it
/// consumed, so that the host can read its own data placed right after the core block
/// (REQ-CORE-091). Every read is bounds-checked and returns [`InvalidState`] rather than
/// corrupting the machine.
#[derive(Debug, Clone)]
pub struct StateReader<'a> {
    data: &'a [u8],
    origin: usize,
    position: usize,
}

impl<'a> StateReader<'a> {
    /// Creates a reader starting at `offset` in `data`.
    ///
    /// # Errors
    /// [`InvalidState`] when `offset` lies beyond the end of `data`.
    pub fn new(data: &'a [u8], offset: usize) -> Result<Self, InvalidState> {
        if offset > data.len() {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: start position {offset} is outside the {}-byte array.",
                data.len()
            )));
        }

        Ok(Self {
            data,
            origin: offset,
            position: offset,
        })
    }

    /// Number of bytes consumed since the start position.
    #[must_use]
    pub const fn consumed(&self) -> usize {
        self.position - self.origin
    }

    /// Number of bytes left in the underlying slice.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    /// Reads `count` raw bytes.
    ///
    /// # Errors
    /// [`InvalidState`] when fewer than `count` bytes are left.
    pub fn read_raw(&mut self, count: usize) -> Result<&'a [u8], InvalidState> {
        if count > self.remaining() {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: {count} bytes were needed at offset {}, only {} are available.",
                self.position,
                self.remaining()
            )));
        }

        let bytes = &self.data[self.position..self.position + count];
        self.position += count;
        Ok(bytes)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], InvalidState> {
        let mut array = [0; N];
        array.copy_from_slice(self.read_raw(N)?);
        Ok(array)
    }

    /// Reads one byte.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_u8(&mut self) -> Result<u8, InvalidState> {
        Ok(self.read_array::<1>()?[0])
    }

    /// Reads a boolean written by [`StateWriter::write_bool`].
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_bool(&mut self) -> Result<bool, InvalidState> {
        Ok(self.read_u8()? != 0)
    }

    /// Reads a signed byte.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_i8(&mut self) -> Result<i8, InvalidState> {
        Ok(i8::from_le_bytes(self.read_array()?))
    }

    /// Reads a 16-bit unsigned integer.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_u16(&mut self) -> Result<u16, InvalidState> {
        Ok(u16::from_le_bytes(self.read_array()?))
    }

    /// Reads a 32-bit unsigned integer written by [`StateWriter::write_u32`].
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_u32(&mut self) -> Result<u32, InvalidState> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    /// Reads a 16-bit signed integer.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_i16(&mut self) -> Result<i16, InvalidState> {
        Ok(i16::from_le_bytes(self.read_array()?))
    }

    /// Reads a 32-bit signed integer.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_i32(&mut self) -> Result<i32, InvalidState> {
        Ok(i32::from_le_bytes(self.read_array()?))
    }

    /// Reads a 64-bit signed integer.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_i64(&mut self) -> Result<i64, InvalidState> {
        Ok(i64::from_le_bytes(self.read_array()?))
    }

    /// Reads a 64-bit unsigned integer.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_u64(&mut self) -> Result<u64, InvalidState> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }

    /// Reads a double.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block.
    pub fn read_f64(&mut self) -> Result<f64, InvalidState> {
        Ok(f64::from_bits(self.read_u64()?))
    }

    /// Reads a size or an offset written by [`StateWriter::write_usize`].
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block or for a negative value.
    pub fn read_usize(&mut self) -> Result<usize, InvalidState> {
        let value = self.read_i32()?;
        usize::try_from(value).map_err(|_| {
            InvalidState::corrupt(format!("Corrupt savestate: negative count {value}."))
        })
    }

    /// Reads an index written by [`StateWriter::write_usize`] that must be below `limit`, so
    /// that a restored table position can never address outside its table.
    ///
    /// # Errors
    /// [`InvalidState`] at the end of the block or when the index is `limit` or more.
    pub fn read_index(&mut self, limit: usize) -> Result<usize, InvalidState> {
        let index = self.read_usize()?;
        if index >= limit {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: index {index} is outside 0..{limit}."
            )));
        }

        Ok(index)
    }

    /// Reads exactly `destination.len()` bytes into it.
    ///
    /// # Errors
    /// [`InvalidState`] when too few bytes are left.
    pub fn read_into(&mut self, destination: &mut [u8]) -> Result<(), InvalidState> {
        destination.copy_from_slice(self.read_raw(destination.len())?);
        Ok(())
    }

    /// Reads a length-prefixed blob into `destination`, which must have exactly the stored
    /// length.
    ///
    /// # Errors
    /// [`InvalidState`] when the stored length differs or the block is truncated.
    pub fn read_blob_into(&mut self, destination: &mut [u8]) -> Result<(), InvalidState> {
        let count = self.read_usize()?;
        if count != destination.len() {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: a block of {} bytes was expected, {count} were stored.",
                destination.len()
            )));
        }

        self.read_into(destination)
    }

    /// Reads a length-prefixed blob as a new vector.
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn read_blob(&mut self) -> Result<Vec<u8>, InvalidState> {
        let count = self.read_usize()?;
        Ok(self.read_raw(count)?.to_vec())
    }

    /// Reads a string written by [`StateWriter::write_string`].
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn read_string(&mut self) -> Result<String, InvalidState> {
        let count = self.read_usize()?;
        Ok(String::from_utf8_lossy(self.read_raw(count)?).into_owned())
    }
}
