# FCEUX-48 — `.fm2` playback on FCEUX (FCEUX-4) and the heap overflow (FCEUX-8)

Branch `feat/fceux-engine`, worktree `cleanroom/wt-fceux2`, base `c9c73033` (lot FCEUX-23).
Fork `cleanroom/fceux-zax`, branch `zax`, base `61c7b080`.

Design note: `docs/review/FCEUX-ENGINE.md` §5 (`.fm2`), §7 (the overflow and the vetting), §9 (the
lots). Previous lots: `docs/progress/FCEUX-1.md`, `FCEUX-6.md`, `FCEUX-23.md`.

Why this lot exists, in the owner's words: in the profile editor, **play a complete TAS (`.fm2`)
and edit the 3dn profile on the fly**. `.fm2` movies only stay in sync on the engine that recorded
them, and the 3D on FCEUX matches Zaxnes's wherever the two machines agree (§6 of the design note).

## Steps

- [x] 1. The FCEUX-8 fix in the fork.
- [ ] 2. The validated-games marker.
- [ ] 3. `.fm2` playback on FCEUX, in the core.
- [ ] 4. `.fm2` in the application.
- [ ] 5. The app with a real movie, end to end.
- [ ] 6. Docs.
- [ ] 7. Final pass.

## The environment this lot ran in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-fx` |
| `TMP` / `TEMP` | `<worktree>/rust/target-fx/tmp` |
| the fork, MSVC | `cleanroom/fceux-zax/build-msvc` |
| the fork, ASan | `cleanroom/fceux-zax/build-asan`, a WSL (Ubuntu 24.04, g++ 13.3) tree with `-fsanitize=address -g` |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |
| SDL2 | `--features zax-app/bundled` |

---

## Step 1 — the CHR-RAM heap overflow (FCEUX-8)

Fork commit `c9f388c3`. `src/ines.cpp` and `zax/README.md`; the whole diagnosis is in the fork's
`zax/README.md` § *The CHR-RAM overflow* and the design note's rewritten §7.

### The brief was right that §7's diagnosis was wrong, and in both directions

| §7 said | what it is |
|---|---|
| "a one-byte heap overflow" | ASAN reports the **first** byte outside the buffer. The whole of `$0400-$1FFF` is outside it |
| "the apparent one-line fix is to pass `mCHRRAMSize` to `SetupCartCHRMapping`" | that leaves `CHRmask8` at 0 for an 8 KiB chip, so the window is unchanged and **the overflow stands** |

The cause is `setchr8()` (`src/cart.cpp:262-274`): all eight 1 KiB `VPage` slots are pointed at one
**linear** 8 KiB window from the chip base, `VPageR[x] = &CHRptr[r][V << 13]` with no per-slot
offset, while `SetupCartCHRMapping` computes `CHRmask8 = (size >> 13) - 1` and **clamps it to 0**
when that underflows (`src/cart.cpp:133,138`). A chip under 8 KiB therefore gets an 8 KiB window
over a smaller buffer. `setchr4` and `setchr2` clamp the same way below 4 KiB and 2 KiB; at or above
the window the mask is sound even for a non-power-of-two size, since `V <= (size >> n) - 1` gives
`(V + 1) << n <= size`.

`oam3.nes`: header `4E 45 53 1A 01 00 70 08 … 04 …` — mapper 7 (ANROM, which calls `setchr8`),
NES 2.0, byte 11 `0x04` so `vram_size = 64 << 4 = 1024` and `battery_vram_size = 0`.

### The fix

`src/ines.cpp:1217-1234`: pad the allocation to **8 KiB**, keep `FCEU_MemoryRand(VROM, CHRRAMSize)`
on the **declared** size, `memset` the padding to zero, and keep passing the declared size to
`SetupCartCHRMapping` and `AddExState`. The three things that keeps intact, each of which a
different-looking fix would break:

* **movies.** `FCEU_MemoryRand` (`src/fceu.cpp:990-1020`) draws one `xoroshiro128plus` value per
  byte under `RAMInitOption` 3, so filling the declared size consumes exactly the values it
  consumed before. §5 of the design note is about a movie's header carrying that option, so this is
  not a hypothetical;
