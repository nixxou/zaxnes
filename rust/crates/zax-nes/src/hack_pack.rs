//! Hack packs (see `docs/HACK-PACKS.md`): a hack's code, which the widescreen extension shows from
//! memory of its own, and its hooks, a few bytes of the game's code the core checks and replaces
//! in its copy of the PRG-ROM, by component (the widescreen, an aid such as "Invincible"), each
//! checked and turned on apart. The ROM keeps its size and its free room.
//!
//! Little-endian throughout. Format 1:
//!
//! | Field | Contents |
//! |---|---|
//! | magic | `ZXPK` |
//! | format | u8, 1 |
//! | name | u8 length + UTF-8 |
//! | code | u32 length + the code |
//! | components | u8 count, each: u8 length + its name (`widescreen`, or the key of the option that turns it on), u16 hooks, each: u32 offset in the PRG-ROM, u8 length, the original bytes, the new bytes |
//!
//! Format 2, for a pack that should follow other dumps and ROM hacks of its game:
//!
//! | Field | Contents |
//! |---|---|
//! | magic | `ZXPK` |
//! | format | u8, 2 |
//! | name | u8 length + UTF-8 |
//! | region | u8: 0 NTSC, 1 PAL, `$FF` any |
//! | mappers | u8 count (0: any), each a u16 iNES mapper number |
//! | code | u32 length + the code |
//! | components | u8 count, each: name as above, u8 groups, each: u8 length + its name, its label in English then in French (each u8 length + UTF-8: what the hacks screen calls it), u8 count + the indexes of the groups of the component it requires, u16 hooks, each: a place, u8 length, the original bytes, the new bytes; u16 checks, each: a place, u8 length, the bytes expected there |
//!
//! A place is a u8 bank of 16 KiB (bit 7: counted from the end of the PRG-ROM, `$80` its last
//! bank) and a u16 CPU address, of which the low 14 bits are the offset in that bank: the same
//! code finds its hooks in a ROM whose banks were added to (its fixed bank moved to the end).
//! A check is code or data of the game the hack's code relies on (a routine it calls, a table
//! it reads): it is compared, not replaced.
//!
//! A component applies when its first group does (its region and mapper accepted, its hooks and
//! checks all finding their bytes); each other group of it then applies on its own when it finds
//! its bytes and every group it requires applies. Components never depend on each other: the
//! widescreen and an aid are judged apart.

/// The magic a pack starts with (a BPS patch starts with `BPS1`).
pub const MAGIC: &[u8; 4] = b"ZXPK";

/// The component that widens the game (with the advanced mode); the others are aids, each named
/// after the key of the option that turns it on.
pub const WIDESCREEN: &str = "widescreen";

/// Size of the banks a place counts in.
const BANK_SIZE: usize = 0x4000;

/// A pack read from its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HackPack {
    /// Its name.
    pub name: &'static str,
    /// The region it was made for (format 1: any).
    pub region: Region,
    /// The iNES mappers it runs on; empty for any.
    pub mappers: Vec<u16>,
    /// The hack's code.
    pub code: &'static [u8],
    /// Its components, in the pack's order.
    pub components: Vec<Component>,
}

/// The region a pack was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Ntsc,
    Pal,
    Any,
}

/// A part of a pack checked and turned on apart from the others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// [`WIDESCREEN`], or the key of the option that turns it on.
    pub name: &'static str,
    /// Its groups, the first one the component itself (format 1: one group).
    pub groups: Vec<Group>,
}

/// Hooks of a component that apply together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Its name.
    pub name: &'static str,
    /// What the hacks screen calls it, in English and in French (format 1: its name).
    pub label: &'static str,
    pub label_fr: &'static str,
    /// The indexes of the groups of its component it needs.
    pub requires: Vec<usize>,
    /// Its hooks.
    pub hooks: Vec<Hook>,
    /// Bytes of the game it relies on, compared only.
    pub checks: Vec<Check>,
}

