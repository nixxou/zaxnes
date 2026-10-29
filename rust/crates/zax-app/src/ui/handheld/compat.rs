//! How a game's hack pack fits a ROM, as the hacks screen tells it (Rust-only, owner request):
//! the ROM the hack is made for, another ROM it fits whole or in part, or one it does not fit,
//! part by part with how much of each the ROM has (its hooks and checks that find their bytes).
//! Worked out when the hacks screen opens, from the ROM's file; it draws nothing.

use zax_nes::builtin_patches::{self, BuiltinPatch};
use zax_nes::hack_pack::{AppliedPack, HackPack, Place, Refusal, Region, RomTraits};

use crate::ui::{keys, strings};

/// How the hack pack of a game fits one ROM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compat {
    /// The ROM the hack is made for: its CRC is the patch's.
    Original,
    /// Another ROM, the pack judged against it.
    Evaluated {
        pack: Box<HackPack>,
        traits: RomTraits,
        applied: Box<AppliedPack>,
    },
}

/// What a line of the detail says, for its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Warning,
    Bad,
    Plain,
}

impl Compat {
    /// The pack of `patch` judged against `image` (iNES header or not); `None` when the patch
    /// is not a hack pack.
    #[must_use]
    pub fn evaluate(patch: &BuiltinPatch, image: &[u8]) -> Option<Self> {
        let pack = HackPack::read(patch.bps)?;
        let data = builtin_patches::rom_data(image);
        let traits = RomTraits::of(image, data);
        let applied = pack.resolve(data, &traits);
        Some(Self::Evaluated {
            pack: Box::new(pack),
            traits,
            applied: Box::new(applied),
        })
    }

    /// Whether the component called `name` applies (on the ROM the hack is made for, all do).
    #[must_use]
    pub fn component_applies(&self, name: &str) -> bool {
        match self {
            Self::Original => true,
            Self::Evaluated { applied, .. } => applied.component(name).is_some(),
        }
    }

    /// The hooks and checks that find their bytes, of all of them, over every part.
    fn score(applied: &AppliedPack) -> (usize, usize) {
        applied.parts.iter().fold((0, 0), |(found, total), part| {
            (found + part.found, total + part.total)
        })
    }

    /// The word the compatibility row shows.
    #[must_use]
    pub fn summary(&self) -> String {
        strings::get(match self {
            Self::Original => keys::HACK_COMPAT_ORIGINAL,
            Self::Evaluated { applied, .. } if !applied.applies() => keys::HACK_COMPAT_NONE,
            Self::Evaluated { applied, .. } if applied.refused.is_empty() => keys::HACK_COMPAT_FULL,
            Self::Evaluated { .. } => keys::HACK_COMPAT_PARTIAL,
        })
    }

    /// The detail, line by line: what the ROM is to the hack, then its parts (for a ROM it fits
    /// in part or not at all), then the ROM the hack is made for (for one it does not fit).
    /// `patch` is the game's patch, `crc` the ROM's data CRC-32.
    #[must_use]
    pub fn lines(&self, patch: &BuiltinPatch, crc: Option<u32>) -> Vec<(String, Tone)> {
        let crc = crc.map_or_else(|| "?".to_owned(), |crc| format!("{crc:08X}"));
        let Self::Evaluated {
            pack,
            traits,
            applied,
        } = self
        else {
            return vec![(
                strings::format(&strings::get(keys::HACKS_COMPAT_ORIGINAL), &[&crc]),
                Tone::Good,
            )];
        };
        let (found, total) = Self::score(applied);
        let (found, total) = (found.to_string(), total.to_string());
        let mut lines = Vec::new();
        if !applied.applies() {
            lines.push((strings::get(keys::HACK_COMPAT_NONE), Tone::Bad));
            lines.push((strings::get(keys::HACKS_COMPAT_NONE), Tone::Plain));
        } else if applied.refused.is_empty() {
            lines.push((
                strings::format(&strings::get(keys::HACKS_COMPAT_FULL), &[&found, &total]),
                Tone::Good,
            ));
            lines.push((
                strings::format(&strings::get(keys::HACKS_COMPAT_FULL_BODY), &[&crc]),
                Tone::Plain,
            ));
            return lines;
        } else {
            lines.push((
                strings::format(&strings::get(keys::HACKS_COMPAT_PARTIAL), &[&found, &total]),
                Tone::Warning,
            ));
            lines.push((strings::get(keys::HACKS_COMPAT_PARTIAL_BODY), Tone::Plain));
        }
        match applied.gate {
            Some(Refusal::Mapper) => {
                let mappers: Vec<String> = pack.mappers.iter().map(|&m| mapper_name(m)).collect();
                let rom = traits.mapper.map_or_else(|| "?".to_owned(), mapper_name);
                lines.push((
                    strings::format(
                        &strings::get(keys::HACKS_COMPAT_MAPPER),
                        &[&rom, &mappers.join(", ")],
                    ),
                    Tone::Bad,
                ));
            }
            Some(Refusal::Region) => {
                let wanted = if pack.region == Region::Pal {
                    "PAL"
                } else {
                    "NTSC"
                };
                let rom = if wanted == "PAL" { "NTSC" } else { "PAL" };
                lines.push((
                    strings::format(&strings::get(keys::HACKS_COMPAT_REGION), &[rom, wanted]),
                    Tone::Bad,
                ));
            }
            _ => lines.extend(applied.parts.iter().map(|part| {
                let label = if strings::language() == strings::UiLanguage::French {
                    part.label_fr
                } else {
                    part.label
                };
                let counts = format!("{}/{}", part.found, part.total);
                match part.refusal {
                    None => (format!("+ {label} {counts}"), Tone::Good),
                    Some(Refusal::Requires) => (
                        strings::format(&strings::get(keys::HACKS_COMPAT_PART_NEEDS), &[label]),
                        Tone::Bad,
                    ),
                    Some(_) => (
                        strings::format(
                            &strings::get(keys::HACKS_COMPAT_PART_DIFFERS),
                            &[
                                label,
                                &counts,
                                &part.first_miss.map_or_else(String::new, place),
                            ],
                        ),
                        Tone::Bad,
                    ),
                }
            })),
        }
        if !applied.applies() {
            lines.push((
                strings::format(
                    &strings::get(keys::HACKS_COMPAT_EXPECTED),
                    &[patch.rom_name, &format!("{:08X}", patch.source_crc)],
                ),
                Tone::Plain,
            ));
        }
        lines
    }
}

