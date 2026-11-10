# Playing TAS movies on Zaxnes — where its engine parts from one, and whether `.bk2` is the answer

Lot **EVAL-SYNC**, branch `eval/bk2`, base `432492c4` (lot FCEUX-48, which gave the application a
second engine and `.fm2` playback on it). This is an **evaluation**: it measures and explains, and
it changes no emulation in `zax-nes`.

The working, with every command, is `docs/progress/EVAL-SYNC.md`. The instrument is
`zax-harness movie-sync`.

---

## The short version

The owner's two questions had two answers that point the same way.

**Where does Zaxnes part from an `.fm2`, and why?** On all three movies measured, at a point where
**FCEUX is the engine that is wrong** and Zaxnes is the one that passes the public test. FCEUX
takes the NMI one CPU cycle early, its vblank period is too short with the background off, and its
MMC3 scanline counter is wrong in five named ways — and these TASes were recorded on it, so they
depend on all of that. There is **no Zaxnes emulation error to fix** among the three.

**Can Zaxnes play a `.bk2`, and on which engine?** Yes, on **Zaxnes's own engine**, and the
evidence is as strong as evidence gets short of hardware: the 83 218-frame *Faxanadu* movie
recorded on `NesHawk` replays on Zaxnes **identically to `BizHawk`'s own run** at every frame
compared, on both dumps, and Zaxnes's lag count over those 83 218 frames is **13 398** — exactly
`BizHawk`'s own counter. The same movie on FCEUX never leaves the title screen.

**So a third engine would buy nothing.** `NesHawk` would be integrated to play `.bk2` movies
accurately, and Zaxnes already does. The recommendation is §6.

---

## 1. The instrument

`zax-harness movie-sync <movie.fm2|.bk2> <rom.nes> [--engine zaxnes|fceux|both] …` runs the two
machines **in lockstep** over a movie's inputs and compares, every frame:

* the **2 KiB of work RAM** — where a game keeps everything it has decided, and what parts first;
* the **picture**, one NES colour index a pixel;
* whether the frame was **input-sensitive**.

and **frame 0** before all of them: the machine at power-on, work RAM only, because a movie whose
two machines start from a different 2 KiB has already lost.

It reports the first frame at which anything differs, what differs, **the first difference outside
the stack page** (memory the game is using rather than a leftover below the stack pointer), **the
first difference that would be seen**, and then keeps going to say whether the two come back
together.

### The one measure that had to be invented

Neither engine publishes a lag flag. FCEUX keeps one and no C API call reaches it — and the fork
is read-only in this lot — and `zax-nes` keeps none at all, which this lot may not change. So the
frame is **measured** instead: run it twice from the same state, once with the movie's word and
once with its complement; the frame is *input-sensitive* when the two outcomes differ. A frame the
game spent without reading `$4016` cannot react to either word.

It is applied **identically to both machines**, which is the property a comparison needs and the
one that would be lost by using FCEUX's own flag on one side and something else on the other. It
was written up as an operational stand-in that could differ from a real `lagFlag` in a known
direction — and §4 is where that caution was put to the test against `BizHawk`'s own counter and
came back exact.

The probe runs **before** the real frame, and the real frame runs last, because
`NesCore::restore_core_state` deliberately leaves the picture out of the block; a probe run after
the real one would overwrite the picture it was meant to be checking. That contract is a test on
each engine separately, FCEUX's state block being a different mechanism from `CoreState`.

### What it was shown to be able to do

| | |
|---|---|
| say "nothing differs" when nothing does | `dmc_tests/buffer_retained.nes`, 121 frames, with inputs, with two resets and with the probe on |
| **find a difference**, which is the half a comparison tool usually cannot demonstrate | `other/oam3.nes`, where the two engines part by frame 10 |
| reproduce FCEUX's own movie mode by feeding it inputs directly | the three real `.fm2` movies, 33 000 to 124 000 frames, **the same final picture digest both ways** |

Finding a ROM for the first row took two sweeps, and the figures say how rare agreement between
the two engines is: of **290** public ROMs, **25** have no pixel difference at any compared frame,
and of those 25 only **five** also have no work-RAM difference. `nestest.nes` — the obvious
candidate — is in neither set.

---

## 2. The four `.fm2` movies