/// Where in the PRG-ROM a hook or a check is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// An offset (format 1).
    Offset(usize),
    /// A 16 KiB bank, counted from the start or (`from_end`, 0 the last) from the end, and the
    /// offset in it.
    Bank {
        bank: u8,
        from_end: bool,
        offset: u16,
    },
}

/// A few bytes of the game's code replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hook {
    /// Where.
    pub place: Place,
    /// The game's bytes there.
    pub original: &'static [u8],
    /// What the CPU reads there instead.
    pub new: &'static [u8],
}

/// A few bytes of the game the hack relies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    /// Where.
    pub place: Place,
    /// The bytes expected there.
    pub expected: &'static [u8],
}

/// What a pack needs to know of a ROM before its bytes: read from its iNES header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RomTraits {
    /// Size of its PRG-ROM.
    pub prg_len: usize,
    /// Its iNES mapper, if it has a header.
    pub mapper: Option<u16>,
    /// Its region as the header gives it (or `rom::region_exception`, for a dump whose header is
    /// wrong about it): 0 NTSC, 1 PAL, 2 both, 3 Dendy; `None` without one.
    pub region: Option<u8>,
}

impl RomTraits {
    /// The traits of `image` (iNES header or not), whose data (PRG-ROM then CHR-ROM) is `data`.
    #[must_use]
    pub fn of(image: &[u8], data: &[u8]) -> Self {
        if !image.starts_with(b"NES\x1A") || image.len() < 16 {
            return Self {
                prg_len: data.len(),
                mapper: None,
                region: None,
            };
        }
        let nes2 = image[7] & 0x0C == 0x08;
        let high = if nes2 {
            usize::from(image[9] & 0x0F)
        } else {
            0
        };
        let prg_len = (usize::from(image[4]) | (high << 8)) * BANK_SIZE;
        let mapper = u16::from(image[6] >> 4)
            | u16::from(image[7] & 0xF0)
            | if nes2 {
                u16::from(image[8] & 0x0F) << 8
            } else {
                0
            };
        let region = if nes2 {
            image[12] & 0x03
        } else {
            image[9] & 0x01
        };
        // A dump whose header is known to be wrong about its region (`rom::region_exception`).
        let region = crate::rom::region_exception(crate::builtin_patches::crc32(data))
            .map_or(region, |region| region as u8);
        Self {
            prg_len: prg_len.min(data.len()),
            mapper: Some(mapper),
            region: Some(region),
        }
    }
}

/// Why a group does not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The ROM is of another region.
    Region,
    /// The ROM runs on a mapper the pack is not made for.
    Mapper,
    /// A hook or a check does not find its bytes (another build, a ROM hack that changed them).
    Bytes,
    /// A group it requires does not apply.
    Requires,
}

/// A hook at its offset in a ROM's PRG-ROM: the offset, the original bytes, the new ones.
pub type PlacedHook = (usize, &'static [u8], &'static [u8]);

/// A pack as it applies to one ROM: its components that do, their hooks at their offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedPack {
    /// The pack's name.
    pub name: &'static str,
    /// The hack's code.
    pub code: &'static [u8],
    /// The components that apply, in the pack's order.
    pub components: Vec<AppliedComponent>,
    /// The groups that do not, with why: (component, group, reason).
    pub refused: Vec<(&'static str, &'static str, Refusal)>,
    /// Why no part applies, when the ROM's region or mapper is not one the pack is made for.
    pub gate: Option<Refusal>,
    /// Every group, in the pack's order, with how much of it the ROM has.
    pub parts: Vec<PartReport>,
}

/// How much of a group a ROM has: its hooks and checks that find their bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartReport {
    /// Its component's name and its own.
    pub component: &'static str,
    pub group: &'static str,
    /// What the hacks screen calls it, in English and in French.
    pub label: &'static str,
    pub label_fr: &'static str,
    /// Its hooks and checks that find their bytes, of all of them.
    pub found: usize,
    pub total: usize,
    /// The first that does not, if any.
    pub first_miss: Option<Place>,
    /// Why it does not apply, if it does not.
    pub refusal: Option<Refusal>,
}

