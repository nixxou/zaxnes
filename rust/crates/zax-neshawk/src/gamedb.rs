//! The two databases NesHawk consults before it looks at the iNES header.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/BootGodDB.cs` (NesCartDB, the
//! `NesCarts.xml` state machine), `NES.BoardSystem.cs`'s `IdentifyFromGameDB` /
//! `IdentifyFromBootGodDB`, and `BizHawk.Emulation.Common/Database/Database.cs`'s record format.
//! MIT; see `LICENSE-BizHawk`.
//!
//! Why this matters for a *timing* port: the board a cartridge gets is chosen from these, and the
//! board is not only a mapper. *Faxanadu* is `NES-SGROM` in NesCartDB with **no** WRAM, where its
//! iNES header implies the generic 8 KiB; MMC3 carts name their revision chip here
//! (`MMC3A`/`MMC3C`…), and that revision changes the IRQ edge. Loading without the databases is a
//! different machine, not a cosmetic difference.

use std::collections::HashMap;
use std::path::Path;

use crate::cart::CartInfo;

/// `Database.cs`'s `CompactGameInfo`, cut down to the fields the NES core reads.
#[derive(Debug, Clone)]
pub struct GameInfo {
    pub hash: String,
    pub name: String,
    pub system: String,
    /// The `k=v;k=v` field `GameInfo.GetOptions` splits.
    pub metadata: String,
    /// `RomStatus.BadDump` — `IsRomStatusBad`.
    pub bad: bool,
}

/// Both databases, loaded from a BizHawk `gamedb` directory.
#[derive(Debug, Default)]
pub struct GameDb {
    /// `Database.DB`, keyed by bare uppercase hex (`Database.FormatHash`).
    game_db: HashMap<String, GameInfo>,
    /// `BootGodDb._sha1Table`, keyed by `SHA1:`-prefixed uppercase hex, as the C# keys it.
    bootgod: HashMap<String, Vec<CartInfo>>,
}

/// `Database.FormatHash`: drop anything up to a `:` and upper-case the rest.
fn format_hash(hash: &str) -> String {
    let bare = match hash.find(':') {
        Some(i) => &hash[i + 1..],
        None => hash,
    };
    bare.to_ascii_uppercase()
}

