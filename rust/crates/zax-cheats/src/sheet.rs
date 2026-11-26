//! Which cheat sheet of a database goes with a ROM.
//!
//! The databases (BeesNES / Mesen, libretro `cht`) name one file per ROM image after its
//! No-Intro name (`Mega Man 3 (USA)`), hacks and homebrews after a free name. The caller gives
//! what it knows of the ROM: the No-Intro name of its data CRC (Zaxnes's game database) and the
//! file name; this module picks the sheet and says how sure the pick is, since a code is made for
//! one image: Game Genie codes patch ROM bytes that move between versions, RAM addresses move
//! less often but do.

/// How a sheet was matched to the ROM, surest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SheetMatch {
    /// The sheet carries the ROM's No-Intro name: made for this image.
    Exact,
    /// The sheet carries the ROM's file name (a hack, a homebrew, a renamed set).
    FileName,
    /// Same title and a region in common, another version (`Rev 1`, `Virtual Console`).
    SameRegion,
    /// Same title, another region: the codes may not work.
    OtherRegion,
}

/// What is known of the ROM to find its sheet.
#[derive(Debug, Clone, Copy, Default)]
pub struct RomNames<'a> {
    /// Its No-Intro name, empty when the database does not know it.
    pub no_intro_name: &'a str,
    /// Its file name without the extension (inside an archive: the entry's).
    pub file_stem: &'a str,
}

/// The title of a No-Intro name: what comes before the first tag, lowercase, with the `GoodSets`'
/// `zz_` sorting prefix and a trailing `, The` taken off.
#[must_use]
pub fn title(name: &str) -> String {
    let untagged = name.split(['(', '[']).next().unwrap_or_default().trim();
    let lower = untagged.to_lowercase();
    let lower = ["zz_", "z_"]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix))
        .map_or(lower.clone(), str::to_owned);
    lower
        .strip_suffix(", the")
        .map_or(lower.clone(), str::to_owned)
}

/// The regions of a No-Intro name, from its first tag (`(USA, Europe)`); `World` counts as
/// every region.
#[must_use]
pub fn regions(name: &str) -> Vec<String> {
    let Some(start) = name.find('(') else {
        return Vec::new();
    };
    let tag = name[start + 1..].split(')').next().unwrap_or_default();
    tag.split(',')
        .map(|region| region.trim().to_lowercase())
        .filter(|r| !r.is_empty())
        .collect()
}

fn share_region(left: &[String], right: &[String]) -> bool {
    let world = |set: &[String]| set.iter().any(|r| r == "world");
    world(left) || world(right) || left.iter().any(|region| right.contains(region))
}

/// The sheet among `sheets` (names without their extension) that goes with the ROM, and how
/// sure it is; `None` when no sheet has its name or its title. Among sheets as sure as each
/// other, the first in `sheets` order wins: give them sorted for a stable pick.
#[must_use]
pub fn choose<'s>(sheets: &[&'s str], rom: RomNames<'_>) -> Option<(&'s str, SheetMatch)> {
    let find = |wanted: &str| {
        (!wanted.is_empty())
            .then(|| {
                sheets
                    .iter()
                    .find(|sheet| sheet.eq_ignore_ascii_case(wanted))
            })
            .flatten()
            .copied()
    };
    if let Some(sheet) = find(rom.no_intro_name) {
        return Some((sheet, SheetMatch::Exact));
    }
    if let Some(sheet) = find(rom.file_stem) {
        return Some((sheet, SheetMatch::FileName));
    }
    let name = if rom.no_intro_name.is_empty() {
        rom.file_stem
    } else {
        rom.no_intro_name
    };
    let wanted_title = title(name);
    if wanted_title.is_empty() {
        return None;
    }
    let wanted_regions = regions(name);
    let mut other = None;
    for sheet in sheets {
        if title(sheet) != wanted_title {
            continue;
        }
        if share_region(&regions(sheet), &wanted_regions) {
            return Some((sheet, SheetMatch::SameRegion));
        }
        other.get_or_insert((*sheet, SheetMatch::OtherRegion));
    }
    other
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHEETS: &[&str] = &[
        "Guardian Legend, The (USA)",
        "Mega Man 3 (Europe)",
        "Mega Man 3 (PC10)",
        "Mega Man 3 (USA)",
        "Mario Adventure 3",
        "Pinball (Japan, USA)",
        "Super Mario Bros. (World)",
    ];

    fn pick(no_intro_name: &str, file_stem: &str) -> Option<(&'static str, SheetMatch)> {
        choose(
            SHEETS,
            RomNames {
                no_intro_name,
                file_stem,
            },
        )
    }

    #[test]
    fn exact_name_first_then_file_name() {
        assert_eq!(
            pick("Mega Man 3 (USA)", "mm3"),
            Some(("Mega Man 3 (USA)", SheetMatch::Exact))
        );
        assert_eq!(
            pick("", "Mario Adventure 3"),
            Some(("Mario Adventure 3", SheetMatch::FileName))
        );
        assert_eq!(
            pick("", "mario adventure 3"),
            Some(("Mario Adventure 3", SheetMatch::FileName))
        );
    }

    #[test]
    fn another_version_says_so() {
        assert_eq!(
            pick("Mega Man 3 (Europe) (Rev 1)", ""),
            Some(("Mega Man 3 (Europe)", SheetMatch::SameRegion))
        );
        assert_eq!(
            pick("Super Mario Bros. (Japan, USA) (Rev 1)", ""),
            Some(("Super Mario Bros. (World)", SheetMatch::SameRegion))
        );
        assert_eq!(
            pick("Pinball (Europe)", ""),
            Some(("Pinball (Japan, USA)", SheetMatch::OtherRegion))
        );
        assert_eq!(
            pick("Guardian Legend, The (Europe)", ""),
            Some(("Guardian Legend, The (USA)", SheetMatch::OtherRegion))
        );
        assert_eq!(pick("Zelda (USA)", "zelda"), None);
    }

    #[test]
    fn titles_and_regions() {
        assert_eq!(
            title("zz_Guardian Legend, The (USA) [!]"),
            "guardian legend"
        );
        assert_eq!(regions("Gumshoe (USA, Europe)"), ["usa", "europe"]);
        assert_eq!(regions("Mario Adventure 3"), Vec::<String>::new());
    }
}