/// A component that applies to a ROM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedComponent {
    /// Its name.
    pub name: &'static str,
    /// The names of its groups that apply.
    pub groups: Vec<&'static str>,
    /// Their hooks at their offsets.
    pub hooks: Vec<PlacedHook>,
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
        if reader.take(4)? != MAGIC {
            return None;
        }
        let format = reader.u8()?;
        let pack = match format {
            1 => Self::read_v1(&mut reader)?,
            2 => Self::read_v2(&mut reader)?,
            _ => return None,
        };
        (reader.at == bytes.len()).then_some(pack)
    }

    fn read_v1(reader: &mut Reader) -> Option<Self> {
        let name = reader.text()?;
        let code_length = usize::try_from(reader.u32()?).ok()?;
        let code = reader.take(code_length)?;
        let mut components = Vec::new();
        for _ in 0..reader.u8()? {
            let name = reader.text()?;
            let mut hooks = Vec::new();
            for _ in 0..reader.u16()? {
                let place = Place::Offset(usize::try_from(reader.u32()?).ok()?);
                let length = usize::from(reader.u8()?);
                let original = reader.take(length)?;
                let new = reader.take(length)?;
                hooks.push(Hook {
                    place,
                    original,
                    new,
                });
            }
            components.push(Component {
                name,
                groups: vec![Group {
                    name,
                    label: name,
                    label_fr: name,
                    requires: Vec::new(),
                    hooks,
                    checks: Vec::new(),
                }],
            });
        }
        Some(Self {
            name,
            region: Region::Any,
            mappers: Vec::new(),
            code,
            components,
        })
    }

    fn read_v2(reader: &mut Reader) -> Option<Self> {
        let name = reader.text()?;
        let region = match reader.u8()? {
            0 => Region::Ntsc,
            1 => Region::Pal,
            0xFF => Region::Any,
            _ => return None,
        };
        let mut mappers = Vec::new();
        for _ in 0..reader.u8()? {
            mappers.push(reader.u16()?);
        }
        let code_length = usize::try_from(reader.u32()?).ok()?;
        let code = reader.take(code_length)?;
        let mut components = Vec::new();
        for _ in 0..reader.u8()? {
            let name = reader.text()?;
            let mut groups = Vec::new();
            let group_count = reader.u8()?;
            if group_count == 0 {
                return None;
            }
            for index in 0..group_count {
                let group_name = reader.text()?;
                let label = reader.text()?;
                let label_fr = reader.text()?;
                let mut requires = Vec::new();
                for _ in 0..reader.u8()? {
                    let earlier = reader.u8()?;
                    // Only an earlier group: the order is the order of evaluation.
                    if earlier >= index {
                        return None;
                    }
                    requires.push(usize::from(earlier));
                }
                let mut hooks = Vec::new();
                for _ in 0..reader.u16()? {
                    let place = reader.place()?;
                    let length = usize::from(reader.u8()?);
                    let original = reader.take(length)?;
                    let new = reader.take(length)?;
                    hooks.push(Hook {
                        place,
                        original,
                        new,
                    });
                }
                let mut checks = Vec::new();
                for _ in 0..reader.u16()? {
                    let place = reader.place()?;
                    let length = usize::from(reader.u8()?);
                    let expected = reader.take(length)?;
                    checks.push(Check { place, expected });
                }
                groups.push(Group {
                    name: group_name,
                    label,
                    label_fr,
                    requires,
                    hooks,
                    checks,
                });
            }
            components.push(Component { name, groups });
        }
        Some(Self {
            name,
            region,
            mappers,
            code,
            components,
        })
    }

    /// The component called `name`, whether or not it applies to a ROM.
    #[must_use]
    pub fn component(&self, name: &str) -> Option<&Component> {
        self.components
            .iter()
            .find(|component| component.name == name)
    }

    /// The pack as it applies to the ROM whose data (PRG-ROM then CHR-ROM) is `data`: each
    /// component judged apart, each group of it as the module says.
    #[must_use]
    pub fn resolve(&self, data: &[u8], traits: &RomTraits) -> AppliedPack {
        let mut applied = AppliedPack {
            name: self.name,
            code: self.code,
            components: Vec::new(),
            refused: Vec::new(),
            gate: self.gate(traits),
            parts: Vec::new(),
        };
        let gate = applied.gate;
        for component in &self.components {
            let mut on = vec![false; component.groups.len()];
            let mut hooks = Vec::new();
            let mut groups = Vec::new();
            for (index, group) in component.groups.iter().enumerate() {
                let refusal = if let Some(refusal) = gate {
                    Some(refusal)
                } else if index > 0 && !on[0] || group.requires.iter().any(|&other| !on[other]) {
                    Some(Refusal::Requires)
                } else {
                    group
                        .placed(data, traits.prg_len)
                        .map_or(Some(Refusal::Bytes), |placed| {
                            hooks.extend(placed);
                            None
                        })
                };
                if let Some(refusal) = refusal {
                    applied.refused.push((component.name, group.name, refusal));
                } else {
                    on[index] = true;
                    groups.push(group.name);
                }
                let (found, total, first_miss) = group.tally(data, traits.prg_len);
                applied.parts.push(PartReport {
                    component: component.name,
                    group: group.name,
                    label: group.label,
                    label_fr: group.label_fr,
                    found,
                    total,
                    first_miss,
                    refusal,
                });
            }
            if on[0] {
                applied.components.push(AppliedComponent {
                    name: component.name,
                    groups,
                    hooks,
                });
            }
        }
        applied
    }

    /// Why no part of the pack applies to a ROM of `traits`, if its region or mapper is not one
    /// the pack is made for. A header that does not say leaves it to the bytes.
    fn gate(&self, traits: &RomTraits) -> Option<Refusal> {
        let region_ok = match (self.region, traits.region) {
            (Region::Any, _) | (_, None | Some(2)) => true,
            (Region::Ntsc, Some(region)) => region == 0,
            (Region::Pal, Some(region)) => region == 1 || region == 3,
        };
        if !region_ok {
            return Some(Refusal::Region);
        }
        let mapper_ok = self.mappers.is_empty()
            || traits
                .mapper
                .is_none_or(|mapper| self.mappers.contains(&mapper));
        (!mapper_ok).then_some(Refusal::Mapper)
    }
}

