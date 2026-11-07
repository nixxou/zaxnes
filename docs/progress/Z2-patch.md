# Lot Z2-patch — the one `--ignored` test that has failed since ZR-1

`builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom` fails on
`assert!(apply(&hack).is_none())`, the hack being the Zelda II (USA) image with its **last byte**
flipped. Which side is wrong?

Baseline: `b9586937` (MF-4e-gate's end). No change to emulation, performance or the save-state
format.

## Steps

1. The diagnosis: what the recorded checksum covers, which path takes the hack, and whether that
   is deliberate or an accident. ✔
2. The fix, on whichever side step 1 names.
3. The check: fmt, clippy, the suite, the `--ignored` suite, and `zax-harness packs` on
   `test-originals` before and after.

## Step 1 — the diagnosis

### What the checksum covers: the whole data, the CHR-ROM included

`BuiltinPatch::source_crc` is compared in `apply_with` against `crc32(data)` where `data` is
`image[start..end]` from `data_bounds` — the **PRG-ROM and the CHR-ROM**, without the iNES header,
without a trainer and without an overdump's trailing bytes (the No-Intro CRC). For Zelda II (USA)
that is 262 144 bytes of the 262 160-byte file: 128 KiB PRG + 128 KiB CHR, and the file's last
byte is the data's last byte, a CHR byte.

So the checksum **does** cover the byte the test flips, and the fast path duly misses:

| image | `data_crc` | `for_data_crc` |
| --- | --- | --- |
| the USA dump | `BA322865` | `Zelda II (USA) widescreen` |
| its last byte flipped (CHR) | `CD3518F3` | **none** |

### Which path takes the hack, then

`apply_with` chooses a patch in three ways, in order: the CRC, then `pack_matching` — the pack
whose **signatures** fit, whatever the CRC — then `forced`. The hack is taken by the second.

Measured on the tree (a throwaway probe module, in no commit), one byte of the image flipped at a
time:

| the byte flipped | `data_crc` found | `apply` | the components that apply |
| --- | --- | --- | --- |
| none (the dump) | yes | `Some` | `widescreen[core, enemies, world map, edge spawns]`, `invincible` |
| the last byte (CHR) | **no** | **`Some`** | the same five groups |
| the first byte of the PRG | no | `Some` | the same five groups |
| the first byte of the widescreen's first hook | no | **`None`** | `invincible` only |

The Zelda II (USA) pack hooks **46 places and checks 12**, and every one of them is in the
PRG-ROM: `core` 5 hooks + 2 checks, `enemies` 9 + 2, `world map` 23 + 4, `edge spawns` 9 + 4, and
the aid `invincible` 2 hooks. Not one is in the CHR-ROM. Flipping any single byte of the 46 hooks
makes `apply` refuse in exactly **5 of 46** cases — the `core` group's, which every other group
requires (`HackPack::resolve`: a component applies only when its first group does). Flipping a CHR
byte, or a PRG byte outside those 58 places, changes nothing the pack looks at.

### Deliberate, and load-bearing

Not an accident. Three independent records say so, and a fourth measures what tightening it would
cost.

1. **`15927846` "Hack packs, format 2"**, the commit that made the Zelda II patch a pack, opens
   with "*A pack can now follow other dumps, translations and ROM hacks of its game*", and its
   checked list reads "*the USA Virtual Console ROM (4 bytes of a table differ) takes the whole
   pack by signatures and gives the same RAM as the USA ROM over 12 000 frames*". `b3468920` adds
   "*the Rev 2's translations and Virtual Console had the Europe pack's aid only*".
2. **`docs/HACK-PACKS.md`** states the rule and reports the very experiment this test makes:
   "*by the ROM's CRC first, then by the packs' signatures; a ROM that has them (another revision,
   a ROM hack leaving the hooked code alone) gets the hack, its options and the margin decoder
   **without forcing**. Checked on a Mega Man 6 ROM with **one byte changed outside the hooks**
   (CRC `D9F52C71`): the pack applies*".
3. **`spec/changes/Q-108.md` §2** names the three ways as a three-way design, and marks the
   signature path *verified*, not bypassed:

   | the way | the ROM's standing | |
   | --- | --- | --- |
   | `for_data_crc(crc32(data))` | « ROM d'origine » | *vérifié* |
   | `pack_matching(image, data)` | « Compatible » / « Partielle » | *vérifié* |
   | `forced` | « Forcer l'activation » | **non vérifié** |

   and it makes "Origine **et compatibles**" the **default** level, with "*un patch forcé est le
   seul cas où l'utilisateur a passé outre les vérifications*". (Nothing in the spec proper asks
   for the strict check either: REQ-PROF-064 says there is **no** automatic ROM recognition at
   all — no hash, no CRC, no game database — so the whole CRC/signature business is Rust-side
   design, and Q-108 is its authority.)