impl GameDb {
    /// Nothing loaded: every lookup misses, so detection falls through to the iNES header. This
    /// is what a build with no `gamedb` directory gets, and it is **not** NesHawk-equivalent for
    /// any cartridge the databases would have described differently.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.game_db.is_empty() && self.bootgod.is_empty()
    }

    /// How many entries each database holds, for the load report.
    #[must_use]
    pub fn counts(&self) -> (usize, usize) {
        (self.game_db.len(), self.bootgod.len())
    }

    /// Load `gamedb_nes.txt` (with its `#include`s) and `NesCarts.xml` out of `dir`. A file that
    /// is missing is skipped rather than fatal; `Database` itself throws on a missing bundled
    /// include, but a partial database here only costs fidelity, and saying so is better than
    /// refusing to run.
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut db = Self::default();

        // `Database.InitializeWork` on gamedb_nes.txt. NesHawk only ever looks up NES hashes, so
        // the other systems' files are not read.
        db.load_gamedb_file(&dir.join("gamedb_nes.txt"), dir, 0);

        if let Ok(text) = std::fs::read_to_string(dir.join("NesCarts.xml")) {
            db.load_bootgod(&text);
        }

        db
    }

    fn load_gamedb_file(&mut self, path: &Path, root: &Path, depth: u32) {
        if depth > 8 {
            return; // an include cycle; the C# would recurse for ever
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        for line in text.lines() {
            if line.starts_with(';') {
                continue; // comment
            }
            if let Some(rest) = line.strip_prefix("#include") {
                // `LoadDatabase_Escape`. `#includeuser` has no user root here.
                let rest = rest.strip_prefix("user").unwrap_or(rest).trim_start();
                self.load_gamedb_file(&root.join(rest.trim()), root, depth + 1);
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            if let Some(game) = parse_cgi_record(line) {
                self.game_db.insert(game.hash.clone(), game);
            }
        }
    }

    /// `IdentifyFromGameDB`: `Database.CheckDatabase` then the `board=…` options.
    #[must_use]
    pub fn identify_from_game_db(&self, hash: &str) -> Option<CartInfo> {
        let gi = self.game_db.get(&format_hash(hash))?;
        let dict = get_options(&gi.metadata);

        let mut cart = CartInfo {
            game_info_hash: Some(gi.hash.clone()),
            name: Some(gi.name.clone()),
            game_info_bad: gi.bad,
            ..CartInfo::default()
        };

        // "NES gamedb entries must have a board identifier!" — a record without one is skipped
        // rather than thrown, so one bad line cannot stop a load.
        cart.board_type.clone_from(dict.get("board")?);
        if let Some(system) = dict.get("system") {
            cart.system = Some(system.clone());
        }

        cart.prg_size = dict.get("PRG").and_then(|s| s.parse().ok()).unwrap_or(-1);
        cart.chr_size = dict.get("CHR").and_then(|s| s.parse().ok()).unwrap_or(-1);
        cart.vram_size = dict.get("VRAM").and_then(|s| s.parse().ok()).unwrap_or(-1);
        cart.wram_size = dict.get("WRAM").and_then(|s| s.parse().ok()).unwrap_or(-1);

        if let Some(v) = dict.get("PAD_H").and_then(|s| s.parse().ok()) {
            cart.pad_h = v;
        }
        if let Some(v) = dict.get("PAD_V").and_then(|s| s.parse().ok()) {
            cart.pad_v = v;
        }
        if let Some(mir) = dict.get("MIR") {
            if mir == "H" {
                cart.pad_v = 1;
                cart.pad_h = 0;
            } else if mir == "V" {
                cart.pad_h = 1;
                cart.pad_v = 0;
            }
        }

        if dict.contains_key("BAD") {
            cart.bad = true;
        }
        if let Some(mmc3) = dict.get("MMC3") {
            cart.chips.push(mmc3.clone());
        }
        if let Some(pcb) = dict.get("PCB") {
            cart.pcb = Some(pcb.clone());
        }
        if let Some(batt) = dict.get("BATT") {
            cart.wram_battery = batt == "true" || batt == "True";
        }
        if let Some(palette) = dict.get("palette") {
            cart.palette = Some(palette.clone());
        }
        if let Some(vs) = dict.get("vs_security").and_then(|s| s.parse().ok()) {
            cart.vs_security = vs;
        }

        Some(cart)
    }

    /// `IdentifyFromBootGodDB`: the first board listed for the first hash that hits.
    #[must_use]
    pub fn identify_from_bootgod(&self, hashes: &[String]) -> Option<CartInfo> {
        for hash in hashes {
            if let Some(choices) = self.bootgod.get(hash) {
                if !choices.is_empty() {
                    return Some(choices[0].clone());
                }
            }
        }
        None
    }

    /// `BootGodDb`'s `NesCarts.xml` reader, state for state.
    fn load_bootgod(&mut self, text: &str) {
        let mut state = 0;
        let mut curr_cart: Option<CartInfo> = None;
        let mut curr_name: Option<String> = None;

        for node in XmlNodes::new(text) {
            match state {
                0 => {
                    if let XmlNode::Element { name, attrs } = &node {
                        if *name == "game" {
                            curr_name = attr(attrs, "name").map(str::to_string);
                            state = 1;
                        }
                    }
                }
                2 => {
                    if let XmlNode::Element { name, attrs } = &node {
                        if *name == "board" {
                            if let Some(cart) = curr_cart.as_mut() {
                                cart.board_type.clear();
                                cart.board_type.push_str(attr(attrs, "type").unwrap_or(""));
                                cart.pcb = attr(attrs, "pcb").map(str::to_string);
                            }
                            state = 3;
                        }
                    }
                }
                3 => match &node {
                    XmlNode::Element { name, attrs } => {
                        let Some(cart) = curr_cart.as_mut() else {
                            continue;
                        };
                        match *name {
                            "prg" => cart.prg_size += parse_size(attr(attrs, "size")),
                            "chr" => cart.chr_size += parse_size(attr(attrs, "size")),
                            "vram" => cart.vram_size += parse_size(attr(attrs, "size")),
                            "wram" => {
                                cart.wram_size += parse_size(attr(attrs, "size"));
                                if attr(attrs, "battery").is_some() {
                                    cart.wram_battery = true;
                                }
                            }
                            "pad" => {
                                cart.pad_h =
                                    attr(attrs, "h").and_then(|s| s.parse().ok()).unwrap_or(0);
                                cart.pad_v =
                                    attr(attrs, "v").and_then(|s| s.parse().ok()).unwrap_or(0);
                            }
                            "chip" => {
                                if let Some(t) = attr(attrs, "type") {
                                    cart.chips.push(t.to_string());
                                }
                                if attr(attrs, "battery").is_some() {
                                    cart.wram_battery = true;
                                }
                            }
                            _ => {}
                        }
                    }
                    XmlNode::EndElement { name } => {
                        if *name == "board" {
                            state = 4;
                        }
                    }
                },
                4 => {
                    if let XmlNode::EndElement { name } = &node {
                        if *name == "cartridge" {
                            if let Some(cart) = curr_cart.take() {
                                let key = cart.sha1.clone().unwrap_or_default();
                                self.bootgod.entry(key).or_default().push(cart);
                            }
                            state = 5;
                        }
                    }
                }
                _ => {
                    // states 1 and 5 share an arm in the C# too
                    match &node {
                        XmlNode::Element { name, attrs } => {
                            if *name == "cartridge" {
                                curr_cart = Some(CartInfo {
                                    system: attr(attrs, "system").map(str::to_string),
                                    sha1: Some(format!(
                                        "SHA1:{}",
                                        attr(attrs, "sha1").unwrap_or("")
                                    )),
                                    name: curr_name.clone(),
                                    ..CartInfo::default()
                                });
                                state = 2;
                            }
                        }
                        XmlNode::EndElement { name } => {
                            if *name == "game" {
                                curr_name = None;
                                state = 0;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `BootGodDb.ParseSize`: a decimal count of KiB with a trailing `k`.
fn parse_size(str: Option<&str>) -> i32 {
    let Some(str) = str else { return 0 };
    let mut temp: i32 = 0;
    let bytes = str.as_bytes();
    let len = bytes.len().saturating_sub(1);
    for &b in &bytes[..len] {
        temp = temp.wrapping_mul(10);
        temp = temp.wrapping_add(i32::from(b) - i32::from(b'0'));
    }
    temp
}

/// `Database.ParseCGIRecord`, trimmed to the fields used here.
fn parse_cgi_record(line: &str) -> Option<GameInfo> {
    let mut iter = line.split('\t');
    let hash = format_hash(iter.next()?);
    let status = iter.next().unwrap_or("").trim();
    let name = iter.next().unwrap_or("").to_string();
    let system = iter.next().unwrap_or("").to_string();
    // the 5th field is "rarely populated; possibly genre or just a remark"
    let _ = iter.next();
    let metadata = iter.next().unwrap_or("").to_string();

    Some(GameInfo {
        hash,
        name,
        system,
        metadata,
        // `RomStatus.BadDump` is "B" or "V"; `IsRomStatusBad` is true for those only.
        bad: status == "B" || status == "V",
    })
}

/// `GameInfo.GetOptions`: `k=v` pairs separated by `;`.
fn get_options(metadata: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in metadata.split(';') {
        if part.is_empty() {
            continue;
        }
        let mut kv = part.splitn(2, '=');
        let k = kv.next().unwrap_or("");
        let v = kv.next().unwrap_or("");
        if !k.is_empty() {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

// ---------------------------------------------------------------------- a minimal XML scanner

/// Just enough of `XmlReader` for `NesCarts.xml`: elements with their attributes, and end tags.
/// A self-closing tag yields one [`XmlNode::Element`] and no end tag, which is what `XmlReader`
/// reports and what the state machine above was written against.
enum XmlNode<'a> {
    Element {
        name: &'a str,
        attrs: Vec<(&'a str, &'a str)>,
    },
    EndElement {
        name: &'a str,
    },
}

fn attr<'a>(attrs: &[(&'a str, &'a str)], name: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

struct XmlNodes<'a> {
    rest: &'a str,
}

impl<'a> XmlNodes<'a> {
    fn new(text: &'a str) -> Self {
        Self { rest: text }
    }
}

impl<'a> Iterator for XmlNodes<'a> {
    type Item = XmlNode<'a>;

    fn next(&mut self) -> Option<XmlNode<'a>> {
        loop {
            let start = self.rest.find('<')?;
            let after = &self.rest[start + 1..];

            // skip declarations, comments and CDATA
            if after.starts_with('?') || after.starts_with('!') {
                let end = after.find('>')?;
                self.rest = &after[end + 1..];
                continue;
            }

            let end = after.find('>')?;
            let tag = &after[..end];
            self.rest = &after[end + 1..];

            if let Some(name) = tag.strip_prefix('/') {
                return Some(XmlNode::EndElement {
                    name: name.trim_end(),
                });
            }

            let tag = tag.strip_suffix('/').unwrap_or(tag);
            let mut it = tag.splitn(2, |c: char| c.is_ascii_whitespace());
            let name = it.next().unwrap_or("");
            let attrs = it.next().map(parse_attrs).unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            return Some(XmlNode::Element { name, attrs });
        }
    }
}

fn parse_attrs(mut s: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    loop {
        s = s.trim_start();
        let Some(eq) = s.find('=') else { break };
        let key = s[..eq].trim_end();
        let after = &s[eq + 1..];
        let after = after.trim_start();
        let quote = match after.as_bytes().first() {
            Some(&b'"') => b'"',
            Some(&b'\'') => b'\'',
            _ => break,
        };
        let body = &after[1..];
        let Some(close) = body.find(quote as char) else {
            break;
        };
        out.push((key, &body[..close]));
        s = &body[close + 1..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bootgod_state_machine_reads_a_cartridge() {
        let xml = r#"
        <database>
         <game name="Hoshi no Kirby">
          <cartridge system="Famicom" sha1="BB71">
           <board type="HVC-TKROM" pcb="HVC-TKROM-03" mapper="4">
            <prg size="512k"/>
            <chr size="256k"/>
            <wram size="8k" battery="1"/>
            <chip type="MMC3C"/>
           </board>
          </cartridge>
         </game>
        </database>"#;
        let mut db = GameDb::empty();
        db.load_bootgod(xml);
        let cart = db
            .identify_from_bootgod(&["SHA1:BB71".to_string()])
            .expect("the cartridge is found by its prefixed hash");
        assert_eq!(cart.board_type, "HVC-TKROM");
        assert_eq!(cart.prg_size, 512);
        assert_eq!(cart.chr_size, 256);
        assert_eq!(cart.wram_size, 8);
        assert!(cart.wram_battery);
        assert_eq!(cart.chips, vec!["MMC3C".to_string()]);
        assert_eq!(cart.system.as_deref(), Some("Famicom"));
    }

    #[test]
    fn a_gamedb_record_yields_the_board_options() {
        let line = "sha1:77241A57\t\tTsuri\tNES\t\tboard=MAPPER002;PRG=128;CHR=0;WRAM=8;VRAM=8;PAD_H=1;PAD_V=0";
        let rec = parse_cgi_record(line).expect("the record parses");
        assert_eq!(rec.hash, "77241A57");
        let mut db = GameDb::empty();
        db.game_db.insert(rec.hash.clone(), rec);
        let cart = db
            .identify_from_game_db("sha1:77241a57")
            .expect("the hash is matched case-insensitively");
        assert_eq!(cart.board_type, "MAPPER002");
        assert_eq!(cart.prg_size, 128);
        assert_eq!(cart.wram_size, 8);
        assert_eq!(cart.pad_h, 1);
    }

    #[test]
    fn a_size_without_its_k_suffix_is_read_as_the_cs_reads_it() {
        // ParseSize drops the last character, whatever it is.
        assert_eq!(parse_size(Some("512k")), 512);
        assert_eq!(parse_size(Some("8k")), 8);
    }
}