impl Place {
    /// The offset in a PRG-ROM of `prg_len` bytes, if it lies inside it.
    #[must_use]
    pub fn offset(self, prg_len: usize) -> Option<usize> {
        match self {
            Self::Offset(offset) => Some(offset),
            Self::Bank {
                bank,
                from_end,
                offset,
            } => {
                let banks = prg_len / BANK_SIZE;
                let bank = usize::from(bank);
                let bank = if from_end {
                    banks.checked_sub(1 + bank)?
                } else {
                    bank
                };
                (bank < banks).then(|| bank * BANK_SIZE + usize::from(offset) % BANK_SIZE)
            }
        }
    }
}

impl Group {
    /// How many of its hooks and checks find their bytes in `data`, of how many, and the place
    /// of the first that does not.
    fn tally(&self, data: &[u8], prg_len: usize) -> (usize, usize, Option<Place>) {
        let places = self
            .hooks
            .iter()
            .map(|hook| (hook.place, hook.original))
            .chain(
                self.checks
                    .iter()
                    .map(|check| (check.place, check.expected)),
            );
        let mut found = 0;
        let mut total = 0;
        let mut first_miss = None;
        for (place, bytes) in places {
            total += 1;
            let there = place
                .offset(prg_len)
                .and_then(|offset| data.get(offset..offset.checked_add(bytes.len())?));
            if there == Some(bytes) {
                found += 1;
            } else if first_miss.is_none() {
                first_miss = Some(place);
            }
        }
        (found, total, first_miss)
    }