4. **What the strict check would cost**, measured: `zax-harness packs G:/3dsen/test-originals`,
   98 original dumps.

   | how the pack is found | ROMs |
   | --- | ---: |
   | by CRC | 45 |
   | **by signatures** | **52** |
   | no pack | 1 (Blaster Master USA Beta) |

   Refusing an image whose whole data does not match a recorded checksum would strip the patch
   from **52 of the 98 originals** — every Virtual Console dump, the GameCube and Switch Online
   builds, the World dumps, the Konami Collector's Series, Final Fantasy (USA) (Beta). That is
   the opposite of the lot's own acceptance criterion ("all the original dumps the patches target
   must still be patched").

### The verdict, and a correction to the brief's premise

**Deliberate: the test is wrong.** The brief offers "flip a byte the checksum covers (a PRG
byte)" — but the checksum covers the CHR-ROM too (table above), so that is not the discriminating
property. What decides is whether the byte is one the **pack** reads: the test must flip a byte of
the widescreen's `core` group, which is PRG by construction.

### Two more assertions in the same block are stale for the same reason

The block dates from `ff049f17` ("Forcing a widescreen patch on another version; ROM CRC over the
data only"), when every patch was a BPS and `apply` really did stand or fall by a checksum. The
Zelda II patch became a pack at `15927846`, and **all 57 built-in patches are packs today** (0
BPS). Three of the block's assertions are BPS-era claims; the test fails on the first and so
never reaches the other two:

| assertion | now | why |
| --- | --- | --- |
| `apply(&hack).is_none()` | `Some` | the signatures take it (above) |
| `source_size(zelda) == Some(262144)` | **`None`** | it reads a BPS header's size number; a pack has none |
| `apply_with(&hack[..len-8192], zelda).is_none()` | **`Some`** | 8 KiB off the CHR leaves the PRG, so the hooks are all still there |

And the block's `forced` assertion passes for the wrong reason: `apply_with(&hack, zelda)` returns
`Some` through `pack_matching`, never through `forced`. For a hack pack **`forced` cannot produce
anything `apply` would not** — `forced` is only consulted when the CRC and the signatures both
fail, and the pack path then still requires the widescreen component to find its bytes. `zax-app`
already encodes exactly that: `ui/handheld/hacks.rs` gates its force button on
`!hack_pack::is_pack(other.bps)`, with the comment "*a hack pack is never forced (its signatures
decide)*".

## Step 2 — the fix

The test was wrong, so the diff is test code only: `builtin_patches.rs`'s `mod tests`, no change to
any function. Four changes and one new test.

1. **A helper**, `zelda2_usa_rom()`, so the two ROM tests share the `ZAXNES_ROMS` lookup and skip
   the same way when the ROM is absent.
2. **The hack the test builds** is now a byte of the code the hack hooks: the first byte of the
   widescreen's first hook, read from the pack **as it resolves against this ROM** rather than
   written as a magic offset, so it follows the hack when `z2pack` moves it. `HackPack::resolve`
   lists a component's hooks group by group, so it belongs to the `core` group — the one every
   other group requires — which is what makes `apply` refuse rather than drop one group.
   Step 1 measured that: 5 of the 46 hooks refuse, and they are `core`'s five.
3. **The refusal is shown to be the widescreen's own** and not the pack going unrecognised: the
   aid `invincible`, judged apart, still applies to the same image (`pack_for` names the Zelda II
   patch, `component(WIDESCREEN)` is `None`, `component("invincible")` is `Some`).
4. **The `forced` assertion is turned round, and the two other BPS-era assertions replaced.**
   Forcing does *not* help and must not: it only leaves a BPS patch's checksums unasked, and a
   pack's hooks are checked byte by byte whatever the CRC — so `apply_with(&hack, Some(zelda))` is
   `None` too, and `source_size(zelda)` is `None` because a pack has no BPS header to read a size
   from. The comment names `zax-app`'s identical gate (`ui/handheld/hacks.rs`, `!is_pack`).
   The first assertion of the test is now the positive one it was missing: the dump is found
   **by its CRC**, so the two tests say which path each image takes.

