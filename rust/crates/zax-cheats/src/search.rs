//! A RAM search, the tool used to find where a game keeps its lives, energy or money.
//!
//! The classic workflow of FCEUX's cheat search, BizHawk's RAM Search and RetroArch's cheat
//! search: [`RamSearch::start`] takes every address of the chosen regions as a candidate with its
//! current value; after playing a bit, [`RamSearch::filter`] keeps only the candidates whose value
//! now compares as asked, either to a number the player knows ("lives = 3") or to the value at
//! the previous step ("energy went down"). A few rounds leave a handful of addresses, which become
//! cheats (`AAAA:VV`).
//!
//! Memory is read through a closure, `Fn(u16) -> u8`, the engines' `read_memory`, so the search
//! has no engine dependency. On the NES a pointer scan buys little: RAM is 2 KB at fixed places
//! and games rarely allocate; what matters is the value encodings NES games use, which is why
//! [`ValueSize`] has BCD (two decimal digits per byte) besides 8 and 16-bit values.

/// A range of CPU addresses to search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// First address.
    pub start: u16,
    /// Number of bytes.
    pub len: u16,
}

/// The console's 2 KB of RAM, `$0000-$07FF`.
pub const CPU_RAM: Region = Region { start: 0x0000, len: 0x0800 };

/// The cartridge's work RAM, `$6000-$7FFF`, when it has some.
pub const WORK_RAM: Region = Region { start: 0x6000, len: 0x2000 };

/// How the bytes at a candidate address are read as a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSize {
    /// One byte.
    Byte,
    /// Two bytes, little-endian, as the 6502 stores them.
    Word,
    /// One byte holding two decimal digits (`0x42` is 42); a byte that is not valid BCD is
    /// dropped by a filter.
    Bcd,
}

/// The comparison a filter makes between the current value and its operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    Less,
    /// `>`
    Greater,
    /// `<=`
    LessOrEqual,
    /// `>=`
    GreaterOrEqual,
}

impl Comparison {
    fn holds(self, current: i64, operand: i64) -> bool {
        match self {
            Self::Equal => current == operand,
            Self::NotEqual => current != operand,
            Self::Less => current < operand,
            Self::Greater => current > operand,
            Self::LessOrEqual => current <= operand,
            Self::GreaterOrEqual => current >= operand,
        }
    }
}

/// What the current value is compared to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// The value at the previous step (unknown-value search).
    Previous,
    /// The previous value plus a delta ("went down by 1" is `Equal` to `PreviousPlus(-1)`).
    PreviousPlus(i64),
    /// A number the player knows.
    Value(i64),
}

/// One address still in the running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// CPU address of the value's first byte.
    pub address: u16,
    /// Value at the last step.
    pub previous: i64,
}

/// A search in progress.
#[derive(Debug, Clone)]
pub struct RamSearch {
    regions: Vec<Region>,
    size: ValueSize,
    signed: bool,
    candidates: Vec<Candidate>,
    history: Vec<Vec<Candidate>>,
}

/// How many filter steps [`RamSearch::undo`] can take back.
const HISTORY_DEPTH: usize = 16;

impl RamSearch {
    /// An empty search over `regions`, reading values as `size` (signed when `signed`, which only
    /// changes how 8 and 16-bit values compare).
    #[must_use]
    pub fn new(regions: &[Region], size: ValueSize, signed: bool) -> Self {
        Self { regions: regions.to_vec(), size, signed, candidates: Vec::new(), history: Vec::new() }
    }

    /// The value at `address`, `None` when it does not decode (BCD) or runs past 64 KB.
    fn value_at(&self, address: u16, read: &impl Fn(u16) -> u8) -> Option<i64> {
        match self.size {
            ValueSize::Byte => {
                let byte = read(address);
                Some(if self.signed { i64::from(i8::from_ne_bytes([byte])) } else { i64::from(byte) })
            }
            ValueSize::Word => {
                let word = u16::from_le_bytes([read(address), read(address.checked_add(1)?)]);
                Some(if self.signed { i64::from(i16::from_ne_bytes(word.to_ne_bytes())) } else { i64::from(word) })
            }
            ValueSize::Bcd => {
                let byte = read(address);
                let (high, low) = (byte >> 4, byte & 0x0F);
                (high < 10 && low < 10).then(|| i64::from(high * 10 + low))
            }
        }
    }

