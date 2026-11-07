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
Zelda II patch became a pack at `15927846`, and **all 51 built-in patches are packs today** (0
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

To come: the test, on the side step 1 names.

## Step 3 — the check

To come.