**The new test**, `a_change_to_the_chr_rom_alone_still_takes_the_zelda_ii_patch`, pins the design
the old assertion contradicted. The last byte of the image, which the **header** proves is a CHR
byte (`data_bounds`'s own size reading, NES 2.0 high bits included — Zelda II's header is NES 2.0),
is flipped; the CRC moves and `for_data_crc` no longer finds the patch; the signatures do, with the
**same components and the same groups as the untouched dump** (`AppliedPack::components` compared
whole), and `apply` returns the patch under its own name at the ROM's own size. Then the same for a
CHR-ROM short of 8 KiB — which is the image the old test's last assertion expected to be refused.
Its doc comment carries the design and its three records (`15927846`, `docs/HACK-PACKS.md`,
Q-108 §2) and the 52-of-98 figure, so the next person to see it has the argument and not a bare
expectation.

Both tests are `#[ignore]`d and gated on `ZAXNES_ROMS`.

| at this step | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy --release -p zax-nes --all-targets -- -D warnings` | clean |
| the nine `builtin_patches` tests, `--include-ignored`, `ZAXNES_ROMS` set | **9 passed, 0 failed** |

## Step 3 — the check

All five gates on the committed tree (`109dff24`), each run in the foreground, one at a time.

| gate | result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --release -p zax-nes --all-targets -- -D warnings` | clean (and with `-p zax-harness` too) |
| `cargo test --release -p zax-nes -p zax-harness` | **925 passed, 0 failed, 9 ignored** |
| the same `--no-fail-fast -- --ignored`, `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set | **9 passed, 0 failed** |
| `zax-harness packs G:/3dsen/test-originals`, before and after | **byte-identical**, md5 `5e98bf22…`, 99 lines |

### The `--ignored` suite: nine of nine

The brief asks for "8 of 8"; the count is 9 because step 2 added one test to the eight. The
previously failing one is among them, and nothing else moved:

| test | before | now |
| --- | --- | --- |
| `builtin_patches::…::the_zelda_ii_patch_applies_to_the_usa_rom` | **failed** since ZR-1 | **ok** |
| `builtin_patches::…::a_change_to_the_chr_rom_alone_still_takes_the_zelda_ii_patch` | — (new) | **ok** |
| `zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack` | ok | ok |
| `req_core_004_apu_reset_needs_a_reset_that_keeps_ram` | ok | ok |
| `req_core_113_dmc_dma_during_read4_is_p3` | ok | ok |
| `the_sprite_dma_and_dmc_dma_collision_is_not_cycle_exact` | ok | ok |
| `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line` | ok | ok |
| `the_cost_of_one_capture_and_one_restore` | ok | ok |
| `golden_pixels_tiles_audio_cycles_and_blargg_match_up_to_frame_1200` | ok | ok |

`925 / 0 / 9` is MF-4e-gate's `925 / 0 / 8` with step 2's one new `#[ignore]`d test: no test was
removed, none was un-ignored, and the default suite's count did not move.

### `packs` before and after: identical, dump by dump

Both runs are the release `zax-harness` of this worktree, the "before" one built from
`b9586937`'s `builtin_patches.rs` restored into the tree for the build and checked back out
immediately (`git status` clean after, bar the untracked `rust/target-mf/`). The two outputs are
identical byte for byte — same md5, so identical on all 98 rows and the summary — which is what
the diff predicts, every hunk of it being inside `#[cfg(test)] mod tests`.

| | before | after |
| --- | ---: | ---: |
| by CRC | 45 | 45 |
| by signatures | 52 | 52 |
| no pack (Blaster Master USA Beta) | 1 | 1 |
| `whole / in part / none` | 97 / 0 / 1 | 97 / 0 / 1 |

The three Zelda II dumps the patches target, unchanged and still found by their own CRC:

```
Zelda II (USA)           widescreen by CRC: widescreen[core,enemies,world map,edge spawns] invincible[invincible]
Zelda II (Europe)        widescreen by CRC: widescreen[core,enemies,world map,edge spawns] invincible[invincible]
Zelda II (Europe) (Rev 2) widescreen by CRC: widescreen[core,enemies,world map,edge spawns] invincible[invincible]
```

Nothing outside `mod tests` changed, so emulation, performance and the save-state format are
untouched by construction; the identical `packs` output is the measurement that says so.

## Outcome

The test was wrong, not the integrity check. The checksum covers the PRG-ROM and the CHR-ROM both
(step 1), and a ROM whose hooked code is intact takes a pack by its signatures on purpose — Q-108
§2's default level, and 52 of the 98 original dumps' only way in. The test now flips a byte the
pack reads, a new test pins the CHR-only case with the design's three records in its doc comment,
and the two other BPS-era assertions of the block are replaced by what a hack pack actually does.
Nine of nine `--ignored` tests pass; the last known failure of the suite is closed.
