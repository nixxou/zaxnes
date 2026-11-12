//! Savestate block encoding (port of `src/Zax.Nes/State`): a plain, fixed-width,
//! little-endian format, so that a block is reproducible byte for byte from the same machine
//! state (REQ-ARCH-012, REQ-CORE-090).

// The two typed causes a restore can fail with are on the `Engine` trait, so they live in
// `zax-core-api` since lot FCEUX-23 step 2; re-exported here at the path they were at. The
// *format* below — the writer, the reader and the in-memory `CoreState` — stays Zaxnes's own.
pub use zax_core_api::{InvalidState, StateLoadFailure};

/// The in-memory state of a core (AMD-041 REQ-CORE-121): the buffer
/// [`NesCore::capture_core_state`] fills and [`NesCore::restore_core_state`] reads back.
///
/// This is **not** the portable savestate of REQ-CORE-090. It carries no magic number, no format
/// version, no core name, no ROM name and no ROM image; it is never written to a file, and it is
/// only ever read back by the very instance that wrote it, with the same ROM still loaded. That
/// is what the notes of REQ-LOOP-065 allow, and it is what makes a capture and a restore at every
/// real frame affordable: the buffer is reused, so run-ahead allocates nothing once it has grown
/// to its size. It also leaves out the half of the PPU state that *is* the frame just run
/// ([`crate::ppu::Ppu2C02::save_frame_records`]), which the emulation never reads back.
///
/// The size is bounded by [`NesCore::max_core_state_size`], as REQ-CORE-121 requires.
///
/// [`NesCore::capture_core_state`]: crate::NesCore::capture_core_state
/// [`NesCore::restore_core_state`]: crate::NesCore::restore_core_state
/// [`NesCore::max_core_state_size`]: crate::NesCore::max_core_state_size
#[derive(Debug, Clone, Default)]
pub struct CoreState {
    bytes: Vec<u8>,
}

impl CoreState {
    /// An empty state whose buffer holds `capacity` bytes already.
    /// [`NesCore::max_core_state_size`] is the value that makes it grow exactly never.
    ///
    /// [`NesCore::max_core_state_size`]: crate::NesCore::max_core_state_size
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    /// Bytes the captured state occupies, 0 when nothing was captured yet.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether no state has been captured into this buffer.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The captured bytes, for a test that compares two captures.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Empties the buffer, keeping its allocation, and hands a writer over it to the capture.
    pub(crate) fn begin(&mut self) -> StateWriter {
        StateWriter::reusing(std::mem::take(&mut self.bytes))
    }

    /// Takes the bytes of `writer` back after a capture.
    pub(crate) fn end(&mut self, writer: StateWriter) {
        self.bytes = writer.into_vec();
    }
}

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
    format_version: i32,
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
            format_version: 0,
        })
    }

    /// Format version of the block being read, 0 until the caller that read the header hands
    /// it over with [`Self::set_format_version`]. An object whose own layout grew with a
    /// version of the format reads it to know which layout the bytes follow: the MMC5's sound
    /// state (REQ-CORE-075, AMD-054) is the first such case.
    #[must_use]
    pub const fn format_version(&self) -> i32 {
        self.format_version
    }

    /// Records the format version read from the block's header.
    pub const fn set_format_version(&mut self, version: i32) {
        self.format_version = version;
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

    /// Reads a length-prefixed blob into `destination` without requiring the stored length to
    /// match: the bytes that fit are kept, any excess is dropped, and a shorter blob leaves
    /// the tail of `destination` as it was.
    ///
    /// Only the cartridge's volatile memories use it, because their size depends on the board
    /// and lot MF-1 cut three of them down to what the board really carries (MR-4-6: the
    /// MMC6's 1 KiB of PRG-RAM; MR-13-1: CPROM's 16 KiB of CHR-RAM; MR-30-2: UNROM 512's
    /// absent work RAM). A state written before that lot stores the header's larger blob for
    /// those boards and has to stay loadable (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block holds fewer bytes than its own length prefix announces.
    pub fn read_blob_prefix_into(&mut self, destination: &mut [u8]) -> Result<(), InvalidState> {
        let count = self.read_usize()?;
        let stored = self.read_raw(count)?;
        let shared = count.min(destination.len());
        destination[..shared].copy_from_slice(&stored[..shared]);
        Ok(())
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
