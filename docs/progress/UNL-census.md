# Lot UNL-census — which unlicensed and homebrew boards carry games worth preserving

**Baseline**: `8496c503`, the end of MF-6a3. **This lot writes no emulator code** and changes no
`.rs` file; its deliverables are `docs/review/UNL-census.md` and this file.

**The owner's rules that moved the goalposts** (2026-11-06): a board is **never left out for
performance** any more — a costly board ships behind its own Cargo feature, off by default
(`docs/review/BOARD-FEATURES.md`). So `MF-6-inventory.md` §6 item 1's argument ("each board costs
`Board` a variant and `irq_asserted` an arm") no longer excludes anything. The only three reasons
left are **no reliable public documentation**, **no title**, and a **development cost out of
proportion to what it preserves**. The content test: games with **original content** are worth
preserving; compilations, multicarts of licensed titles and FDS-to-cartridge conversions are not.

## The steps

| k | step | state |
| ---: | --- | --- |
| 0 | the plan, this file, and the exact number set the census has to cover | **done** |
| 1 | census part A — §3.3's single-publisher boards (Sachen, Kaiser, Waixing, TXC, NTDEC, JY, Whirlwind, FFE) | **done** |
| 2 | census part B — §3.4's explicitly listed numbers | **done** |
| 3 | census part C — the MMC3 clones of §3.2, mapper 14 among them | to do |
| 4 | census part D — the homebrew numbers, and the census's category totals | to do |
| 5 | the category-A detail: documentation quality, hooks, effort per number | to do |
| 6 | the plan of the MF-UNL lots, and the A boards excluded under the three remaining reasons | to do |
| 7 | the check — `cargo fmt --check`, `cargo test --release -p zax-nes -p zax-harness` | to do |

## Step 0 — the number set, and an arithmetic correction to the inventory

The brief asks for "every number of §3.3 and §3.4 (182 numbers), plus 14, 29, 31, 111, 218 and the
other homebrew numbers of §3.1's note". Enumerating the inventory's own lists gives a different
count, and the census is built on the enumeration rather than on the subtotal:

| source | numbers | count |
| --- | --- | ---: |
| §3.3, eight publisher groups | Sachen 14, Kaiser 11, Waixing 8, TXC 7, NTDEC 7, JY 5, Whirlwind 5, FFE 3 | **60** |
| §3.4, the explicit five-line list | `38 … 530` | **83** |
| §3.2's MMC3-clone row, which §3.4 adds by reference | `12 … 366` | **48** |
| §3.1's homebrew note + the one homebrew row in §3.1's table | 29, 31, 111, 218, 284, 323, 324, 682 | **8** |
| | **total, distinct** | **199** |

**Where 182 comes from, and why it is wrong.** §3.4's header says "The remaining 134 numbers" and
its list ends "+ the 48 MMC3 clones listed in §3.2"; 134 + 48 = 182, which is the figure §6 item 1
and this brief both carry. But the list under that header holds **83** numbers, not 134 — and 134
is not §3.3's 60 plus anything either. So §3.4's "134" is a miscount of its own list, 182 is that
miscount plus the 48 clones, and the true size of §3.3 + §3.4 is **60 + 83 + 48 = 191**. Mapper 14
is already in the clone row, so the brief's "plus 14" adds no row; 29, 31, 111, 218 and §3.1's
remaining four (284, 323, 324, 682) add eight. **199 rows.** No number is in two groups — checked
pairwise across all four lists.

### The categories, as the brief defines them

* **A — original content**: at least one game that exists on no other board this build supports.
* **B — derivative**: translations, hacks or ports whose original runs on a supported board.
* **C — compilation**: multicarts of games available elsewhere.
* **D — FDS conversion**.
* **E — no title / no documentation**.

A number can be truthfully describable by two of these (a board carrying one original game and
nine multicarts is A, not C). The rule applied throughout: **the highest category any one of its
titles reaches**, because the question the census answers is what shipping the board would
preserve. Where that promotion is the whole reason for the category, the row says so.

### What "supported" means for the A test