| | Kabuki ×2 | Metal Force | Kirby |
|---|---|---|---|
| frames | 33 306 | 47 454 | 123 937 |
| mapper | 4 (MMC3) | 156 (Daou) | 4 (MMC3) |
| dump the movie names | `Kabuki - Quantum Fighter (USA)` | `Metal Force [Korea][Unl][!]` | **`Kirby's Adventure [USA][!!]`**, not `[Rev 1]` |
| **first divergence** | frame **2** | frame **5** | frame **3** |
| what it is | 1 byte, `$01FB`, in the stack page | **1 pixel** at (231,79) | **82 RAM bytes**, 68 outside the stack |
| first difference outside the stack | frame **258** | the same frame | the same frame |
| first difference that would be **seen** | frame **787** | the same frame | frame **4** |
| compared frames differing visibly | 32 395 of 33 304 | **5** of 47 449 | 123 646 of 123 934 |
| final picture, the two engines | different | **byte-identical** | different |
| **FCEUX plays it** | to the end (clock 33 306/33 306) | to the end | to the end |
| **Zaxnes plays it** | to frame ~787 | **to the end** | to frame 3 |

The two *Kabuki* files are the same movie: their 33 306 frame lines are byte-identical and the
only difference is one header line, `FDS 1`.

Digests are not enough to say whether a run is still a game, so the pictures were written out and
read:

| | Zaxnes at the last frame | FCEUX at the last frame |
|---|---|---|
| Kabuki | still in stage 1, score 100 | the **ending** |
| Metal Force | **the same picture as FCEUX, byte for byte** | — |
| Kirby | a grass level, 1 life | the **endgame** star corridor, 8 lives |

Kabuki's Zaxnes run also has a **380-frame stretch in which the picture does not change at all**
from frame 29 187, which FCEUX's run does not — the shape of a run that has stopped being played.

---

## 3. Each divergence, with its class

Classes: **(a)** a Zaxnes emulation error, **(b)** an FCEUX inaccuracy the movie relies on,
**(c)** a setup difference fixable in the player.

### The (c) items were settled before the first measurement

Four of them, found by reading the headers: the region, the power-on work RAM, the per-frame reset
commands, and a `FDS 1` claim on a cartridge game. The instrument honours all four on both
machines.

The one worth stating on its own: **an `.fm2` with no `RAMInitOption` is not a movie that wants
zeroes.** `MovieData`'s constructor defaults the option to 0 and `FCEUI_LoadMovie` copies it into
FCEUX's global; option 0 is FCEUX's `00 00 00 00 FF FF FF FF` pattern. `zax-nes` powers on all
zero (REQ-CORE-004). All four movies state no option, so all four ask for the pattern.

What that is worth was measured, not assumed — the same movie on Zaxnes alone under three fills:

| | `--ram zero` | `--ram default` | `--ram ff` |
|---|---|---|---|
| Metal Force | `3de90a647172d715` | the same | the same |
| Kabuki | `eb96fb9d5d6fc02d` | the same | the same |
| **Kirby** | `34f0ab4acf20a98d` | **`107d27b697e2aa5c`** | **`04648ed5dc58ca23`** |

One game of three reads its uninitialised RAM. And even for that one it is not the cause of the
divergence: with both machines on `zero`, on `ff` and on `default`, they part at frame 3 in all
three.

### Metal Force — class (b), and the mildest possible one

Five frames of 47 454 differ, **the work RAM is identical at every one of them**, and the last
picture of the movie is byte-identical on the two engines.

Frame 5 is a single pixel at (231,79). The screen is flat — frame 4 is 61 440 pixels of one colour
and frame 6 is 61 440 of another — so rendering is off and what is on screen is the backdrop;
scanline 79 is where it changes, and the two engines put the change **one dot apart**. The other
four (20 313, 26 278, 26 298, 29 735) are 6 to 15 pixels on **row 0 only**.

A sub-scanline difference of PPU phase. FCEUX fails `ppu_vbl_nmi/09-even_odd_frames` and
`10-even_odd_timing` — the skipped dot of the odd pre-render line, which is a one-dot shift of a
whole frame — and Zaxnes passes both.

**Nothing to fix on either side.** The movie syncs.

### Kabuki — class (b): an interrupt taken a few cycles apart

| frame | what differs |
|---|---|
| 1 | nothing |
| **2** | one byte, `$01FB`: `56` / `53`, in the stack page |
| 3-8 | nothing at all |
| 9 | `3A F9` written at `$01E7`/`$01E8` on Zaxnes and `$01E5`/`$01E6` on FCEUX — **two bytes apart** |
| 258 | the first bytes outside the stack: `$0018`, `$0072`-`$0079` |
| 787 | the first pixels, five of them |
| 1 167 | the last frame the two pictures are ever the same |