/// An iNES mapper by its number, with the name of the common ones.
fn mapper_name(mapper: u16) -> String {
    let name = match mapper {
        0 => " (NROM)",
        1 => " (MMC1)",
        2 => " (UxROM)",
        4 => " (MMC3)",
        5 => " (MMC5)",
        _ => "",
    };
    format!("{mapper}{name}")
}

/// Where a part of the ROM differs: the CPU address in its bank, `$xxxx`.
fn place(place: Place) -> String {
    match place {
        Place::Offset(offset) => format!("@{offset:05X}"),
        Place::Bank { offset, .. } => format!("${offset:04X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A format 2 pack: NTSC, mapper 1; widescreen in "core" (a hook in the last bank at
    /// `$C002`: 12 -> 56) and "map" (a hook in bank 0 at `$8003`: 34 -> 78).
    const PACK: &[u8] = &[
        b'Z', b'X', b'P', b'K', 2, 1, b'T', 0, 1, 1, 0, 0, 0, 0, 0, 1, 10, b'w', b'i', b'd', b'e',
        b's', b'c', b'r', b'e', b'e', b'n', 2, 4, b'c', b'o', b'r', b'e', 4, b'B', b'a', b's',
        b'e', 4, b'B', b'a', b's', b'e', 0, 1, 0, 0x80, 0x02, 0xC0, 1, 0x12, 0x56, 0, 0, 3, b'm',
        b'a', b'p', 3, b'M', b'a', b'p', 5, b'C', b'a', b'r', b't', b'e', 1, 0, 1, 0, 0x00, 0x03,
        0x80, 1, 0x34, 0x78, 0, 0,
    ];

    fn patch() -> BuiltinPatch {
        let mut patch = builtin_patches::PATCHES[0];
        patch.bps = PACK;
        patch.rom_name = "Test (USA)";
        patch.source_crc = 0x1234_5678;
        patch
    }

    /// An iNES image of two 16 KiB banks of mapper `mapper`, region `pal`, with the bytes the
    /// hooks expect, but for those `changed`.
    fn image(mapper: u8, pal: bool, changed: &[usize]) -> Vec<u8> {
        let mut image = vec![0u8; 16 + 2 * 0x4000];
        image[..4].copy_from_slice(b"NES\x1A");
        image[4] = 2;
        image[6] = mapper << 4;
        image[9] = u8::from(pal);
        image[16 + 3] = 0x34;
        image[16 + 0x4000 + 2] = 0x12;
        for &at in changed {
            image[16 + at] ^= 0xFF;
        }
        image
    }

    fn texts(lines: &[(String, Tone)]) -> Vec<&str> {
        lines.iter().map(|(text, _)| text.as_str()).collect()
    }

    #[test]
    fn the_three_cases_say_what_fits_and_why() {
        strings::set_language(strings::UiLanguage::English);
        let patch = patch();

        let whole = Compat::evaluate(&patch, &image(1, false, &[])).unwrap();
        assert_eq!(whole.summary(), "Compatible");
        let lines = whole.lines(&patch, Some(0xABCD_0001));
        assert_eq!(lines[0], ("Compatible - 2/2".to_owned(), Tone::Good));
        assert!(lines[1].0.contains("ABCD0001"));

        let partial = Compat::evaluate(&patch, &image(1, false, &[3])).unwrap();
        assert_eq!(partial.summary(), "Partial");
        assert!(partial.component_applies(hack_pack_widescreen()));
        let lines = partial.lines(&patch, None);
        assert_eq!(lines[0], ("Partial - 1/2".to_owned(), Tone::Warning));
        assert_eq!(
            texts(&lines)[2..],
            ["+ Base 1/1", "x Map 0/1: the game's code differs at $8003"]
        );

        let mapper = Compat::evaluate(&patch, &image(5, false, &[])).unwrap();
        assert_eq!(mapper.summary(), "Incompatible");
        assert!(!mapper.component_applies(hack_pack_widescreen()));
        let lines = mapper.lines(&patch, None);
        assert_eq!(
            texts(&lines),
            [
                "Incompatible",
                "The hack can't apply to this ROM.",
                "Mapper 5 (MMC5): the hack is made for 1 (MMC1).",
                "Expected ROM: Test (USA), CRC 12345678"
            ]
        );

        let region = Compat::evaluate(&patch, &image(1, true, &[])).unwrap();
        assert_eq!(
            texts(&region.lines(&patch, None))[2],
            "PAL ROM: this pack is made for a NTSC ROM."
        );

        let lines = Compat::Original.lines(&patch, Some(0x1234_5678));
        assert_eq!(lines.len(), 1);
        assert!(lines[0].0.contains("12345678"));
    }

    fn hack_pack_widescreen() -> &'static str {
        zax_nes::hack_pack::WIDESCREEN
    }
}