`SUPPORTED_MAPPERS` (`rust/crates/zax-nes/src/cartridges/registry.rs:43-137`) by default:
0-5, 7, 9-11, 13, 15, 16, 18, 19, 21-28, 30, 32, 33, 34, 37, 41, 47, 48, 64-67, 69, 70, 71, 73,
75, 76, 78, 85, 87, 88, 89, 93, 94, 95, 97, 101, 105, 116, 118, 119, 140, 152, 154, 155, 156, 158,
180, 184, 185, 206, 210, 225, 228, 232 — **69 numbers**, 45 of them with every board feature off.

## Step 1 — §3.3's 60 numbers: 29 A, 4 B, 10 C, 12 D, 5 E

`docs/review/UNL-census.md` §0 (scope, categories, sources) and §1 (the eight publisher groups).
The findings that change a decision:

* **29 of the 60 numbers §6 excluded in one sentence are category A.** The densest clusters are
  **Sachen** (13 of 14) and **J.Y. Company** (5 of 5).
* **36, 132, 172, 173 are one ASIC** — the TXC/Idea-Tek `05-00002-010`, the same `JV001` that
  Sachen's **136** and **147** carry. Six numbers, ~20 original titles, one board with wiring
  parameters: the best effort-to-titles ratio in the document.
* **298 (UNL-TF1201) is a `Vrc2Vrc4` `Vrc4b` with one documented difference** ($F003 acknowledges
  only), for one game that exists on no other NES board. A `const` parameter in the `McAcc` manner.
* **Kaiser (11) and Whirlwind Manu (5) hold no A at all** — seven and five FDS conversions
  respectively, two bootlegs of licensed cartridges, one pirate multicart, one no-title number.
  §3.3's own notes were right and the content rule retires both groups.
* **176 (FK23C), which §6 named as revisitable, is C**: every title on its page is a multicart or a
  plug-and-play. It is out on the content rule *and* on cost, which is two of the three reasons.
* **Four of §3.3's eight publisher labels are wrong**, recorded in §1: 171 is BBK, not Kaiser (and
  NESdev and Mesen disagree about what the number even is); 164 is Dōngdá/Yànchéng, not Wàixīng;
  61 is NTDEC/GS, 172 Super Mega, 173 Idea-Tek, 286 Benshieng.
* **NesCartDB answers HTTP 403 to this environment**, so no per-number cartridge count could be
  refreshed. Recorded in §0 as the census's one open gap.

## Step 2 — §3.4's 83 numbers: 15 A, 7 B, 47 C, 6 D, 8 E

`docs/review/UNL-census.md` §2. The findings that change a decision:

* **Fifty-three of the 83 are multicarts, bootlegs or FDS conversions.** §3.4's own description was
  right; the owner's content rule reaches the same verdict without any performance argument.
* **Fifteen are A**, and they are fewer boards than numbers: **79 + 113 + 146 + 148 are one
  circuit** (NESdev gives them one page, "NINA-003-006"; Mesen returns `Nina03_06` for three of
  them), and **83 + 264 are one page and one Cony/Yoko ASIC**.
* **79 is AVE's whole catalogue** — Wikipedia's `/wiki/American_Video_Entertainment` lists 19
  NES-exclusive AVE titles — behind a board whose documentation is a full table and whose hook list
  is empty. One register at $4100.
* **163 (Nánjīng) survives the content rule**: seven titles with catalogue numbers, including the
  Chinese *Harvest Moon*, *Diablo* and *Final Fantasy IV*. It needs a PPU-read hook, battery
  PRG-RAM and, on submapper 1, ADPCM expansion audio.
* **83 (Cony/Yoko) is the find of this step**: four ports of Capcom/SNK/Neo Geo games, a full
  register table, and an IRQ counter with *two* selectable clock sources (CPU M2 or PPU A12).
* **Six of the fifteen A numbers have a stub for documentation** — 144, 170, 216, 240, 244, 521 —
  so the "no reliable public documentation" reason bites inside category A, not only outside it.
  170 is one sentence with no register and no mask.
* **301 has no documentation of its own**: NESdev redirects `NES 2.0 Mapper 301` to INES Mapper
  242, a different board, while Mesen builds `Bmc8157`. Recorded as E on that contradiction.
* **177 and 200 stay unsupported**, so `req_core_102_any_other_mapper_is_refused` keeps three of
  its four numbers (6, 177, 200) and holds unchanged.