Frame 2's byte is in the stack, which is the most informative place it could be: the bytes an
interrupt pushes are the return address and the flags, and a byte differing by **3** is a return
address three bytes further along the same loop — **the interrupt was taken at a different point**,
a couple of CPU cycles apart. Frame 9 says how much: the same value two bytes lower on FCEUX is
one return address, so the two stacks are at a different depth. Frames 3 to 8 being identical
rules out a drift: it is one event.

**Why FCEUX.** `ppu_vbl_nmi/05-nmi_timing` prints the cycle at which the NMI was taken:
**Zaxnes `00 4`, passed; FCEUX `00 3`, Failed #1** — one CPU cycle apart, Zaxnes's being right.
Its whole family goes the same way: `01-vbl_basics`, `02-vbl_set_time`, `03-vbl_clear_time`,
`06-suppression`, `07-nmi_on_timing`, `08-nmi_off_timing`, all passed by Zaxnes and failed by
FCEUX. *Kabuki* is MMC3, so a second interrupt is in play and FCEUX is wrong about that one too:
it fails **all five** of `mmc3_test` 1 to 5 and all five of `mmc3_test_2`, each with a named
message, and Zaxnes passes every one.

Which of the two fires at frame 2 is not settled here — saying so would need cycle-level tracing
this lot has no hook for — and it does not change the class, both being mechanisms Zaxnes passes
the public test on and FCEUX fails it on.

**What Zaxnes would have to do to sync this TAS:** take the NMI one cycle early and clock the MMC3
counter wrongly in five specific ways. That is, fail ten public tests it currently passes.

### Kirby — class (b): the vblank the game waits for at boot

Frames 1 and 2 identical; **frame 3 differs in 82 bytes**, 68 of them outside the stack. Which
side holds what settles the direction: at almost every differing address **FCEUX still holds the
power-on `00`/`FF` pattern and Zaxnes holds data** — `$0627`-`$0636` is `0E` sixteen times on
Zaxnes and the untouched pattern on FCEUX — and a frame later Zaxnes has filled `$0200`-`$02FD`
with `F0` (the sprite table pushed off-screen) and written `KIRBY` at `$0517`, and FCEUX has done
neither. Zaxnes is **one step further through the same initialisation**.

Two things it is not, both measured: not the power-on RAM (frame 3 under all three fills), and not
a whole-frame offset of the boot — `--warm-up` was added to the instrument for exactly that
question, and `fceux:1`, `fceux:2`, `fceux:3` and `zaxnes:1` all fail to move the first divergence
past frame 3. Whatever is apart is apart **inside** a frame.

*Kirby* waits for vblank at boot with the background off, and that is what the public test names:
`ppu_vbl_nmi/01-vbl_basics` — Zaxnes **`Passed`**, FCEUX **`VBL period is too short with BG off`,
Failed #7**. Both screens were captured and read rather than taken from a table.

### No class (a) was found

On the 113 public ROMs that report a verdict on both engines, Zaxnes passes 95 and FCEUX 58, and
the count passed by FCEUX and **not** by Zaxnes is **0**. Every mechanism the three divergences
land on is one of the 37 Zaxnes passes and FCEUX fails. That is not a proof that Zaxnes has no
error here; it is the strongest evidence available short of hardware, and it points the other way.

The one public test both fail is `mmc3_test/6-MMC6`, where Zaxnes's failure is a **decision**:
REQ-CORE-100 pins mapper 4 to MMC3 revision B and `6-MMC6` is the revision A half of a
contradictory pair (AMD-061, Q-114).

**No Q ticket was opened.** `SPEC-GUIDE.md`'s rule is that a ticket is for a blocking ambiguity in
the spec; nothing here is that, and the findings are about FCEUX, which the spec does not govern.

### The one class (c) left, and it is in the application

`zax-app`'s own `.fm2` path on the Zaxnes engine does not honour the header the way the harness
does:

| the header says | `zax-app`'s `play_fm2` (`emulation/host/movies.rs`) |
|---|---|
| `RAMInitOption` / `RAMInitSeed` | **ignored** — the core powers on all zero |
| a reset command on a frame | **not replayed**; it warns and goes on (`movies.rs:174-183`) |
| `palFlag` | **compared and warned about**, not applied (`movies.rs:158-164`) |

