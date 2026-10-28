//! Hack packs (see `docs/HACK-PACKS.md`): a hack's code, which the widescreen extension's PRG
//! window shows from memory of its own, and its hooks, a few bytes of the game's code the core
//! checks and replaces in its copy of the PRG-ROM, by component (the widescreen, an aid such as
//! "Invincible"), each turned on apart. The ROM keeps its size and its free room.
//!
//! Little-endian throughout:
//!
//! | Field | Contents |
//! |---|---|
//! | magic | `ZXPK` |
//! | format | u8, 1 |
//! | name | u8 length + UTF-8 |
//! | code | u32 length + the code, assembled for `$5800` |
//! | components | u8 count, each: u8 length + its name (`widescreen`, or the key of the option that turns it on), u16 hooks, each: u32 offset in the PRG-ROM, u8 length, the original bytes, the new bytes |

/// The magic a pack starts with (a BPS patch starts with `BPS1`).
pub const MAGIC: &[u8; 4] = b"ZXPK";

/// The format this core reads.
const FORMAT: u8 = 1;

/// The component that widens the game (with the advanced mode); the others are aids, each named
/// after the key of the option that turns it on.
pub const WIDESCREEN: &str = "widescreen";

/// A pack read from its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HackPack {
    /// Its name.
    pub name: &'static str,
    /// The hack's code, assembled for `$5800`.
    pub code: &'static [u8],
    /// Its components, in the pack's order.
    pub components: Vec<Component>,
}

/// A part of a pack turned on apart from the others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// [`WIDESCREEN`], or the key of the option that turns it on.
    pub name: &'static str,
    /// Its hooks.
    pub hooks: Vec<Hook>,
}

/// A few bytes of the game's code replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hook {
    /// Offset in the PRG-ROM.
    pub offset: usize,
    /// The game's bytes there.
    pub original: &'static [u8],
    /// What the CPU reads there instead.
    pub new: &'static [u8],
}

/// Whether `bytes` are a pack (and not a BPS patch).
#[must_use]
pub fn is_pack(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

impl HackPack {
    /// Reads a pack; `None` when `bytes` are not one this core reads.
    #[must_use]
    pub fn read(bytes: &'static [u8]) -> Option<Self> {
        let mut reader = Reader { bytes, at: 0 };
        if reader.take(4)? != MAGIC || reader.u8()? != FORMAT {
            return None;
        }
        let name = reader.text()?;
        let code_length = usize::try_from(reader.u32()?).ok()?;
        let code = reader.take(code_length)?;
        let mut components = Vec::new();
        for _ in 0..reader.u8()? {
            let name = reader.text()?;
            let mut hooks = Vec::new();
            for _ in 0..reader.u16()? {
                let offset = usize::try_from(reader.u32()?).ok()?;
                let length = usize::from(reader.u8()?);
                let original = reader.take(length)?;
                let new = reader.take(length)?;
                hooks.push(Hook {
                    offset,
                    original,
                    new,
                });
            }
            components.push(Component { name, hooks });
        }
        (reader.at == bytes.len()).then_some(Self {
            name,
            code,
            components,
        })
    }

    /// Whether every hook of every component finds its original bytes in `prg`: a pack applies
    /// to a PRG-ROM only whole (its hooks depend on each other).
    #[must_use]
    pub fn matches(&self, prg: &[u8]) -> bool {
        self.components
            .iter()
            .flat_map(|component| &component.hooks)
            .all(|hook| {
                prg.get(hook.offset..hook.offset + hook.original.len()) == Some(hook.original)
            })
    }

    /// The component called `name`.
    #[must_use]
    pub fn component(&self, name: &str) -> Option<&Component> {
        self.components
            .iter()
            .find(|component| component.name == name)
    }
}

impl Component {
    /// Writes its hooks into `prg` (`on`), or the game's bytes back (`!on`).
    pub fn apply(&self, prg: &mut [u8], on: bool) {
        for hook in &self.hooks {
            let bytes = if on { hook.new } else { hook.original };
            if let Some(place) = prg.get_mut(hook.offset..hook.offset + bytes.len()) {
                place.copy_from_slice(bytes);
            }
        }
    }
}

struct Reader {
    bytes: &'static [u8],
    at: usize,
}

impl Reader {
    fn take(&mut self, length: usize) -> Option<&'static [u8]> {
        let bytes = self.bytes.get(self.at..self.at.checked_add(length)?)?;
        self.at += length;
        Some(bytes)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.take(2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        let bytes = self.take(4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn text(&mut self) -> Option<&'static str> {
        let length = usize::from(self.u8()?);
        std::str::from_utf8(self.take(length)?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_reads_and_applies_whole_or_not_at_all() {
        // Code "AB"; component widescreen: one hook at 2 (12 34 -> 56 78); invincible: at 5.
        const PACK: &[u8] = &[
            b'Z', b'X', b'P', b'K', 1, 1, b'T', 2, 0, 0, 0, b'A', b'B', 2, 10, b'w', b'i', b'd',
            b'e', b's', b'c', b'r', b'e', b'e', b'n', 1, 0, 2, 0, 0, 0, 2, 0x12, 0x34, 0x56, 0x78,
            10, b'i', b'n', b'v', b'i', b'n', b'c', b'i', b'b', b'l', b'e', 1, 0, 5, 0, 0, 0, 1,
            0x9A, 0xBC,
        ];
        let pack = HackPack::read(PACK).expect("a pack");
        assert_eq!(pack.name, "T");
        assert_eq!(pack.code, b"AB");
        let mut prg = vec![0, 0, 0x12, 0x34, 0, 0x9A, 0];
        assert!(pack.matches(&prg));
        let widescreen = pack.component(WIDESCREEN).expect("widescreen");
        widescreen.apply(&mut prg, true);
        assert_eq!(prg, [0, 0, 0x56, 0x78, 0, 0x9A, 0]);
        widescreen.apply(&mut prg, false);
        assert_eq!(prg, [0, 0, 0x12, 0x34, 0, 0x9A, 0]);
        // Another byte where a hook goes: the pack does not apply.
        prg[5] = 0;
        assert!(!pack.matches(&prg));
        assert!(HackPack::read(&PACK[..PACK.len() - 1]).is_none());
        assert!(is_pack(PACK) && !is_pack(b"BPS1"));
    }
}