* **savestates.** `AddExState(VROM, CHRRAMSize, 0, "CHRR")` keeps the block at its old length;
* `iNES_ExecPower` (`src/ines.cpp:78-79`) re-fills the declared size on every power-on and leaves
  the padding alone, so the padding stays zero for the life of the cartridge.

It carries **no `// ZAX:` marker**, deliberately: it is a bug in FCEUX rather than an adaptation for
Zaxnes, and the owner is to submit it upstream as it stands. `zax/README.md`'s change table says so
and now lists five `src/` files rather than four.

**What the fix is not.** It makes the access safe, not accurate: above the declared size the PPU
sees the zero padding, where `zax-nes` mirrors (`cartridges/mod.rs`'s `wrap_offset`, modulo the
buffer length — the hardware's own behaviour for a 1 KiB chip). Making `setchr8` mirror per slot is
a change to a function every board uses and an emulation decision; it is not needed to close the
memory error and was not taken.

### Verified

| | |
|---|---|
| `oam3.nes`, ASan, 3 000 frames | **clean**. Without the fix it faults at about frame 300 — reproduced first, so the test is known to be able to fail |
| `oam3.nes` on MSVC and on ASan/Linux | `frames=3000 pixels=f2810f098f36bc51 emphasis=0f7bf6886a9c2325 audio=3d5558af9c5e2ee5 samples=2395928`, `state: hint=85250 written=72137` — **the same on both**, which it could not be before: the overflow read whatever the allocator had put after the block |
| `nestest.nes`, 60 frames, both builds | `pixels=e455081e6a3eefe9 emphasis=d395e456e4e72325 audio=7fc10a9ce4ed8fcd samples=47917`, `state: hint=93304 written=79296` — the fork's recorded digests, unchanged |
| the MSVC library rebuilt | `zax\build-msvc.cmd`, ok (the script's `vswhere.exe` line prints "not recognized" and falls through to its own search; harmless and pre-existing) |
| every `zax-fceux` test, feature on, `--include-ignored` | **35 passed**, 0 failed, 0 ignored — 15 `fceux_core_tests` (including `the_tile_capture_does_not_change_what_fceux_draws`) and 20 `fceux_engine_tests` |

### Everywhere else CHR-RAM is registered

`grep -rn SetupCartCHRMapping src/` gives 42 sites. Every one with a size under 8 KiB was read; the
table is in the fork's `zax/README.md`. The summary:

* **the UNIF path** (`src/unif.cpp:535-547`) takes 8, 16, 32, 128 or 256 KiB — **8 KiB is the
  floor**, so it has no sub-window chip at all, and `UNIFGI`'s `memset(UNIFchrrama, 0, 8192)` on
  power (`:575`) is in bounds for the same reason. Nothing to fix;
* **mapper 77** (`src/boards/77.cpp:69`) is the one sub-8 KiB chip addressed by anything wider than
  `setchr1r`: 6 KiB, with `setchr2r(0x10, 0x0800, 2)` and `setchr4r(0x10, 0x1000, 0)`. Both fit —
  `CHRmask2 = 2` puts the 2 KiB window at `[4096, 6144)` and `CHRmask4 = 0` the 4 KiB one at
  `[0, 4096)`. Safe, and **not** touched;
* every other sub-8 KiB chip — MMC3 mappers 74, 165, 191, 192, 194, 195 at 2 or 4 KiB, mappers 252
  and 253 at 2 KiB, and `n106`'s `NTARAM` at 2 KiB — is addressed **only** by `setchr1r`, whose
  mask is 1 or 3 and never clamped. Safe;
* `fds.cpp` and the rest are 8 KiB or more, or a window of an already larger buffer.

### The two engines on `oam3.nes`, after the fix

`zax-harness fceux-tiles`, frames 60 / 300 / 1 200 / 3 000:

    1 of 1 ROMs identical at every compared frame

1 024 records at every frame, **identical in order**, pictures identical. `first_pixel_diff` is 3,
so the two machines do part early and come back together; the padding-against-mirroring difference
of the fix is therefore **latent** on this ROM rather than absent, and stated that way rather than
claimed to be harmless in general.

And the scope of the whole change, measured rather than argued: of the **446** iNES images in
`assets-free/test-roms`, `demo/roms` and `test-originals`, **exactly one** declares a NES 2.0
CHR-RAM under 8 KiB — `oam3.nes`, 1 024 bytes, mapper 7. No other ROM in reach can have moved.

---