Fixing it would **not** make either movie sync, the reason they do not being class (b); it would
remove a second, avoidable reason. The size: the pattern and the region are about four lines,
written between `session.reset(…)` and `core.save_state(true)` (`movies.rs:184-187`). The resets
are the real work — `MovieRun` has no notion of a command on a frame. It is a change to the
application's movie player and not to `zax-nes`, and it is **left for the owner to schedule**
rather than made here, this lot being an evaluation.

---

## 4. `.bk2`

### What is supported

A reader built from the **public description of the format on TASVideos** and from the file's own
self-description. No `BizHawk` source of any kind was consulted.

The input log **describes itself** — the `LogKey:` line names every button in the order the frame
lines write them, `#` starting a new pipe-separated field — so the reader builds its mapping from
the key rather than hard-coding a layout. That is the design's whole point: a `.bk2`'s pad order
is `Up Down Left Right Start Select B A` and an `.fm2`'s is `RLDUTSBA` with bit 7 first, and a
reader that assumed either would replay the wrong buttons while looking perfectly healthy. The
test is 400 frames of a sequence that presses every button, written in both formats, which must
leave the machine on the same picture — with a silent movie of the same length as the control.

`movie-sync` takes a `.bk2` wherever it takes an `.fm2`.

### The limits, each a refusal with the thing named

| the movie needs | the message names |
|---|---|
| a `CoreState` member | "anchored to a savestate … a state of another emulator's core" |
| `StartsFromSavestate` / `StartsFromSaveRam` | which of the two |
| `Platform` other than `NES` | the system |
| a `RegionOverride` other than 0 | the value, and that the public description does not say what it means |
| non-empty `BoardProperties` | the properties, by key |
| `Famicom`, a port that is not `ControllerNES`/`UnpluggedNES`, anything on `FamicomExpPort` | the device |
| a `LogKey` naming anything else | the names |

The format description is explicit that a movie is savestate-anchored by the **existence** of
`CoreState` whatever the header says, so both are checked and the member wins.

**One thing is deliberately not refused:** a `SHA1` that matches neither the whole image nor the
data after the iNES header is reported **unverified**, not "another ROM". The description says the
header carries "SHA1 (also CRC32 or MD5)" and does not say what the digest covers — and the one
real `.bk2` in hand proves the point: its `9A8BF108…` matches neither *Faxanadu* dump and the
movie nevertheless replays on both of them exactly.

