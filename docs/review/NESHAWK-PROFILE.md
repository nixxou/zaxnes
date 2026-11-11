# The two timing profiles of the 2A03, and how to add an item to the NesHawk one

Lot **NESHAWK-PROFILE**, branch `feat/neshawk-profile`, base `b604dc83` (lot EVAL-SYNC-4). The
working is `docs/progress/NESHAWK-PROFILE.md`; the result in context is
`docs/review/MOVIE-SYNC.md` §4f.

## What this is, in three sentences

Zaxnes's core is generic over a **timing profile**: `zax_nes::Standard` is the 2A03 it has always
emulated, and `zax_nes::NesHawkCompat` is one that makes BizHawk NesHawk's choices where they
differ. The `.bk2` player selects the second automatically when a movie's `Core` header says
`NesHawk`, and nothing else ever does. With it, the *Hoshi no Kirby* `.bk2` that parted at frame
926 replays with **no live difference from BizHawk's own work RAM at any of the 1 100 frames the
reference covers**.

## What it is **not**

**The NesHawk profile is not the more accurate chip.** On every item below, Zaxnes's own profile
agrees with Mesen 2 and with the nesdev wiki and the NesHawk one does not; the NesHawk profile
**fails five public test ROMs and eight `AccuracyCoin` rows that the standard profile passes**
(`docs/progress/NESHAWK-PROFILE.md` step 3 has the table, each failure traced to the item that
causes it). It exists so that a movie recorded against those choices can be replayed against them,
which is a different job from being right.

So: never make it the default, never offer it as a "better accuracy" setting, and never fix a game
with it. If a game needs it, the bug is somewhere else.

## The items, each with its source

Every item is an associated constant of `zax_nes::TimingProfile`, read at exactly the sites listed.
`Standard`'s value is `false` everywhere, and the `else` arm of every site is the code this core had
before the profile existed — not an equivalent of it, the same source.

| | what the NesHawk profile does | read from | sites |
|---|---|---|---|
| `DMC_HALT_ON_REQUEST_CYCLE` (1a) | the halt of a DMC reload lands on the **request cycle itself**, because `DMCUnit.Run()` lowers `RDY` and `RunOneFirst()` runs before `cpu.ExecuteOne()` | `APU.cs` `DMCUnit.Run`, `NES.Core.cs` `RunCpuOne` | `nes_bus.rs` `poll_dmc_dma` |
| `DMC_FETCH_HAS_TWO_DUMMIES` (1b) | a fetch is a **fixed four** cycles — halt, two re-reads, fetch — with no get/put alignment, there being no `% 2` on that path | `APU.cs` `dmc_dma_countdown = 3`, `RunDMCHaltFetch` | `nes_bus.rs` `promote_dmc_wait`, `dmc_fetch_cycles`, `run_dma`, `dma_cycle`, `is_a_lone_dmc_fetch` |
| `DMC_LENGTH_LAGS_THE_FETCH` (2) | the sample length, the loop and the IRQ land **three cycles after** the fetch, so `$4015` bit 4 stays set across them | `APU.cs` `DMCUnit.Fetch` (`dmc_reload_countdown = 3`) and `RunOneLast` | `apu/dmc_channel.rs` `supply_byte`, `step_reload_countdown`, `quiet_clocks`, the savestate; `apu/mod.rs` `step_cpu_cycle` |
| `DMC_ENABLE_WRITES_MID_SAMPLE` (3) | a `$4015` enable with the buffer empty starts a write-style fetch **even mid-sample**, delayed by the **DMC timer's** parity and three cycles long | `APU.cs` `set_lenctr_en` (the `!sample_buffer_filled` sibling test) and `Run`'s `else` branch | `apu/dmc_channel.rs` `set_enabled`; `nes_bus.rs` `after_apu_write` |

**BizHawk is MIT** and nothing is copied from it: each item is written from the behaviour its source
describes, and the table names the function so that the reading can be checked. Mesen 2 is GPL-3.0
and the standard arms, which agree with it, carry their attribution where they already did.

**Which item closed *Kirby*** was measured rather than assumed, one item at a time: **1a** does it
(1b then takes the residue from 17 frames to 8), **3** does it independently, and **2 is exactly
inert on that movie**. The table is `docs/progress/NESHAWK-PROFILE.md` step 5.

## How to add an item — the whole recipe

1. **Read it, and write down where.** The item goes in the progress note of your lot with the
   NesHawk file and function it came from and what the lines mean. An item nobody can check is not
   an item.
2. **Add one associated constant** to `TimingProfile` in `crates/zax-nes/src/timing_profile.rs`,
   with the doc comment carrying the source; `false` (or "as today") on `Standard`, the NesHawk
   value on `NesHawkCompat`.
3. **Read it at the fewest sites you can**, each as `if P::YOUR_ITEM { … } else { … }` where the
   `else` arm is the existing code **unchanged**. That is what makes the standard profile free: the
   constant folds and the `else` arm is all that is emitted. Do not add a runtime field that the
   standard profile has to test.