    /// Its hooks at their offsets in `data`, when every hook and check finds its bytes there.
    fn placed(&self, data: &[u8], prg_len: usize) -> Option<Vec<PlacedHook>> {
        let found = |place: Place, bytes: &[u8]| {
            let offset = place.offset(prg_len)?;
            (data.get(offset..offset.checked_add(bytes.len())?) == Some(bytes)).then_some(offset)
        };
        if !self
            .checks
            .iter()
            .all(|check| found(check.place, check.expected).is_some())
        {
            return None;
        }
        self.hooks
            .iter()
            .map(|hook| Some((found(hook.place, hook.original)?, hook.original, hook.new)))
            .collect()
    }
}

impl AppliedPack {
    /// The component called `name`, if it applies.
    #[must_use]
    pub fn component(&self, name: &str) -> Option<&AppliedComponent> {
        self.components
            .iter()
            .find(|component| component.name == name)
    }

    /// Whether any component applies.
    #[must_use]
    pub fn applies(&self) -> bool {
        !self.components.is_empty()
    }
}

impl AppliedComponent {
    /// Writes its hooks into `prg` (`on`), or the game's bytes back (`!on`).
    pub fn apply(&self, prg: &mut [u8], on: bool) {
        for &(offset, original, new) in &self.hooks {
            let bytes = if on { new } else { original };
            if let Some(place) = prg.get_mut(offset..offset + bytes.len()) {
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

    fn place(&mut self) -> Option<Place> {
        let bank = self.u8()?;
        let address = self.u16()?;
        Some(Place::Bank {
            bank: bank & 0x7F,
            from_end: bank & 0x80 != 0,
            offset: address,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANY: RomTraits = RomTraits {
        prg_len: 7,
        mapper: None,
        region: None,
    };

    #[test]
    fn a_format_1_pack_reads_and_applies_component_by_component() {
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
        let applied = pack.resolve(&prg, &ANY);
        assert_eq!(applied.components.len(), 2);
        let widescreen = applied.component(WIDESCREEN).expect("widescreen");
        widescreen.apply(&mut prg, true);
        assert_eq!(prg, [0, 0, 0x56, 0x78, 0, 0x9A, 0]);
        widescreen.apply(&mut prg, false);
        assert_eq!(prg, [0, 0, 0x12, 0x34, 0, 0x9A, 0]);
        // Another byte where the aid's hook goes: the widescreen still applies, apart.
        prg[5] = 0;
        let applied = pack.resolve(&prg, &ANY);
        assert!(applied.component(WIDESCREEN).is_some());
        assert!(applied.component("invincible").is_none());
        assert_eq!(
            applied.refused,
            [("invincible", "invincible", Refusal::Bytes)]
        );
        assert!(HackPack::read(&PACK[..PACK.len() - 1]).is_none());
        assert!(is_pack(PACK) && !is_pack(b"BPS1"));
    }

    /// A format 2 pack: NTSC, mapper 1; component widescreen: group "core" (a hook in the last
    /// bank at `$C002`: 12 -> 56, a check in bank 0 at `$8001`: AA), group "map" requiring it (a
    /// hook in bank 0 at `$8003`: 34 -> 78).
    fn pack_v2() -> HackPack {
        const PACK: &[u8] = &[
            b'Z', b'X', b'P', b'K', 2, 1, b'T', 0, 1, 1, 0, 0, 0, 0, 0, 1, 10, b'w', b'i', b'd',
            b'e', b's', b'c', b'r', b'e', b'e', b'n', 2, // two groups
            4, b'c', b'o', b'r', b'e', 4, b'C', b'o', b'r', b'e', 4, b'B', b'a', b's', b'e', 0, 1,
            0, 0x80, 0x02, 0xC0, 1, 0x12, 0x56, 1, 0, 0x00, 0x01, 0x80, 1, 0xAA, // core
            3, b'm', b'a', b'p', 3, b'M', b'a', b'p', 5, b'C', b'a', b'r', b't', b'e', 1, 0, 1, 0,
            0x00, 0x03, 0x80, 1, 0x34, 0x78, 0, 0, // map
        ];
        HackPack::read(PACK).expect("a format 2 pack")
    }

    fn rom(banks: usize) -> Vec<u8> {
        let mut prg = vec![0; banks * BANK_SIZE];
        prg[1] = 0xAA;
        prg[3] = 0x34;
        prg[(banks - 1) * BANK_SIZE + 2] = 0x12;
        prg
    }

    fn traits(prg: &[u8], mapper: u16, region: u8) -> RomTraits {
        RomTraits {
            prg_len: prg.len(),
            mapper: Some(mapper),
            region: Some(region),
        }
    }

    #[test]
    fn hooks_follow_the_fixed_bank_to_the_end_of_a_bigger_rom() {
        let pack = pack_v2();
        for banks in [8, 16] {
            let prg = rom(banks);
            let applied = pack.resolve(&prg, &traits(&prg, 1, 0));
            let widescreen = applied.component(WIDESCREEN).expect("applies");
            assert_eq!(widescreen.groups, ["core", "map"]);
            assert_eq!(widescreen.hooks[0].0, (banks - 1) * BANK_SIZE + 2);
            assert_eq!(widescreen.hooks[1].0, 3);
        }
    }

    #[test]
    fn a_group_is_left_out_alone_and_the_core_takes_its_dependants() {
        let pack = pack_v2();
        let mut prg = rom(8);
        prg[3] = 0;
        let applied = pack.resolve(&prg, &traits(&prg, 1, 0));
        assert_eq!(
            applied.component(WIDESCREEN).expect("core").groups,
            ["core"]
        );
        assert_eq!(applied.refused, [(WIDESCREEN, "map", Refusal::Bytes)]);
        // The report: each group, how much of it the ROM has, the first place that differs.
        let map = applied.parts[1];
        assert_eq!((map.label, map.label_fr), ("Map", "Carte"));
        assert_eq!((map.found, map.total), (0, 1));
        assert_eq!(
            map.first_miss,
            Some(Place::Bank {
                bank: 0,
                from_end: false,
                offset: 0x8003
            })
        );
        assert_eq!((applied.parts[0].found, applied.parts[0].total), (2, 2));

        // A check that fails: the core goes, and the map with it.
        let mut prg = rom(8);
        prg[1] = 0;
        let applied = pack.resolve(&prg, &traits(&prg, 1, 0));
        assert!(!applied.applies());
        assert_eq!(
            applied.refused,
            [
                (WIDESCREEN, "core", Refusal::Bytes),
                (WIDESCREEN, "map", Refusal::Requires)
            ]
        );
    }

    #[test]
    fn another_region_or_mapper_refuses_the_whole_pack() {
        let pack = pack_v2();
        let prg = rom(8);
        let pal = pack.resolve(&prg, &traits(&prg, 1, 1));
        assert!(!pal.applies());
        assert_eq!(pal.refused[0].2, Refusal::Region);
        let mmc5 = pack.resolve(&prg, &traits(&prg, 5, 0));
        assert!(!mmc5.applies());
        assert_eq!(mmc5.refused[0].2, Refusal::Mapper);
        // A ROM for both regions, or without a header: up to the bytes.
        assert!(pack.resolve(&prg, &traits(&prg, 1, 2)).applies());
        let headerless = RomTraits {
            prg_len: prg.len(),
            mapper: None,
            region: None,
        };
        assert!(pack.resolve(&prg, &headerless).applies());
    }

    #[test]
    fn the_traits_come_from_the_ines_header() {
        let mut image = vec![0u8; 16 + 2 * BANK_SIZE];
        image[..4].copy_from_slice(b"NES\x1A");
        image[4] = 2;
        image[6] = 0x10;
        image[7] = 0x00;
        image[9] = 1;
        let traits = RomTraits::of(&image, &image[16..]);
        assert_eq!(traits.prg_len, 2 * BANK_SIZE);
        assert_eq!(traits.mapper, Some(1));
        assert_eq!(traits.region, Some(1));
    }
}