    /// Starts again: every address of the regions is a candidate with its current value.
    pub fn start(&mut self, read: impl Fn(u16) -> u8) {
        let width: u16 = if self.size == ValueSize::Word { 2 } else { 1 };
        let mut candidates = Vec::new();
        for region in &self.regions {
            let count = region.len.saturating_sub(width - 1);
            for offset in 0..count {
                let Some(address) = region.start.checked_add(offset) else { break };
                if let Some(previous) = self.value_at(address, &read) {
                    candidates.push(Candidate { address, previous });
                }
            }
        }
        self.candidates = candidates;
        self.history.clear();
    }

    /// Keeps the candidates whose current value compares to `operand` as `comparison` says, and
    /// makes the current values the previous ones for the next step.
    pub fn filter(&mut self, comparison: Comparison, operand: Operand, read: impl Fn(u16) -> u8) {
        let kept: Vec<Candidate> = self
            .candidates
            .iter()
            .filter_map(|candidate| {
                let current = self.value_at(candidate.address, &read)?;
                let target = match operand {
                    Operand::Previous => candidate.previous,
                    Operand::PreviousPlus(delta) => candidate.previous.saturating_add(delta),
                    Operand::Value(value) => value,
                };
                comparison
                    .holds(current, target)
                    .then_some(Candidate { address: candidate.address, previous: current })
            })
            .collect();
        if self.history.len() == HISTORY_DEPTH {
            self.history.remove(0);
        }
        self.history.push(std::mem::replace(&mut self.candidates, kept));
    }

    /// Takes the last filter back; `false` when there is none to take back.
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(candidates) => {
                self.candidates = candidates;
                true
            }
            None => false,
        }
    }

    /// The candidates left, in address order.
    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// Removes one candidate the player ruled out.
    pub fn remove(&mut self, address: u16) {
        self.candidates.retain(|candidate| candidate.address != address);
    }

    /// The current value of each candidate, for the list shown while the game runs.
    pub fn current_values(&self, read: impl Fn(u16) -> u8) -> Vec<(u16, Option<i64>)> {
        self.candidates
            .iter()
            .map(|candidate| (candidate.address, self.value_at(candidate.address, &read)))
            .collect()
    }

    /// The size values are read as.
    #[must_use]
    pub fn size(&self) -> ValueSize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn finds_lives_by_unknown_then_known_value() {
        let ram = RefCell::new([0u8; 0x800]);
        ram.borrow_mut()[0x75A] = 2;
        ram.borrow_mut()[0x100] = 2; // a decoy that does not move with the lives
        let read = |a: u16| ram.borrow()[usize::from(a)];
        let mut search = RamSearch::new(&[CPU_RAM], ValueSize::Byte, false);
        search.start(read);
        assert_eq!(search.candidates().len(), 0x800);
        ram.borrow_mut()[0x75A] = 1; // a life lost
        search.filter(Comparison::Equal, Operand::PreviousPlus(-1), read);
        assert_eq!(search.candidates().len(), 1);
        assert_eq!(search.candidates()[0].address, 0x75A);
        assert!(search.undo());
        search.filter(Comparison::Equal, Operand::Value(2), read);
        assert_eq!(search.candidates().iter().map(|c| c.address).collect::<Vec<_>>(), [0x100]);
    }

    #[test]
    fn words_and_bcd() {
        let mut ram = [0u8; 0x800];
        ram[0x10] = 0x34;
        ram[0x11] = 0x12;
        ram[0x20] = 0x42;
        ram[0x21] = 0x4A; // not BCD
        let read = |a: u16| ram[usize::from(a)];
        let mut words = RamSearch::new(&[CPU_RAM], ValueSize::Word, false);
        words.start(read);
        assert_eq!(words.candidates().len(), 0x7FF);
        words.filter(Comparison::Equal, Operand::Value(0x1234), read);
        assert_eq!(words.candidates()[0].address, 0x10);
        let mut bcd = RamSearch::new(&[CPU_RAM], ValueSize::Bcd, false);
        bcd.start(read);
        assert_eq!(bcd.candidates().len(), 0x7FF);
        bcd.filter(Comparison::Equal, Operand::Value(42), read);
        assert_eq!(bcd.candidates().iter().map(|c| c.address).collect::<Vec<_>>(), [0x20]);
    }
}