4. **New state goes in the struct, and in the savestate only under the profile.**
   `DmcChannel::reload_countdown` is the example: `if P::DMC_LENGTH_LAGS_THE_FETCH {
   writer.write_i32(…) }`, so a standard block stays byte for byte what it always was and
   `NesCore::STATE_VERSION` does not move.
5. **If the state changes between cycles the APU may batch over**, make `quiet_clocks` answer 0
   while it is pending — again behind the constant. Forgetting this is the one error that produces a
   difference only on some games.
6. **Write a differential test** in `crates/zax-nes/tests/neshawk_profile_tests.rs`: measure the
   same thing on both profiles and assert the gap the item predicts. A test that pins only the
   NesHawk number cannot tell a regression of the standard profile from a change of yours.
7. **Re-run the two suites that say you did not break the standard path**, and no others count:
   * `golden-core` over the originals must stay
     `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`;
   * `test-roms --profile standard` over `dmc_dma_during_read4`, `sprdma_and_dmc_dma`, `apu_test`,
     `dmc_tests` and `AccuracyCoin` must be unchanged — 24 pass, 6 manual, 128/147.
   Then record what `--profile neshawk` scores and which item moved it.
8. **Measure the movie.** `movie-sync --profile neshawk` with `--ram-ref` against a per-frame
   reference, and `--profile standard` beside it so the two columns are from the same command.

### Two traps that cost this lot time, so they are written down

* **A hot entry point must be concrete per profile.** Rust monomorphises a generic function in the
  crate that *calls* it, and a generic body with one call site is inlined into its wrapper. A
  generic `run_frame` cost **+3.8 %** on Super Mario Bros. 3 for both reasons together, with no
  emulation changed. `NesCoreOf<Standard>::run_frame` and `NesCoreOf<NesHawkCompat>::run_frame` are
  therefore declared separately, each a one-line forward to an `#[inline(never)]` generic body. A
  new hot entry point needs the same treatment; a cold one does not. `nes_core.rs` carries the
  measurements beside the code.
* **A default type parameter does not help callers.** Rust applies a type parameter's default when
  it resolves a type and **not** when it resolves a path through one, so `NesCore::STATE_VERSION`
  and `NesCore::new()` both stop compiling behind `NesCore<P = Standard>`. The generic structs are
  therefore `NesCoreOf<P>` and `NesBusOf<P>`, with `pub type NesCore = NesCoreOf<Standard>` and
  `pub type NesBus = NesBusOf<Standard>` — plain aliases to a concrete instantiation, which resolve
  everywhere. That is why not one line outside `zax-nes` had to change.

## Where the profile is chosen, and the one place to change it

| | |
|---|---|
| the rule | `MovieFile::recorded_profile` in `zax-harness/src/movie_file.rs`: a `.bk2` whose `Core` header is exactly `NesHawk`, and nothing else — not `QuickNes`, not an `.fm2`, not a header with no `Core` line |
| the override | `--profile standard\|neshawk` on `movie-sync`, `frame-trace` and `test-roms`, through `zax-harness/src/profile_choice.rs`. An unknown value is a usage error, never a silent fall-back |
| where it is read | `movie_sync.rs` `zaxnes_machine` (one `match`, then a `Box<dyn Machine>`) and `frame_trace.rs` `run` → `trace_on::<P>` |
| in a report | `ZaxnesMachine::name()` answers `zaxnes` or `zaxnes/neshawk`, so no figure can be mistaken for the other chip's |
| in a savestate | `TimingProfile::STATE_CORE_NAME` — `"Zaxnes"` or `"Zaxnes-NesHawk"` — at the head of the block, so a block of one profile is refused by the other with the message REQ-CORE-120 fixes, and `STATE_VERSION` never moved |

**The application cannot open a `.bk2` at all** (`grep -rn bk2 crates/zax-app/src/` finds nothing;
the movie dialog admits `.fm2`, `.fcm` and `.mv`), so there is nothing in it that selects a profile
today. The day it gains a `.bk2` importer, `Engine` is already implemented for both profiles and the
rule above is one function call — and the switch must be **for that playback only**, never written
to the game's own engine setting, and said on screen, which is what lot FCEUX-48 settled for `.fm2`.

## The cost of having two chips

| | |
|---|---|
| speed, standard profile | within the lot's +1 % budget: `bench-core` on five ROMs read −3.0 %, −3.0 %, −2.2 %, +0.5 %, +0.6 % against the parent commit |
| behaviour, standard profile | `golden-core` over 98 ROMs × 1 200 frames **byte-identical**; `AccuracyCoin` 128/147 unchanged; 24 of 24 DMC/DMA ROMs unchanged |
| memory | none: a profile is a zero-sized type, and the two cores are the same size |
| binary | **+147 408 bytes of `.text`** (+3.0 %) for a whole second CPU, bus, APU and DMA — a fourteenth of the 2 MB that would have put it behind a feature, so it is behind none |
| savestates | unchanged for the standard profile, to the byte and to the version |