Two things the format does not carry, each a note rather than a guess: **no power-on work RAM**
(the machine's own all-zero fill is used, REQ-CORE-004) and **no region** (taken from the ROM's
own iNES header, `InesHeader`'s rule).

### What the real one did

The owner supplied `invariel-faxanadu.bk2` (`NesHawk`, 83 218 frames), both *Faxanadu* dumps, and
— which turned out to matter more — **`BizHawk`'s own run of it**: screenshots at six frames per
dump and a log with a running lag count.

| frame | Zaxnes against `BizHawk`'s own screenshot |
|---|---|
| 1 000, 10 000, 30 000, 60 000, 83 000, 83 218 | **identical, at all six** |

Measured rather than eyeballed: `BizHawk` writes 256×224 and its colour table is not Zaxnes's, so
the comparison takes this run's rows 8 to 231 against the reference's 224 and asks whether **every
colour of one maps to exactly one colour of the other**. It does, on 9 to 14 distinct colours —
and a single pixel of different content would break it, that colour then having two images.

It holds on **both dumps**, whose `BizHawk` runs differ from frame 100 onwards and whose Zaxnes
runs end on different pictures. So it is not a movie that would look right whatever the machine
did.

**And the lag count.** `BizHawk`'s own counter reads **13 398**; the probe finds 69 820 of 83 218
frames input-sensitive, which is **13 398** insensitive. The same number — so Zaxnes agrees with
`NesHawk` about *which* frames the game polled the pad on, and the probe this lot had to invent is
exact where a real emulator's counter can check it.

**FCEUX cannot play this movie at all.** The two engines part at frame 3, the picture at frame 12
(61 435 pixels of 61 440), and FCEUX's screen then **does not change for 10 768 frames**: it never
leaves the title screen, the movie's Start not taking on it, and 83 000 frames later it is sitting
on the password screen with nonsense typed in. The same class (b) finding as §3, from the other
side. `--movie-mode` is not even an option: FCEUX's movie code reads `.fm2` and nothing else.

---

## 5. What is still unknown

Said plainly, because the result in §4 is strong enough to be worth bounding.

* **One `.bk2`, one game, one core.** *Faxanadu* is MMC1 with CHR-RAM. A `.bk2` of an MMC3 game,
  or of a board with its own IRQ, has not been tried, and the first divergence of such a movie
  would be the interesting one. The command is in the progress note and costs about two minutes
  per movie.
* **`QuickNes` is not `NesHawk`.** `BizHawk` ships two NES cores; this movie names `NesHawk` in
  its header. A `.bk2` recorded on `QuickNes` would be a movie recorded on a *fast* core and
  should be expected to behave like the `.fm2` movies do, not like this one. The reader records
  the core's name in a note for that reason.
* **A savestate-anchored `.bk2` cannot be played at all**, by anything here, and no amount of work
  on this side changes that: the anchor is a state of another emulator's core.
* **`RegionOverride`** non-zero is refused rather than guessed, so a PAL `.bk2` that sets it is
  out of reach until someone can say what the values mean from a public source.
* **The six frames of §4 are six frames**, spread over 83 218 and including the last. A
  frame-by-frame comparison against `BizHawk` would need `BizHawk`'s own per-frame output, which
  is the owner's to produce; the CRC-per-100-frames column of the supplied log is one, but what it
  is a CRC *of* is not in the public description, so it was not used.

---

## 6. Is a third engine worth it?

**No.** The recommendation is to integrate neither `NesHawk` nor any other core, and the reasons
are the measurements above rather than a preference.

**1. The job `NesHawk` would be integrated to do, Zaxnes already does.** The case for a third
engine was: `.bk2` movies are published by TAS sites, they are recorded on a cycle-accurate core,
and Zaxnes's own engine is cycle-accurate too, so they ought to sync. They do — §4 is that, at
83 218 frames, on two dumps, with the lag count matching to the frame. Adding `NesHawk` to play
movies Zaxnes already plays exactly would buy nothing.

**2. The thing that does not sync cannot be fixed by an engine at all.** The `.fm2` movies fail
because FCEUX is wrong and they were recorded on it. The *existing* second engine already solves
that: lot FCEUX-48 plays an `.fm2` on FCEUX, which is where it syncs. There is no third case.

**3. The cost would not be small.** Lot FCEUX-23 took four steps to abstract one engine behind
`zax-core-api` and FCEUX is C++ with a C API that one person wrote a fork for. `NesHawk` is C#
inside a large managed application; reaching it from a Rust desktop build means either a CLR host
or a port, and the licence question would be new work too. Against that, §1's figures: of 113
public ROMs, Zaxnes passes 95.

**4. And it would make the engine list worse, not better.** Today the rule a user can hold in
their head is one sentence: *Zaxnes's engine is the accurate one, FCEUX is there for coverage and
for `.fm2` movies.* A third engine whose only distinguishing feature is being right about things
Zaxnes is already right about is a row in a menu that nobody can be told how to choose.

### What to do instead, in order of what it is worth

1. **Make the application play `.bk2`.** It is the only thing in this report with a user-visible
   win and no risk: the reader exists and is tested, the movies sync on the engine the application
   already defaults to, and the work is wiring `bk2::read` into `play_movie` beside the `.fm2`
   path — plus the refusals, which are already written and already name what they refuse.
   **Opening a `.bk2` must not switch the engine**: unlike an `.fm2`, this is a movie that syncs
   on Zaxnes's own.
2. **Fix the three header fields the application's Zaxnes `.fm2` path ignores** (§3, last
   section). It will not make a TAS sync, and it removes a reason that is ours rather than FCEUX's.
3. **Say the `.fm2` situation out loud in the interface.** A user who opens a TAS of an MMC3 game
   and watches it desync on the Zaxnes engine has met a documented FCEUX inaccuracy, not a bug;
   the application already forces the FCEUX engine for an `.fm2` (lot FCEUX-48), which is the
   right behaviour, and the reason belongs in `HOWTO-RUN.md` beside it.

### The one thing that would change this recommendation

A `.bk2` of a game with a mapper IRQ that **does not** sync on Zaxnes. That would mean the
*Faxanadu* result is a property of a quiet MMC1 game rather than of the two cores agreeing, and
the question would be open again. Until someone runs one, §4 is the evidence and §5 is its bound.
