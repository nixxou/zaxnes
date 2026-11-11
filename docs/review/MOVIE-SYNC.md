# Playing TAS movies on Zaxnes — where its engine parts from one, and whether `.bk2` is the answer

Lots **EVAL-SYNC**, **EVAL-SYNC-2**, **EVAL-SYNC-3** and **EVAL-SYNC-4**, branch `eval/bk2`, base
`432492c4` (lot FCEUX-48, which gave the application a second engine and `.fm2` playback on it).
All of them are **evaluations** — they measure and explain — except the third, which makes **one**
emulation change on the owner's decision: see §4d. **EVAL-SYNC-4 changes no emulation at all.**

The working, with every command, is `docs/progress/EVAL-SYNC.md`,
`docs/progress/EVAL-SYNC-2.md`, `docs/progress/EVAL-SYNC-3.md` and
`docs/progress/EVAL-SYNC-4.md`. The instruments are `zax-harness movie-sync` and
`zax-harness frame-trace`.

**EVAL-SYNC-2 is the second `.bk2`**, the one §6 of the first lot named as the measurement that
would reopen the question — an MMC3 game with seven resets that syncs on `BizHawk` and **does
not sync on Zaxnes**. Its sections are §1b (the work-RAM trace), §4b (the result) and the
amendments marked *EVAL-SYNC-2* in §5 and §6.

**EVAL-SYNC-3 is why it did not sync**, from a per-frame reference the owner produced afterwards,
and it is **not** what EVAL-SYNC-2 expected: the instrument had been playing the game's own 3D
widescreen hack instead of the ROM the movie was recorded on. Its sections are §1c (the three
instruments it added and the Reset semantics it measured), §4c (*Kirby* after the fix), §4d (the
mapper's soft reset, the one emulation change) and the amendments marked *EVAL-SYNC-3* in §5
and §6.

**EVAL-SYNC-4 is the 600-frame window closed**, from the second per-frame reference the owner
produced (frames 400 to 1 100). It finds the divergence — frame **926** — names the byte, the
instruction and the branch, and puts a **number** on the artefact class that the three earlier lots
could only name: the two machines read the 2 KiB **5 to 17 CPU cycles apart**. Its sections are
§1d (the two instruments it added), §4e (frames 400-1 100, frame 926 and the DMC) and the
amendments marked *EVAL-SYNC-4* in §5 and §6.

**NESHAWK-PROFILE closes frame 926**, and it is the first lot of this series on branch
`feat/neshawk-profile` rather than `eval/bk2`. It was allowed to read NesHawk's source, found four
timing choices the core makes that Zaxnes does not, and put them behind a **timing profile** of
Zaxnes's own core — `zax_nes::NesHawkCompat`, selected automatically by a `.bk2` whose header names
`NesHawk`. *Kirby* then replays with **no live difference at any of the 1 100 frames** the
references cover. It changes the standard profile's behaviour **not at all** (`golden-core`
byte-identical) and needs no third engine, which sharpens §6's answer rather than reversing it. Its
section is §4f, with the amendments marked *NESHAWK-PROFILE* in §5 and §6;
`docs/review/NESHAWK-PROFILE.md` is how to add an item.

**NESHAWK-PROFILE-2 closes frame 4666 and the 4 688 frames after it**, from the third per-frame
reference the owner produced (frames 1 100 to 10 000). It built the instrument the diagnosis needed
— `frame-trace --dmc-census`, behind a feature that is off by default — found that frame 4666's DMA
is entirely ordinary and that the missing difference was **item 1's third half**, the one
NESHAWK-PROFILE named and did not implement: NesHawk halts on the request cycle *even when the CPU
is writing on it*. *Kirby* now replays with **no live difference until frame 9 353**, 8 175 of
8 901 frames clean against 3 531, and **15 of the owner's 18 screenshots match against 0** for the
standard profile. It also corrects a parity in item 3 and adds `dmc_realign`, both of which turn
out to be *exactly* inert on this movie, and it eliminates the end-of-sample aborts from *Kirby*'s
candidates by arithmetic rather than by experiment. The standard profile is again byte-identical
and about 2 % faster. Its section is §4g, with the amendments marked *NESHAWK-PROFILE-2* in §5.

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

**EVAL-SYNC-2 qualifies the second answer and leaves the third standing.** A second `.bk2` — an
MMC3 game, *Hoshi no Kirby*, 156 312 frames with **seven resets**, which syncs to its ending on
`BizHawk` 2.11.1 — **does not sync on Zaxnes**: the two machines are already running different
games by frame 100, and none of the six `BizHawk` screenshots matches. So "Zaxnes plays a `.bk2`"
is now **one movie of two**, and there is an accuracy gap of ours somewhere in *Kirby*'s first
sixty frames. The hunt for it is §4b; it ends with the gap **not found** and six candidate causes
eliminated by measurement. One real Zaxnes error came out of the hunt on the way — **a soft reset
puts the MMC3's registers back to power-on, and the hardware's cartridge connector has no reset
pin** — and it is not the one. The recommendation of §6 does not change, but its reasoning does.

**EVAL-SYNC-3 found the cause, and it was the instrument rather than the engine.** `NesCore`
powers on with the **3D built-in patches enabled**, so `load_rom` hands a recognised commercial
game its *widescreen hack* instead of the image it was given — and that hack's **overclock** grants
the CPU thousands of free cycles a frame, which no NES does. `movie-sync` never turned it off, so
every Zaxnes measurement of *Kirby* in the two earlier lots was of the hack. With it off, Zaxnes
matches `BizHawk`'s work RAM **in every live address at every one of the 400 frames the finest
reference covers**, and its lag count over the whole 156 312 frames is **24** — `BizHawk`'s own
figure, where the earlier lots read 25. A second, much smaller divergence remains between frames
400 and 1 000 (§4c), and the two candidates EVAL-SYNC-2 left are now eliminated by measurement.
EVAL-SYNC-2's class (a) — the mapper and the RESET button — **is fixed** (§4d). **Reason 1 of §6 is
restored.**

**EVAL-SYNC-4 closes that window and the divergence is twelve bytes wide.** The owner's second
per-frame reference (frames 400 to 1 100) puts *Kirby* in step with `NesHawk` to frame **925** —
ten of eleven screenshots in that stretch now match **exactly**, where EVAL-SYNC-3 had 0 of 6 — and
the parting is at the start of frame **926**, in three bytes. Two of them are the reading-instant
artefact again, and this lot stops calling it a class and measures it: **the reference reads the
2 KiB 5 to 17 CPU cycles before Zaxnes's frame boundary**, eight independent frames agreeing. The
third byte is `$0039`, the game's own frame-sync semaphore, and it differs because Zaxnes's
frame-926 work ran about **sixty CPU cycles** behind `NesHawk`'s and the NMI caught it seventeen
instructions short of the `STA $39` at `$C06E` that clears it. The branch that then turns sixty
cycles into two different games is `BNE $8248` at **`$8241`**, on a read of **`$4015`** — whose
bit 4 is *"the DMC still has bytes to fetch"*. And the only mechanism in the frame of the right
size is the **DMC's own sample-fetch DMA**: a 257-byte DPCM sample at rate `$F`, 69 fetches a
frame, 3 or 4 halt cycles each. Zaxnes passes **52 of 58** public DMA/APU ROMs and fails none; it
is **undetermined**, and §4e names the four lines of `BizHawk` Lua that would close it.

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

## 1b. The work-RAM trace (EVAL-SYNC-2)

The first lot could only ask whether the two engines agreed with **each other**. The owner's
`BizHawk` runs — `G:/3DSen/test-movies/bizhawk-runs/*.txt`, one line per 100 frames, a checksum
of the 2 KiB and a running lag count — are a third machine's answer, so EVAL-SYNC-2 added the
means to hold a Zaxnes run against them line for line:

```
  --trace <file.txt>   the same checksum every --trace-step frames (default 100), in the
                       reference's own shape, with the running count of insensitive frames
  --trace-ref <file>   hold the run against a reference trace
  --dump <dir> --dump-frames a,b,c   those frames' 2 KiB, raw, because a checksum says two
                       machines differ and only the bytes say how
```

The checksum is the owner's, reproduced rather than improved on: `s = s*31 + byte` over
`$0000`-`$07FF`, modulo 2^32. The **frame offset between the two numberings is measured, not
assumed** — the trace keeps every frame's checksum, not only the written ones, and the comparison
tries ±3 and reports the winner with the runner-up beside it. On both *Faxanadu* references the
answer is **offset 0**, and no other offset matches a single line of 832.

**What a run that is in step looks like in this instrument**, which is the thing to know before
reading §4b: *Faxanadu* scores **207 and 220 of 832** checksum lines exactly equal, **832 of 832**
lag counts, and its last agreeing line is the last line there is. It does not score 832 because
the two machines read the 2 KiB at a **different point inside the frame**: of the 625 differing
lines, 54 are explained by a single byte and every one of those 54 is in `$01D7`-`$01FB`, the top
of the stack — the leftovers below the stack pointer, where chance predicts 0.15 such hits. So
**the last agreeing line is the marker of sync, not the first differing one.**

The calibration also found a setup difference of ours, and fixed it: **`NesHawk` powers the work
RAM on with the `00 00 00 00 FF FF FF FF` pattern**, not with zeroes. Of the three deterministic
fills only that one puts any line in step at all (207 and 220 against 0 and 0), so `bk2::read`
now asks for it, with the measurement in the note it prints. The format's one field about the
2 KiB, `InitialWRamStatePattern`, is read too and **refused by name** when it is not `null`.
Neither *Faxanadu* run's picture changed: §4's twelve screenshot matches are twelve of twelve
after the fix as before it.

---

## 1c. Three more instruments, and when a reset happens (EVAL-SYNC-3)

The owner's finer reference — `kirby-jp-step1.txt`, the whole 2 KiB **and** the lag count at the
start of each of the first 400 frames — is a different kind of evidence from a checksum per hundred
frames, and it needed different instruments.

### `--ram-ref`: the bytes, not the checksum

`movie-sync --ram-ref <file>` holds a run's own work RAM against a reference's `ram=<hex>` field
**byte for byte at every frame it has**, streamed so that nothing keeps 2 KiB a frame. Each
differing address is sorted into one of three classes:

| class | addresses | why it is a class and not an excuse |
|---|---|---|
| the stack page | `$0100-$01FF` | leftovers below the stack pointer. §1b measured 54 of 54 single-byte residuals here on *Faxanadu*, which syncs exactly |
| the temporaries | `$0000-$001F` | the scratch of whatever routine is running at the instant the 2 KiB is read |
| **live** | everything else | what the game has *decided* — the only class a divergence can be read off |

A reference that carries no `ram=` field is **refused by name** rather than compared against
nothing, and `--engine both` is refused too: this is one run against a third machine's bytes.

### `frame-trace`: one frame, instruction by instruction

`zax-harness frame-trace <movie> <rom> --frame <n>` plays a movie to a chosen frame with
`movie-sync`'s own machinery and then drives that one frame by hand, reporting the state before
every instruction with the PPU's scanline and dot, the NMI and IRQ handler entries, and every
change of a `--watch`ed address with the instruction that made it.

It is **entirely outside `zax-nes`** — no `cfg`, no feature, no new core API: `CpuTracer::capture`
exists for `nestest.log` (REQ-CORE-110) and reads only through `CpuBus::peek` (REQ-CORE-081),
`NesCore::cpu_and_bus_mut`'s own doc comment says "to step instructions by hand", and
`Ppu2C02::vblank_just_started` is what `run_frame`'s own loop breaks on. And it **checks itself**:
every run prints the work-RAM checksum of the traced frame beside the same frame played the
ordinary way from the same state, and they have to be equal.

### `--converge`: telling a divergence from a reading artefact

The measurement that mattered most, and the one a checksum trace can never make. A reference line
says what the 2 KiB held at the start of a frame; it does **not** say at which dot of the previous
frame the reading was taken. So `--converge <reference>` compares the run's own 2 KiB against the
reference's bytes for that frame at **every instruction** of the frame, and reports where it comes
closest. If some instant of our frame holds the reference's **live** memory, the two machines ran
the same frame and only wrote it down at different moments.

The count is of live bytes alone, because the dead stack below the stack pointer never agrees again
once two machines' interrupts have landed even slightly apart — "identical to the byte" is
unreachable by construction, and "no live byte differs" is exactly the question.

### When a `Reset` takes effect, in each of the two formats

Neither public description says, and the two answers are different. Both were measured.

| | the file's column means | base shift in the reader | `--resets on` |
|---|---|---|---|
| **`.fm2`** | before the flagged frame | 0 | the flagged frame |
| **`.bk2`** | **after** the flagged frame | **+1** | one after the flagged frame |

**The `.bk2` answer** is the owner's per-frame run of the *Kirby* movie. Its first Reset bit is on
frame 3, and the two readings are told apart by what frame 3 does: resetting **after** it leaves
frame 3 a normal frame and all 400 of `BizHawk`'s lag counts agree, while resetting **before** it
makes frame 3 a lag frame — the counter then reads 4 where `BizHawk`'s reads 3 — and leaves the
2 KiB untouched until frame 6 where the reference's moves during frame 3.

**The `.fm2` answer** is FCEUX itself. The format page says a frame's commands are "interpreted as
a bit field corresponding to miscellaneous input states which are **valid at the start of the
frame**", which does not quite say *which* frame — so the same synthetic movie (60 frames on
`nestest.nes`, one soft reset, one A press) was played through `FCEUI_LoadMovie`, where FCEUX's own
movie player decides, and through fed inputs at each of the three placements. Of the 60 frames
compared over the whole 2 KiB, `on` differs on **0** and the two neighbours on 9 to 55, at each of
three reset frames tried. It is kept as a test, with the *negative* half asserted too so that it
cannot pass vacuously.

`--resets` is therefore now **relative to the format's own placement**, and still brackets the
one-frame uncertainty either side of it. Anyone re-running EVAL-SYNC-2's commands should know that
that lot's `--resets late` on a `.bk2` is this lot's default, and this lot's `--resets early` is
that lot's default.

### `--builtin-patches`, which is off, and §4c is why

`ZaxnesMachine::new` now turns the 3D built-in patches **off** before `load_rom`, as
`zax-harness test-roms` has always done. `--builtin-patches` turns them back on. See §4c.

---

## 1d. Two more instruments, and both exist because a count has no sign (EVAL-SYNC-4)

### `movie-sync --ram-ref-table <file>`: the whole table, not the first twelve frames

`--ram-ref`'s printed report lists the first twelve frames with a live difference, which is the
right length for a paragraph and the wrong length for a window of seven hundred frames holding
nine isolated differences and then a hundred and seventy-five in a row. `--ram-ref-table` writes
**every** compared frame's own line — `<frame> total= live= stack= temp=` and the live addresses —
so that `awk` can answer "which frames parted" instead of a reader counting. `RamCompare::keep_table`
is off unless asked for: a movie is 156 312 frames of entries.

### `frame-trace --converge-window <n>`: which addresses, and on which side of the boundary

`--converge` says **how far** our 2 KiB is from the reference's at every instruction and never
**in which direction**. Three bytes differing at a frame boundary can as easily be writes our own
frame has not reached yet as writes the reference's machine never made — and those are opposite
diagnoses. `--converge-window` prints the live difference instruction by instruction for *n*
instructions either side of the boundary, **with the addresses named** and each line marked `<`,
`=` or `>` by its side of the boundary. §4e's frame 926 is what it was built for, and it is what
ruled out the first explanation that fitted.

`frame-trace` also now reports, per frame, `NesBus`'s own two DMA counters — **DMC sample fetches
and OAM transfers**. Both getters (`dmc_dma_count`, `oam_dma_count`) already existed and are
public, for the DMA timing tests; nothing in `zax-nes` changed. A DMC fetch halts the CPU for
three or four cycles, so a frame's count is the size of the one mechanism that moves the program's
position inside a frame with no read and no branch deciding it — which is the whole of §4e's last
half.

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
whole frame — and Zaxnes passes both. Of that suite's ten singles Zaxnes passes **all ten** and
FCEUX fails **nine**; the one both pass is `04-nmi_control`.

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

## 4b. The second `.bk2`, and it does not sync (EVAL-SYNC-2)

| | |
|---|---|
| the movie | `keyword-kirbysadventure-noabilities.bk2` — TASVideos 6192M, `BizHawk` 2.8, `NesHawk` |
| the game | *Hoshi no Kirby: Yume no Izumi no Monogatari*, **MMC3/TxROM**, battery |
| | 156 312 frames, **seven Reset presses**, the first on frame 3 |
| on `BizHawk` 2.11.1 | **syncs to the end**, in the final Nightmare fight |
| the reference | `kirby-jp.txt`, 1 563 lines, and six screenshots |

### The result

| | *Faxanadu* (§4) | *Kirby* |
|---|---|---|
| checksum lines agreeing | 207 and 220 of 832 | **0 of 1 563** |
| the last agreeing line | 83 200 — the last | **none** |
| lag counts agreeing | 832 of 832 | all but three resets' lines; 25 against **24** |
| screenshots matching | **12 of 12** | **0 of 6** |

`BizHawk`'s last frame is the star corridor of the final fight with the `KIRBY` and `ENEMY` bars;
Zaxnes's is a grass level with `SCORE : 0005500`. At frame 1 000 `BizHawk` is playing Vegetable
Valley with 3 400 points and Zaxnes is still on the level-intro card.

### Where it parts: inside the first hundred frames

The reference has no line before 100 and no screenshot before 1 000, so that is the floor the
data imposes, and the trace is already wrong at line 100. Three measurements say what kind of
divergence it is:

* **not a time shift.** Every frame of the first 5 000 was checksummed and looked up against each
  reference line from 100 to 5 000: **no frame of ours holds their 2 KiB**;
* **not a small one.** At lines 100 to 600 no single byte explains the difference and the
  two-byte "solutions" are at the chance rate — against *Faxanadu*'s 54 single-byte hits, all in
  the stack;
* **not a delay on screen either.** Our frames 900 to 2 600, every tenth, were held against
  `kirby-jp-f1000.png` under the same colour-bijection rule as §4: **none of the 171 matches**,
  on a test that says `MATCH` on *Faxanadu* at 1 000 and 10 000.

The lag column agrees almost everywhere, and that is **not** evidence of sync here: *Kirby* lags
24 frames in 156 312 and 6 of them are in the first 100, so outside the resets the counter cannot
tell two runs apart. On *Faxanadu*, with 13 398 lag frames, it could and did.

### Six candidate causes, each eliminated by measurement

Each a 3 000-frame run against the reference. **None puts a single line in step.**

| what was varied | values | lines in step |
|---|---|---|
| the console's work RAM (`--ram`) | `default`, `zero`, `ff` | 0, 0, 0 |
| the cartridge's PRG-RAM (`--prg-ram`) | `default`, `zero`, `ff` | 0, 0, 0 |
| the PPU's OAM, palette and nametable RAM (`--ppu-ram`) | `default`, `zero`, `ff`, `random:0` | 0 ×4 |
| the reset column (`--resets`) | `on`, `off`, `early`, `late` | 0 ×4 |
| blank frames before the movie (`--warm-up`) | 1, 2, 3, 4 | 0 ×4 |
| the PPU write-inhibition window (`--ppu-warmup`) | on | 0 |

Two of those say more than a zero. **`--resets off` leaves the pictures at frames 400 and 1 000
identical** to the honoured run, so the reset on frame 3 is real but not where the machines part.
And **`--ppu-warmup` changes the run not at all** over 1 200 frames of per-frame checksums, which
says *Kirby* writes neither PPUCTRL nor PPUMASK in the frame after a reset — so the ~29 658-cycle
window `zax-nes` does not model (Q-078 §1) is not what parts this movie either.

### What the hunt did find: a class (a), and it is not the cause

> **A soft reset puts the MMC3's registers back to power-on, and the hardware cannot.** nesdev's
> "Cartridge connector" lists every pin of both the 60-pin Famicom and the 72-pin NES edge, and
> there is **no `/RESET` among them**: a mapper never sees the console's reset button, so an MMC3
> keeps every register the game wrote into it across as many resets as the player presses.
> `NesCore::soft_reset` (`rust/crates/zax-nes/src/nes_core.rs:533`) calls `Cartridge::reset`,
> which for mapper 4 is `Mmc3Board::reset`
> (`rust/crates/zax-nes/src/cartridges/mappers/mmc3.rs:1162`) and puts the eight bank registers,
> the bank-select latch, the PRG-RAM protect byte, the whole IRQ unit and the A12 watcher back
> where they started.
>
> **Reproduction**: `rust/crates/zax-harness/tests/soft_reset_mapper_tests.rs` — three tests, a
> public MMC3 ROM, no movie.
>
> **Size of the fix**: one line (not calling `Cartridge::reset`), plus a decision for the
> reset-counting multicarts, which detect a reset by the absence of M2 rather than by a wire, and
> an **amendment to REQ-CORE-122**, whose "PPU, APU **et mapper** suivent leur état documenté
> *after reset*" is what the code implements and whose mapper half has no documented state behind
> it. The spec clause is the owner's to amend; nothing was fixed here.

It is not *Kirby*'s cause: the only reset before the divergence is the one on frame 3, three
frames into a boot that has not yet written a bank register worth keeping, and dropping every
reset in the movie changes nothing visible. Where it would bite is the six later resets, by which
time the two machines are 22 000 frames apart and nothing can be measured.

The MMC3's **IRQ** is not a candidate either, and the evidence is public: Zaxnes passes all ten
`mmc3_test` / `mmc3_test_2` ROMs (the eleventh, `6-MMC6`, is an expected failure by
REQ-CORE-100 / AMD-061 / Q-114) and FCEUX fails all ten. A mapper-IRQ error tears the status bar,
and *Kirby*'s pictures are well-formed.

### What is left, and what would settle it

Two candidates survive, both inside the first sixty frames and both documented choices of
`zax-nes` that **cannot be varied from outside the core**:

1. **Where the PPU is at power-on.** Q-003 and REQ-CORE-020's notes start the machine at the
   start of vblank. `NesHawk`'s position is not published, and *Kirby*'s boot is a chain of
   vblank waits whose first input is a **one-frame A press on frame 63**.
2. **The inhibition window for `$2005` and `$2006`.** `--ppu-warmup` can undo `$2000` and `$2001`
   from outside; `v`, `t` and `fine_x` cannot be zeroed without a side-effecting read of `$2002`,
   which would clear the vblank flag the game is about to read.

Both need the same thing: a **`cfg`-gated experiment inside `zax-nes`**, off by default and with
golden-core unchanged — a power-on dot the harness can choose, and a cycle counter in the PPU
that drops writes to the four registers for 29 658 cycles. Twenty or thirty lines in `Ppu2C02`,
and then one `--trace-ref` run says which of the two it was, or that it is neither.

> **Superseded, EVAL-SYNC-3.** Neither candidate survives and the experiment **should not be
> written**: the 400 frames the owner's finer reference covers include the whole boot, the one-frame
> A presses at 63, 97 and 212 included, and **every live byte of all 400 agrees** (§4c). Both
> candidates predicted a boot that comes out a frame apart, and it does not. The cause was
> elsewhere entirely.

---

## 4c. Why it did not sync: the instrument was playing the hack (EVAL-SYNC-3)

The owner produced the measurement §5 asked for — `kirby-jp-step1.txt`, `step = 1` over the first
400 frames with the whole 2 KiB and the lag count — and it put the first live difference at the
start of frame 247: **`$004B` = `01` on Zaxnes, `00` on `BizHawk`**, then 12 bytes at 248 and 167
at 249.

### `$004B` was not it

`frame-trace --frame 247 --watch 004B` names the instruction: `INC $4B` at **`$E94F`**, nineteen
instructions before the frame boundary, in a routine that restores the byte from the stack at
`$E6D3` twenty-two instructions later. `--converge` then settles what that means: **1 020 instants
of our own frame 247 hold the reference's live memory**, the last of them 19 instructions before our
boundary. So no read decides `$004B`; **where the frame ends relative to the program** does, which
is §1b's artefact class at an address above `$001F`. The frame has no `$2002` read at all.

### Frame 248 was not an emulation difference either

| frame | instructions | CPU cycles | instructions costing **no** cycle |
|---|---|---|---|
| 244-247 | 8 283 - 9 120 | 29 778 - 29 784 | 0 |
| **248** | **19 280** | 29 781 | **9 689** |
| 249, 250 | 18 886, 19 265 | 29 781 | 9 689 each |

19 280 instructions cannot fit in 29 781 CPU cycles — the shortest 6502 instruction is two — and
the trace says why: from instruction 9 552 the PPU stands at scanline 240 dot 0 and the clock does
not move for 9 689 instructions. One thing in this core does that: `WideExtension::take_stall_cycle`,
the **3D extension's overclock**, armed at the start of line 240 and worth `$5006` scanlines of CPU
time with the PPU, the APU and the clock held still. nesdev's *Cycle reference chart* is
unambiguous that no console can: NTSC is one 21.477272 MHz crystal divided by 12 for the CPU and by
4 for the PPU, 29 780.5 CPU cycles a frame, and the only thing that moves CPU time against PPU time
on hardware is DMA, which *takes* cycles and never grants them.

The extension was unlocked because **`NesCore` powers on with `builtin_patches_enabled` true**, so
`load_rom` gives a recognised commercial game its widescreen patch rather than the image it was
handed — and `builtin_patches::KIRBY_JAPAN` is exactly that patch for this ROM. `test_roms` has
always turned it off; `movie-sync` never did.

> **Class (c) — a setup difference of the instrument, fixed in `zax-harness`.**
> `ZaxnesMachine::new` now calls `set_builtin_patches(false)` and `set_advanced_patches(false)`
> before `load_rom`. `--builtin-patches` turns them back on, which is how the figures below were
> measured either side of the line rather than asserted. **Nothing in `zax-nes` changed for this.**

### What the fix is worth

| | patches **on** (the two earlier lots) | patches **off** |
|---|---|---|
| frames of 400 with **no live difference** | 246 | **400 of 400** |
| the first live difference | frame 247 | **there is none** |
| checksum lines agreeing exactly, of 400 | 89 | **190** |
| lag counts agreeing, of 400 | 400 | 400 |
| over the whole movie: lag counts agreeing | all but three resets' lines | **all 1 563** |
| over the whole movie: input-insensitive frames | **25** | **24 — `BizHawk`'s own figure** |
| the 3D extension during the run | UNLOCKED, 9 689 free instructions a frame from 248 | **locked, 0** |

Frame 247 traced again with the patches off says the same from the inside: the extension is locked,
no instruction costs no cycle, `$004B` goes `00`→`01`→`00` **inside** the frame as it does on
`BizHawk`, and **4** bytes differ in the whole 2 KiB with **0** of them live.

### *Kirby*'s standing now, and what still parts

**In step with `NesHawk` for the whole reach of the finest reference there is**, and with
`BizHawk`'s own lag count exact over 156 312 frames. It still does not sync to the end:

| frame | 1 000 | 10 000 | 30 000 | 60 000 | 83 000 | 156 312 |
|---|---|---|---|---|---|---|
| the screenshot | no match | no | no | no | no | no |
| pixels differing of 57 344 | **275** | 50 537 | 15 445 | 48 207 | 46 435 | 22 912 |

275 pixels is not a desynchronised game: at frame 1 000 every differing pixel is inside **one 25×17
box** — a single object — with the background, the HUD and the score identical. (Before the fix,
frame 1 000 was a different screen altogether.) The comparison was validated before being believed:
the same script says **MATCH at all six frames** on *Faxanadu*, with §4's own colour counts.

So one divergence remains, in **frames 401 to 1 000**, and three things bound it: it is not in the
boot (all 400 frames agree), not the extension (locked and measurably idle), and not `$2002`,
sprite 0 or sprite overflow (*Kirby* reads `$2002` not once in a frame). What the trace shows it
**does** use mid-frame is the **MMC3 scanline IRQ**: `$C000` = `$AF` and `$C001`, `$E001` written
in vblank, the IRQ taken at scanline 174 dot 287, its handler running from zero page. Zaxnes passes
`1-clocking`, `2-details`, `3-A12_clocking`, `4-scanline_timing` and `5-MMC3` on **both** public
MMC3 suites and all eleven of `ppu_vbl_nmi`; the two it fails are the revision-A half of a
contradictory pair, and nesdev puts the whole A-against-B difference at `$C000` = `$00`, which
*Kirby* never writes.

**The measurement that would decide it** is the same script again, pointed at frames **400 to
1 100** with `step = 1` in exactly `kirby-jp-step1.txt`'s format — the window that brackets the
first picture difference from both sides. `--ram-ref` then names the frame and `frame-trace
--converge` the instruction, as they did for 247. Nothing is needed from this side first, and in
particular no change to `zax-nes`. If the first live difference lands in pages `$0400`-`$05FF`
(where frame 249's 167 bytes were concentrated), the second thing to ask for is a per-scanline log
of that one frame: `$2000`/`$2001`, the MMC3 counter with its latch, reload and enable, the
scanline **and dot** at which `/IRQ` is asserted and at which the handler's first instruction runs,
and the CPU cycle count at the start of each scanline. Zaxnes can produce all four from
`frame-trace --out` today.

> **Superseded, EVAL-SYNC-4.** The owner made that measurement —
> `kirby-jp-step1b.txt`, frames 400 to 1 100 — and §4e is what it says. The window is closed:
> *Kirby* is in step to frame **925** and parts at **926**. The second ask, the per-scanline log of
> one frame, is **not** what is needed: the first live difference is not in pages `$0400`-`$05FF`
> and the MMC3 IRQ lands on the same scanline and nearly the same dot in the frames either side of
> it. §4e names what to ask for instead.

---

## 4d. The mapper and the RESET button: §4b's class (a), fixed (EVAL-SYNC-3)

The owner decided on 2026-11-10 to fix it, which makes this the one emulation change of the three
lots.

`NesCore::soft_reset` now calls a new `Cartridge::soft_reset`, which calls a new
`Mapper::soft_reset` — **a no-op for every board**, because the cartridge connector has no
`/RESET` pin for a mapper to see. The mirroring and the M2 cycle counter are no longer zeroed by a
soft reset either; `Cartridge::reset`, the power cycle, still zeroes everything.

**One board overrides it**, and only on the strength of its own page. nesdev, "NES 2.0 Mapper 313":
the reset-detecting multicarts see a reset as *"an interruption of the M2 signal"*, not as a wire.
Of that family this build implements one number — **215**, the UNL-8237 — whose page says it in so
many words: *"The Outer Bank Register is reset to its power-up value if a Reset has been detected
via an interruption of the M2 signal."* So `$5001` goes back and nothing else does.

| | |
|---|---|
| tests | **3 new** in `zax-nes`'s `soft_reset_tests.rs`: the MMC3's banks and mirroring survive, its IRQ unit stays armed and fires again with no further write, and the 215's outer bank goes back while the chip behind it does not. There is **no public mapper-215 image** (checked: every `.nes` under `assets-free/test-roms` read and none is 215), so that one uses the synthetic image the workspace's other mapper-215 tests use |
| EVAL-SYNC-2's own test file | **flipped**, as its module comment said whoever fixed this should do: `soft_reset_mapper_tests.rs` is now a contract, with the power cycle asserted beside the soft reset |
| `zax-nes`, the whole suite | **1 177 passed, 0 failed** |
| **golden-core**, 98 originals | `8d50807085…` — **byte-identical** to FCEUX-23's baseline, which is both the expected answer and the bound on the change: golden-core drives the power cycle, which is untouched |
| the spec | **not edited.** The proposed amendment of REQ-CORE-122 — the exact new French wording, with the nesdev citations and the rationale — is `docs/review/AMD-REQ-CORE-122-mapper-reset.md`. It is built on the moule of **AMD-061 §5**, which made the same discovery once already for the *hard* reset (Q-112, list of one: mapper 28) |

**It changes nothing on *Kirby***, which is the expected result twice over: the six screenshots are
byte-identical to §4c's run. The only reset before the divergence is on frame 4, three frames into
a boot that has not yet written a bank register worth keeping, and the six later ones land after
frame 22 000 where the two machines are already apart. What is new and worth saying is that the six
are still survivable — the movie runs its 156 312 frames with no stop and the picture keeps moving
after each — and that they now **matter**: with `--resets off` the pictures at frames 22 556,
22 560 and 22 700 all differ from the honoured run, where at frame 400 EVAL-SYNC-2 found dropping
them changed nothing. The game re-enters its reset vector with the MMC3 banks it chose still
mapped, which is what the hardware does and why nesdev requires an MMC3 game's reset vector to
point into `$E000-$FFFF`.

---

## 4e. The window closed: frame 926, the frame semaphore, and the DMC (EVAL-SYNC-4)

`kirby-jp-step1b.txt` is `kirby-jp-step1.txt`'s format exactly — 701 lines
`<f> <checksum> lag=<n> ram=<4096 hex digits>` for frames **400 to 1 100** — so `--ram-ref` took it
unchanged, and the two files overlap on line 400 **byte for byte**, which is the cheapest check
that the owner's two runs are one run.

### Where it parts

| | |
|---|---|
| frames compared, byte for byte | **701 of 701** |
| with **no live difference** | **517** |
| the last frame with no live difference | **925** |
| frames with a live difference | **184**: nine isolated between 645 and 875, then **926 to 1 100 without a gap** |
| **lag counts** | **all 701 agree** |

And the pictures say the same from the other side. The owner's screenshots every fifty frames,
under §4's colour-bijection rule, with the pairing **measured** rather than assumed (our frame *f*
against `s<f>.png` scores 10 MATCHes of 15; our frame *f-1* scores 1):

| frame | 400 | 450-900, every 50 | 950 | 1 000 | 1 050 | 1 100 |
|---|---|---|---|---|---|---|
| verdict | no, by **1 pixel** | **MATCH, all ten** | no | no | no | no |
| pixels breaking the bijection | 1 | 0 | 38 195 | 335 | 45 088 | 52 156 |

**Ten of eleven shots in the agreeing stretch match exactly** — the first *Kirby* screenshot ever
to match at all; EVAL-SYNC-3 scored 0 of 6, the nearest being 275 pixels. Frame 400's single pixel
is at (209, 167) on a frame whose work RAM has **no** live difference, which is the same class as
the stack page: one object's rendering caught a few cycles apart. Frame 1 000's 335 pixels are
inside x 147-171, y 97-113 — the same 25×17 box EVAL-SYNC-3 measured, now known to be downstream
of frame 926 rather than a thing of its own. At 1 050 the reference's picture is **one colour** and
ours is twelve: two machines playing different games.

### The nine isolated frames, and the artefact finally has a number

Eight of the nine (645, 680, 714, 740, 796, 821, 837, 875) are the same instruction in the same
routine: `STA $0400,X` at **`$C223`**, inside a **job-queue drain** at `$C221`-`$C22E` that copies
bytes into the page-`$04` buffer the NMI handler flushes to the PPU, running on scanlines ~213 to
240. The frame boundary falls **inside a copy**, and whichever machine reads its 2 KiB later has
copied one byte more.

`--converge` turns that into a figure, because our own boundary sits at cycle **29 775-29 780** of
the frame in all nine while the last instant at which no live byte differs moves:

| frame | 645 | 680 | 714 | 740 | 796 | 821 | 837 | 875 | **733** |
|---|---|---|---|---|---|---|---|---|---|
| the reference reads this many CPU cycles early | 6 | 5 | **17** | 5 | 5 | 10 | 10 | 5 | **170** |

**Five to seventeen cycles — less than one 6502 instruction, 15 to 51 PPU dots.** That answers one
of the things §5 listed as out of reach from this side, and it answers it eight times
independently. It also explains why only nine of 526 frames show anything: the drain has to still
be running in those last twenty cycles.

**`AA` is the program's own data**, not open bus, not DMA and not an unmapped read, and the
reference proves it: `$044B`-`$044E` are `AA AA AA AA` *in the reference* at frame 821,
`$04FA`-`$04FD` at 837, `$0491`-`$0492` at 733. Both machines copied the same bytes from the same
table (`LDA ($16),Y` from the cartridge's PRG-RAM at `$671A`, `$674A`, `$6747`-`$674C`) and stopped
one byte apart. `AA` is in none of the four power-on fills either. Frame 714 is the mirror image,
with `AA` on `BizHawk`'s side: there `AA` is what the *previous* frame's copy left at `$0405`.

Frame **733** is the one that is not settled: same routine, same instruction, and an instant of our
own frame still holds the reference's live memory **exactly** — but 170 cycles before our boundary
instead of twenty.

### Frame 926, instruction by instruction

Three live bytes, and the first two are the artefact again:

| | written by | at | scanline/dot | ours | theirs |
|---|---|---|---|---|---|
| `$0038` | `STA $38` at `$F049` | instruction 8 899 | SL 238 dot 225 | `86` | `87` |
| `$0049` | `STA $49` at `$F075` | 8 884 | SL 238 dot 57 | `13` | `00` |
| **`$0039`** | **`INC $39` at `$C37C`** | **133** | **SL 249 dot 120** | **`01`** | **`00`** |

`$0038` and `$0049` are the MMC3 bank-select and bank-data shadows the game keeps beside every
`$8000`/`$8001` write — 22 and 9 changes in frame 926 alone. `$0039` is the **frame semaphore**:

```
    $C37C  E6 39     INC $39     ; set, in the NMI path, scanline 249
    $C06A  10 0A     BPL $C076   ; the wait is enabled by bit 7 of $35
    $C06C  A9 00     LDA #$00
    $C06E  85 39     STA $39     ; cleared: this frame's work is done
    $C070  A5 39     LDA $39
    $C072  F0 FC     BEQ $C070   ; and spin until the next frame's NMI sets it
```

Traced over frames 918-929, every frame clears it between scanline 146 and 235 and then spins to
the frame's end — **except 926, which never reaches `$C06E` at all**. Our frame 926 ends at
instruction 8 977, `$F061 STA $8001`, **scanline 240 dot 338**, and `$C06E` is **seventeen
instructions and 61 CPU cycles** further on. Those seventeen instructions write exactly the
reference's three values (`$F07D STA $38` ← `#$87`; `$F085 STA $49` ← `$0576` = `00`; `$C06E
STA $39` ← `00`), and frame 925's instructions 6001-6015 are the same seventeen with the same
three values, which is the control. So **the reference's frame-926 state is Zaxnes's own state
seventeen instructions later**: `NesHawk` was **ahead**, by at least 61 cycles less the 5-to-17
reading offset — **about 45 to 55 CPU cycles**. (That is the opposite sign from frame 733's 170
behind. The game's spin at `$C070` re-synchronises the two machines every frame, which is why the
drift is invisible except in the two frames of 526 whose work ends within that distance of vblank.)

`--converge` on the three frames:

| frame | |
|---|---|
| **925** | no live byte differs at **2 466 instants**, from instruction 6 741 (−2 405) to 9 206 (+60) — the whole spin, the boundary included |
| **926** | **at no instant of the frame does the live memory agree**; the closest is **1** live byte, `$0039` alone |
| 927 | **56** live bytes differ at every instant, and `$0039` goes `01` → **`02`**: the semaphore is now a count of frames owed, and from here the two machines run different code |

### What decided it: `$4015`, and the DMC

Nothing read anything to decide whether the frame's work fits in a frame. What turns sixty cycles
into two different games is a read, and it is in the frame on both machines:

```
    $823C  AD 15 40  LDA $4015    ; bit 4 = the DMC has bytes left to fetch
    $823F  29 10     AND #$10
    $8241  D0 05     BNE $8248    ; taken: a sample is still playing, leave it alone
    $8243  A9 7F     LDA #$7F
    $8245  8D 11 40  STA $4011
```

| frame | our `$4015` | the branch at `$8241` |
|---|---|---|
| 925, **926** | **`$1F`**, bit 4 **set** | taken, `RTS` |
| 927, 928 | `$0F`, bit 4 clear | **not taken**, `STA $4011` |

And the sample is named from the game's own writes at frame 923: `$4012` = `$F4` (address
`$FD00`), `$4013` = `$10` (**257 bytes**), `$4010` = `$0F` (**rate `$F`**, no loop, no IRQ). nesdev
puts rate `$F` at a 54-cycle period, so a byte every **432** cycles — 68.9 a frame. Zaxnes's own
counters read **69** in a full frame and **63 + 69 + 69 + 56 = 257** over the four frames of the
sample, the sample length to the byte.

So the *number* of fetches is exactly the documented one. What is left to disagree about is what
each costs: nesdev, *DMA* — *"DMC DMA normally takes 3 or 4 cycles, depending on whether alignment
is needed"*, the halt landing only on a read cycle and slipping when the CPU writes. That is
**207 to 276 cycles a frame** and **771 to 1 028 over the sample**, and a disagreement of a
fraction of a cycle per fetch — 0.2 at frame 926, 1.3 at frame 733 — covers both measured
displacements. Nothing else in the frame has the budget:

| | |
|---|---|
| the 3D extension | **locked**, 0 instructions costing no cycle, every frame |
| `$2002`, sprite 0, sprite overflow | **not read once** in any frame examined |
| the OAM transfer | **one a frame, in every frame**, at the same dot in 925 and 926 |
| the MMC3 IRQ | one, entering its handler at scanline **174**, dots 288 and 296 |
| the mapper's soft reset | no reset anywhere in this window |
| an open-bus or unmapped read | every differing value is in the reference's own RAM too |

And the correlation that makes the DMC the candidate rather than a guess:

| frames | DPCM playing | the offset |
|---|---|---|
| 645, 680, 740, 821, 837, 875 | **no, 0 fetches** | **5, 5, 5, 6, 10, 10 cycles** |
| 714, 796 | yes, 49 and 61 | 17 and 5 |
| **733**, **926** | yes, **69** and **56** | **170**, and **≥45** |

**Every frame in which the DMC is idle agrees to within ten cycles; both frames that are tens or
hundreds out have a sample playing.** 212 of the window's 701 frames have one, in 42 runs, so a
sample playing is common and mostly harmless — it is the coincidence of a sample with a frame
whose work ends near vblank that shows.

### Zaxnes's standing on that mechanism, and the one named gap

| suite | ROMs | pass | fail | no verdict |
|---|---|---|---|---|
| `dmc_dma_during_read4`, `sprdma_and_dmc_dma`, `dmc_tests` | 11 | 5 | **0** | 6 |
| `apu_test`, `blargg_apu_2005.07.30`, `apu_reset` | 36 | **36** | **0** | 0 |
| `cpu_interrupts_v2`, `instr_misc` | 11 | **11** | **0** | 0 |
| **total** | **58** | **52** | **0** | **6** |

No failure on any ROM that reports a verdict, and the ones that matter print their own
measurement: `sprdma_and_dmc_dma` `0F 526` and `_512` `0F 528`, `dma_4016_read`
`08 08 07 08 08`, `4-irq_and_dma` `9 +527`, `apu_reset/4017_timing` `Delay … 10`, and both
`7-dmc_basics` and `8-dmc_rates` passed. `AccuracyCoin` scores **128 of 147**, the tally
`docs/review/MF-T-test-roms.md` records for this tree, reproduced character for character.

Three of its nineteen failures are on its *APU Registers and DMA tests* page, and
`docs/progress/MF-4a.md` §Step 3 diagnosed all three already:

> **One named Zaxnes gap on this mechanism, and it is the wrong size.** nesdev, *DMA*: *"When
> sample playback is stopped during the APU cycle before a reload DMA would schedule (that is, on
> the 2nd or 3rd CPU cycle before the halt attempt), the DMA starts, but is aborted after a single
> cycle"* — explicitly by a `$4015` write, or **implicitly when the sample ends**.
> `NesBus::run_dma` (`rust/crates/zax-nes/src/nes_bus.rs:682`) owns every cycle from the halt
> onwards and cannot reach that state: a fetch costs its full three or four cycles or none. Rows
> `Explicit DMA Abort` and `Implicit DMA Abort` of `AccuracyCoin`, code 2 each, are the
> reproduction. The fix is a two-or-three-cycle delay on the `$4015` **stop** matching the one
> `DmcChannel::set_enabled` (`apu/dmc_channel.rs:218`) already returns for the start, plus an abort
> check in the DMA loop; it was **written, measured and taken back out** in lot MF-4a because it
> moved no `AccuracyCoin` row and nothing public pins the new behaviour. REQ touched:
> **REQ-CORE-113**, with REQ-CORE-110 beside it; the open ticket is `Q-119`.
>
> **Not fixed here**, and not only because this lot may not: the implicit abort happens **once per
> sample end** — this sample does not loop — and is worth **one to three cycles**. Frame 926 needs
> forty-five to fifty-five.
>
> The third row, `DMC DMA Bus Conflicts`, is about the *byte* a colliding fetch returns and needs a
> per-controller-port open-bus mask `InputPorts` does not have. It cannot move the CPU's position
> at all.

### The classification, and what would close it

**Undetermined.** Not class (a): no hardware behaviour has been found that Zaxnes gets wrong by
anything like the measured size, and on every public measurement of this mechanism Zaxnes prints
the documented answer. Not class (b) either: "a `NesHawk` behaviour Zaxnes is right not to share"
would need `NesHawk`'s figure for its DMC halt cost, its get/put alignment or its abort behaviour,
and none is published — its source may not be read.

**Four additions to the owner's existing Lua script would close it.** Names are from `BizHawk`'s
public *Lua Functions List* and should be checked against the version in use:

1. **The reading instant**, which settles the whole artefact class and gives frame 926 its sign:
   `emu.totalexecutedcycles()` and `emu.getregister("PC")` written on the same hook that already
   writes `ram=`. The copy loop at `$C221`-`$C22A` is six instructions, so `pc` alone decides eight
   of the nine isolated frames and `cyc` gives the rest. **The single most valuable line in the
   list** — it converts every figure of §4e from a bounded inference into a measurement.
2. **The frame-sync point**, which is frame 926's own question: `event.onmemoryexecute` on
   **`$C06E`** and on **`$C37C`**, logging `emu.framecount()` and `emu.totalexecutedcycles()`, for
   frames 900-950. No `C06E` line at frame 926 means `NesHawk` did not reach it either and the
   three bytes have another explanation; one line means `cyc(C06E) − cyc(C37C)` is the cycle count
   `NesHawk` needed for that frame's work, against Zaxnes's — which never finished it.
3. **The branch's own read**: `event.onmemoryread` on **`$4015`** logging the value and
   `emu.totalexecutedcycles()`, frames 900-950 and 720-745. If `NesHawk` reads `$0F` at frame 926
   where Zaxnes reads `$1F`, the two took different arms at `$8241` and the quantity that decided
   it is the DMC's bytes-remaining counter.
4. **The frame length**: `emu.totalexecutedcycles()` once a frame at `event.onframeend` over
   400-1 100. Equal per-frame totals put the drift **inside** the frame, where the DMC is; unequal
   totals would change the diagnosis entirely.

If 1 and 4 come back and the drift is inside the frame, the remaining ask is narrow: the cycle at
which `NesHawk` performs each DMC fetch for one frame — `event.onmemoryread` on the sample's own
range (`$FD00`-`$FE00`, from `$4012` = `$F4` and `$4013` = `$10`) with
`emu.totalexecutedcycles()`. Zaxnes's own 69 are available from `frame-trace` today, and the two
lists either agree or name the cycle at which they first do not.

---

## 4f. The window closed for good: a NesHawk-compatible chip, not a NesHawk engine (NESHAWK-PROFILE)

§4e left frame 926 **undetermined**, and said why: *"`NesHawk`'s standing on it: **not published**"*
— no figure for its DMC DMA's halt cost, its alignment or its abort behaviour was available, and
its source was out of scope for that lot. This lot was allowed to read it, and the answer turns out
to need **no third engine at all**: four lines of NesHawk's own behaviour, put behind a *timing
profile* of Zaxnes's own core, close the frame.

**The result first.** The *Kirby* `.bk2`, same command, same references, `--profile` the only
difference:

| | `standard` | `neshawk` |
|---|---|---|
| frames 1-400 (`kirby-jp-step1.txt`): frames with a live difference | 0 | 0 |
| frames 400-1 100 (`kirby-jp-step1b.txt`): frames with a live difference | **184** — nine isolated, then **926 to 1 100 without a gap** | **8**, every one isolated |
| frames with no difference outside the stack page and `$0000-$001F` | 517 of 701 | **693** of 701 |
| **the last frame with no live difference** | **925** | **1 100**, the end of the reference |
| all 701 lag counts | agree | agree |
| the owner's fifteen screenshots, 400 to 1 100, under §4's colour-bijection rule | **10** MATCH | **14** MATCH |
| the screenshot at frame 1 000 of the long run | no, 301 px | **MATCH** |

And `frame-trace` on frame 926 says it in one line. Standard profile: *"at **NO** instant of this
frame does the live memory agree, so the two machines did not run the same frame."* NesHawk profile:
*"**NO live byte differs at 293 instruction(s)** of this frame, from 8 934 (−36 from the boundary)
to 9 226: the two machines ran the same frame and read the 2 KiB at different moments."* Frame 926's
three bytes were the last thing §4e left standing, and they are gone.

**No regression.** *Faxanadu* on both dumps gives **207** and **220** of 832 lines with the last
agreeing line at **83 200** and every lag count matching — §4's figures to the line — on *both*
profiles, and our pictures at frames 1 000, 10 000, 30 000, 60 000 and 83 000 are **byte-identical
between the two profiles**. On that movie the profile never fires.

### The four items, and that Zaxnes is the one that is right about them

Read from BizHawk's `src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/APU.cs` and `NES.Core.cs`
(MIT), nothing copied, each named against its function so the reading can be checked. On **every
one of them Zaxnes agrees with Mesen 2 and with the nesdev wiki and NesHawk does not**:

| | NesHawk | Zaxnes, Mesen 2, nesdev |
|---|---|---|
| **1a** the halt of a DMC reload | lands on the **request cycle itself**: `DMCUnit.Run()` lowers `RDY` and `RunOneFirst()` runs before `cpu.ExecuteOne()` | the request is sampled before the cycle, so the **next** read cycle is halted |
| **1b** a fetch's length | a **fixed four** cycles — halt, two re-reads, fetch — with no `% 2` anywhere on the path | three or four, by the get/put parity ("DMC DMA normally takes 3 or 4 cycles, depending on whether alignment is needed") |
| **2** the sample length, loop and IRQ | **three cycles after** the fetch (`dmc_reload_countdown`, spent in `RunOneLast`), so `$4015` bit 4 stays set across them | at the fetch |
| **3** a `$4015` enable with the buffer empty | starts a write-style fetch **even mid-sample**, delayed by the DMC timer's parity and three cycles long (`set_lenctr_en` tests `!sample_buffer_filled` as a *sibling* of the restart) | only a write that actually starts a sample is delayed; a mid-sample write leaves the reload alone |

So the profile is **not** the more accurate chip, and it is not offered as one: it fails five public
test ROMs and eight `AccuracyCoin` rows that the standard profile passes, each failure traced to the
item that causes it (`docs/progress/NESHAWK-PROFILE.md` step 3). `apu_test/7-dmc_basics`'s own text
is *"There should be a one-byte buffer that's filled immediately if empty"*, which is precisely what
item 3 denies. The standard profile's column is unchanged, ROM for ROM and row for row, and
`golden-core` over the 98 originals is byte-identical.

### Which item closed it, which is not the item §4e pointed at

Each item alone, built and run over frames 400-1 100:

| items on | last clean frame | frames differing |
|---|---|---|
| none | 925 | 184 |
| **1a** | **1 100** | 17 |
| **1a + 1b** | **1 100** | **8** |
| **2** | **925** | **184** — the standard profile's figures to the digit |
| **3** | **1 100** | 11 |

**Item 1a is the cause**, and **item 2 is exactly inert on this movie.** §4e named `LDA $4015` at
`$823C` and the `BNE` at `$8241` as the amplifier that turns a few cycles of slack into two
different games, and that reading stands — but *Kirby*'s read never lands inside the three-cycle
window item 2 opens. What moved the machine is **which cycle each of the 56 fetches of that frame
lands on**, not what any of them read. Item 3 closes it independently, because NesHawk picks a
load's delay from the DMC timer's parity where this core picks it from the APU cycle's, which moves
the start of every sample by a cycle.

The arithmetic §4e could not settle is settled too. A lone reload costs NesHawk a fixed 4 cycles
against this core's 3 or 4, so NesHawk should fall *behind* — the opposite sign from a reference
read as 45 to 55 cycles *ahead*. The trace shows why that could not decide anything: the
NesHawk-compatible run does **7 fewer instructions and 1 fewer cycle** in frame 926. It is not a
phase shift in either direction, and a sum over fetches was never going to predict it.

### What is left

**Eight frames**, every one isolated and seven of them a single live byte: 680, 714, 733, 740, 767,
821, 837, 957. Six are the same frames the standard profile has, and they are §4e's measured
artefact class — the reference records its 2 KiB five to seventeen cycles from where this core reads
it. The profile clears 645, 796 and 875 and moves 767 and 957 in, which is what a change of the
reading instant does.

**And beyond frame 1 100 there is nothing to measure against.** `kirby-jp.txt` carries a checksum
and a lag count every hundred frames and no `ram=` field, which is what `--ram-ref` and
`frame-trace --converge` need; a checksum is taken over the whole 2 KiB including the two artefact
ranges, so it disagrees on nearly every line *even in the stretch where the live memory agrees byte
for byte* (2 of 1 563 lines on the standard profile, 1 on the other, all 1 563 lag counts agreeing
on both). The picture at frame 1 000 matches and the picture at 10 000 does not, so the two machines
part somewhere in between, and no reference exists in that interval. **A per-frame `ram=` dump of
frames 1 100 to 10 000 would localise the next difference in one command** — and adding the item it
names is now a half-day's work rather than a lot's, which is the point of the architecture.

### What this cost the standard profile: nothing that can be measured

| | |
|---|---|
| `golden-core`, 98 originals × 1 200 frames | **byte-identical** to lot FCEUX-23's baseline |
| `bench-core`, five ROMs, six interleaved rounds against the parent | −3.0 %, −3.0 %, −2.2 %, **+0.5 %**, **+0.6 %** — worst case inside the +1 % budget |
| `AccuracyCoin` and the 24 DMC/DMA ROMs | unchanged, row for row |
| savestates | unchanged, to the byte and to the format version |
| memory | none — a profile is a zero-sized type |
| binary | +147 408 bytes of `.text` for a whole second core, so it is behind no feature |

`docs/review/NESHAWK-PROFILE.md` is the how-to: what the profile is, what it is not, and the
eight-step recipe for adding an item.

---

## 4g. Frame 4666, and the 4 688 frames after it: item 1's third half (NESHAWK-PROFILE-2)

§4f ended by asking for exactly one thing: *"A per-frame `ram=` dump of frames 1 100 to 10 000
would localise the next difference in one command."* The owner produced it
(`kirby-jp-step1c.txt`, 8 901 frames, and a screenshot every 500), and it said the profile parted
again at frame **4666** with frame 926's own four bytes — the frame semaphore `$0039` still set when
the next NMI landed.

**The result first**, same command, `--profile` the only difference:

| | `standard` | `neshawk`, §4f's four items | `neshawk`, with item 1c |
|---|---|---|---|
| frames compared (1 100-10 000) | 8 901 | 8 901 | 8 901 |
| **the last frame with no live difference** | 925 | **4 665** | **9 353** |
| frames with no difference outside the stack page and `$0000-$001F` | — | 3 531 of 8 901 | **8 175** of 8 901 |
| frames with a live difference | — | 5 370 | **726** |
| all 8 901 lag counts | agree | agree | agree |
| the owner's 18 screenshots, 1 500 to 10 000 | **0** MATCH | — | **15** MATCH |

### The instrument, because aggregate counts could not do it

`frame-trace` could say *"52 DMC sample fetch(es)"* for frame 4666 and no more. This lot built
**`frame-trace --dmc-census`**: every DMC request with the earliest cycle its halt was allowed on,
every halt, every fetch with the cycles it cost, the OAM transfers, every APU register write and
every `$4015` and `$2002` read, each placed against the instruction it happened inside and carrying
the DMC's own `(timer, bits_remaining, bytes_remaining)`.

It is behind the `dma-census` feature, **off by default, with `#[cfg]` on each of its nine call
statements** so the shipped core is the core without them — which is what keeps `golden-core` and
`bench-core` answerable at all.

### What it said, which was a negative and the most useful thing in the lot

**Frame 4666 is entirely ordinary.** 52 fetches, every one a plain reload costing exactly four
cycles (`the costs: 52x4`), one OAM transfer, **no `$4015` write, no `$4010`-`$4013` write, no
`$2002` read**, one `$4015` read — and that read is `LDA $4015` at **`$823C`**, §4e's amplifier,
which returns `1F` so that the `BNE` at `$8241` is taken, the arm that does nothing.

So **neither of the two candidates §4f left standing has a site in that frame**: no sample end for
item 6's aborts, no register write for item 5's alignment, no mid-sample enable for item 3. And the
miss was measured: `$0039` is raised by `INC $39` at `$C37C` in the NMI handler and cleared by
`STA $39` at `$C06E` when the frame's work finishes; frame 4666 never reaches `$C06E`, and the
stretch from `$F06F` to `$C06E` costs **394 cycles** where the frame has slack against the **177**
frame 4666 has left when it gets there. **About 220 cycles short, against a whole-frame DMA bill of
721** — so no change to the cost of a fetch could close it by arithmetic.

### The item, which is item 1's third half

Item 1 was split into **1a** (the halt lands on the request cycle) and **1b** (a fixed four
cycles), both shipped in §4f. The `+2` that NESHAWK-PROFILE step 1 quoted was never implemented,
and with it the reason for it:

* `DMCUnit.Run()` lowers `RDY` on the request cycle **whatever the CPU is doing there**, and
  `MOS6502X`'s write microstep runs unconditionally and never sets `rdy_freeze`;
* `RunCpuOne`'s `if (!cpu.RDY && !cpu.rdy_freeze && countdown == DMC_RDY_check) countdown += 2`
  then gives the fetch two cycles back once the CPU has been writing for three cycles.

Zaxnes has neither half: a request raised on a write cycle waits for the **next read cycle**. Let
`w` be that wait — how many cycles in a row the CPU was writing. NesHawk's stalled read runs
`w + 3` cycles after the request for odd `w` and `w + 4` for even; this core always spends `w + 4`.
So they agree on an even `w` and NesHawk is **one cycle ahead on an odd one** — and a 6502 never
writes on more than three cycles in a row (the three pushes of an interrupt or `BRK`; two for
`JSR` and for a read-modify-write), so `w ≤ 3` and one bit of `w` is the whole rule rather than an
approximation. `DMC_HALT_LANDS_ON_A_WRITE_CYCLE` drops the second re-read on an odd `w`.

In frame 4666 it fires **six** times — `the costs: 6x3 46x4`, 202 cycles instead of 208 — which is
a thirty-seventh of the 220-cycle miss. **The arithmetic was written down in advance, was right
about the size, and was wrong about what it meant, for the second lot running**: item 1a closed
frame 926 against an arithmetic of the wrong *sign*, and item 1c closes frame 4666 against one two
orders of magnitude too small. What these items move is *which cycle* each fetch lands on, and with
it the interleaving of a whole frame.

### Two items that are real and exactly inert, and two eliminated

| | |
|---|---|
| **item 3's parity, corrected** | NesHawk's DMC timer is **this core's plus one** — it reloads to `DMC_RATE[i]` (428) where this core reloads to `rates[i] - 1` (427), both read after the same cycle's clock — so every `timer % 2` taken from its source has to be inverted. Item 3 shipped with the source's text and the wrong convention. Corrected, it moves the sample's whole phase by a cycle (`d=3` where it read `d=2`) and leaves the **8 901-frame table byte-identical** |
| **item 7, `dmc_realign`** | NesHawk's OAM block is gated on `dmc_dma_countdown != 1 && !dmc_realign` and the DMC sets `dmc_realign` on the fetch cycle, so a transfer a fetch falls inside loses **two** cycles where this core's loses one. Implemented, tested, and the table is again **byte-identical**: *Kirby* has one such fetch a frame |
| **item 6's end-of-sample aborts, eliminated** | `fill_glitch` and `fill_glitch_2` are set only at a fetch that *completes* with `out_bits_remaining == 0`, and `DMCUnit.Clock` makes that the last bit period of a byte — seven bit periods from where a fetch can complete at *Kirby*'s 72 and 106 cycles a bit. **Neither can ever fire on this movie**, proved from the source and the measured rate rather than tried |
| **item 5, not implemented** | the `pending_reg` dispatch defers every APU register but `$4003`, `$4007`, `$4010`, `$4015` and `$4017` to an even DMC timer, and a second write while one is pending **loses the first**. Of those only `$4012`/`$4013` can move a fetch, and the address they set is not read for hundreds of cycles — so it is **CPU-timing-neutral here** and would touch `Apu2A03::write` for every game. It stays the first item to add for a game whose *audio* differs |

### Where it parts now: frame 9355, and it is the sound engine

`frame-trace --converge` classifies 9354 as the **artefact** class (an instant of our frame holds
the reference's 2 KiB exactly) and **9355 as real**. `$0641,X` is the per-channel note-duration
counter of the music driver at `$80xx`-`$83xx`; this core writes two of its channels at
instructions 8 697 and **8 733 of frame 9354, seven instructions before the frame boundary**, and
the reference's 2 KiB holds the state before them. In frame 9355 the driver reads one back and
branches on it, and the sound state never converges again.

So it is frame 4666's shape one routine over — a per-frame pass crossing the vblank — **with the
sign reversed**: there this core was late, here it is early by about **thirty cycles** in a frame
whose bill is 274 DMC cycles and one 513-cycle OAM transfer. No difference NesHawk's source shows
supplies thirty cycles on that frame, which is why this lot stops at three rounds of its allowed
five and says so.

### No regression, and the standard profile is faster

| | |
|---|---|
| `golden-core`, 98 originals × 1 200 frames | `8d508070…`, **byte-identical** |
| `bench-core`, six interleaved rounds against `428c50e6` | Super Mario Bros. 3 **−2.24 %**, Mega Man 3 **−1.04 %**, faster in eleven of twelve rounds |
| the 30 DMC/DMA/APU ROMs, `--profile standard` | 24 pass, 6 manual — unchanged |
| `AccuracyCoin`, `--profile standard` | 128/147 — unchanged |
| `AccuracyCoin`, `--profile neshawk` | 120/147 → **122/147**: `DMA + $2002 Read` and `DMA + $4015 Read` now pass |
| *Faxanadu*, both dumps, both profiles | 207 and 220 of 832, last agreeing line 83 200, every lag count agreeing, the same final picture hashes — §4's figures to the hash |
| the suites | `zax-nes` **1 201**, `zax-harness` **127** (and 127 with `dma-census`, 147 with `fceux-core`), 0 failures |

---

## 5. What is still unknown

Said plainly, because the result in §4 is strong enough to be worth bounding.

* ~~**One `.bk2`, one game, one core.** *Faxanadu* is MMC1 with CHR-RAM. A `.bk2` of an MMC3
  game, or of a board with its own IRQ, has not been tried, and the first divergence of such a
  movie would be the interesting one.~~ **Answered, EVAL-SYNC-2, and the answer is the bad one:**
  §4b. An MMC3 `.bk2` with seven resets does not sync, and the first divergence is inside the
  first hundred frames.
* **`QuickNes` is not `NesHawk`.** `BizHawk` ships two NES cores; this movie names `NesHawk` in
  its header. A `.bk2` recorded on `QuickNes` would be a movie recorded on a *fast* core and
  should be expected to behave like the `.fm2` movies do, not like this one. The reader records
  the core's name in a note for that reason.
* **A savestate-anchored `.bk2` cannot be played at all**, by anything here, and no amount of work
  on this side changes that: the anchor is a state of another emulator's core.
* **`RegionOverride`** non-zero is refused rather than guessed, so a PAL `.bk2` that sets it is
  out of reach until someone can say what the values mean from a public source.
* ~~**The six frames of §4 are six frames**, spread over 83 218 and including the last. A
  frame-by-frame comparison against `BizHawk` would need `BizHawk`'s own per-frame output; the
  CRC-per-100-frames column of the supplied log is one, but what it is a CRC *of* is not in the
  public description, so it was not used.~~ **Used, EVAL-SYNC-2**: the owner stated the
  definition, §1b reproduces it, and *Faxanadu* now has 832 lines of agreement to show for it
  rather than six frames.

Three new ones, all from EVAL-SYNC-2 and all about §4b:

* ~~**Why *Kirby* parts is not known.** Six candidate causes were eliminated by measurement and two
  remain that cannot be reached from outside the core. Until the `cfg`-gated experiment of §4b is
  written, "a Zaxnes accuracy gap in the first sixty frames" is a location, not a diagnosis.~~
  **Answered, EVAL-SYNC-3, and it was not an accuracy gap at all:** the instrument was playing the
  game's 3D widescreen hack, whose overclock grants the CPU free cycles no NES gives (§4c). The two
  surviving candidates are eliminated and the `cfg`-gated experiment should not be written.
* ~~**The reference cannot see below frame 100.** Everything §4b says about *where* rests on one
  line of a 100-frame grid plus the structure of the movie. A trace at a finer step over the
  first few hundred frames — the owner's script with `step = 1` — would turn a hundred-frame
  window into a one-frame answer, and it is the cheapest thing anyone could do next.~~
  **Done, EVAL-SYNC-3**: the owner's `step = 1` trace of the first 400 frames is what found it, and
  it took one run of `--ram-ref` to read.
* **Two `.bk2` movies is still two.** One syncs exactly and one does not; the sample does not say
  what fraction of published `.bk2` movies Zaxnes would replay.

Three new ones, from EVAL-SYNC-3:

* ~~**Where *Kirby* parts now is still unknown**, but the window is **frames 401 to 1 000** instead
  of the first sixty, and the first difference is one object on screen rather than a different
  game. §4c names the measurement that would close it — the same Lua script over frames 400 to
  1 100 — and says that nothing is needed from this side first.~~ **Located, EVAL-SYNC-4:** the
  start of frame **926**, three live bytes, of which two are the artefact and one is the game's own
  frame-sync semaphore `$0039` (§4e). *Why* it parts is still undetermined, but it is now one
  mechanism and one number: Zaxnes's frame-926 work ran about **sixty CPU cycles** behind
  `NesHawk`'s.
* ~~**The reference's reading point inside the frame is not published.** … `--converge` can
  *prove* a frame is a reading artefact rather than a divergence, but it cannot say by how many
  dots the two reading points are apart.~~ **Measured, EVAL-SYNC-4**, and it can: **5 to 17 CPU
  cycles** — 15 to 51 PPU dots — the reference reading *earlier*, on eight independent frames whose
  frame boundary falls inside the same copy loop (§4e). What is still not published is the *rule*
  behind that offset, which is why item 1 of §4e's list asks for it directly.
* **No other game with a built-in 3D patch has been held against a reference.** The fix of §4c is
  general — the patches are off for every `movie-sync` run now — but *Kirby* is the only movie in
  hand whose game has one, so the only evidence that the fix is *sufficient* is *Kirby*'s own.

Three new ones, from EVAL-SYNC-4:

* ~~**Whether the DMC's sample-fetch DMA is the cause is not proved, only sized.** It is the only
  mechanism in the frame with the budget … §4e's items 3 and 4 are what would turn it into a
  cause.~~ **Proved, NESHAWK-PROFILE:** it is the cause, and the item is **which cycle the halt
  lands on** rather than how many cycles the fetch costs. Item 1a alone takes the last frame with
  no live difference from 925 to 1 100; item 2 — the `$4015` bit-4 window §4e named as the
  amplifier — is *exactly* inert on this movie (§4f).
* **Zaxnes's post-halt DMA abort is a known gap and its worth on a real game is unmeasured.**
  `AccuracyCoin`'s `Explicit DMA Abort` and `Implicit DMA Abort` have failed since lot MF-4a with a
  written diagnosis; the arithmetic of §4e says one to three cycles per sample end, which is
  twenty times too small for frame 926 — but "too small for *this* divergence" is not "harmless",
  and no lot has yet measured it on a game.
* **Frame 733 remains the odd one out.** It is the same routine and the same instruction as the
  other eight isolated frames, an instant of our frame holds the reference's live memory exactly,
  and yet the offset is **170** cycles where the other eight are 5 to 17. Either the reading point
  is not at a fixed offset from vblank — which item 1 of §4e's list would say outright — or the
  same mechanism as frame 926 was at work three hundred frames earlier without changing the
  program's path.

Two new ones, from NESHAWK-PROFILE:

* ~~**Where *Kirby* parts now is beyond frame 1 100, and no reference can see it.** … **A
  per-frame `ram=` dump of frames 1 100 to 10 000 would name it in one command**, and that is the
  single most valuable thing the owner could produce for this question now.~~ **Produced, and it
  did exactly that (NESHAWK-PROFILE-2):** `kirby-jp-step1c.txt` named frame **4666** in one
  command, and item 1c then took the last clean frame to **9 353** (§4g).
* **Items 2 and 3 are implemented on a reading, and only item 3 has been seen to fire.** Item 2
  changes nothing on either movie in hand, so the only evidence that it is right is NesHawk's
  source; a movie whose `$4015` read lands inside its three-cycle window would be the test.
  NESHAWK-PROFILE-2 adds item 3 to that list: with its parity corrected to the source's own
  convention it moves every sample's phase by a cycle and changes **not one byte** of 8 901 frames,
  so it too rests on the source alone.

Four new ones, from NESHAWK-PROFILE-2:

* **Where *Kirby* parts now is frame 9 355, and the reference covers it — so the gap is a
  mechanism, not data.** The sound driver's per-frame pass crosses the vblank about thirty cycles
  differently in frame 9 354, and **nothing in NesHawk's DMC, DMA or APU-register paths supplies
  thirty cycles on that frame**: item 1c is worth six, item 7 one, items 3 and 5 nothing, and item 6
  cannot fire at these rates. So the next difference is probably **not** on the paths these two lots
  read, and the next lot should read somewhere else — the frame sequencer and `$4017` (written every
  frame, and what decides the four length-counter bits of the `$4015` the driver reads), or the
  `$4016` strobe delay `RunOneLast` applies on an odd DMC timer, are the two the reading passed over
  because *Kirby*'s DMA did not need them.
* **The reference's reading point is not at a fixed offset after all.** EVAL-SYNC-4 measured 5 to 17
  CPU cycles on eight frames and §5 already recorded frame 733's **170** as the odd one out. The
  third window makes that the rule rather than the exception: of the 79 isolated artefact frames
  below 9 354, most match at −6 to −1 instructions but frames 1 254, 2 082, 2 893 and 9 354 match
  at **−58, −89, −1 486 and −97 to −44**. Until the rule behind the offset is published, a
  `--converge` verdict of "artefact" on a frame with a large offset is weaker evidence than one at
  −6, and frame 9 354 is exactly such a frame.
* **Eleven frames below 4 666 were real, transient divergences and nobody knows what they were.**
  §4g's window has 79 isolated differing frames; before item 1c there were 35, and
  `frame-trace --converge` found **11 of them real** (no instant of the frame agrees) **and
  self-healing** — 1 752, 1 776, 2 072, 3 355, 3 356, 4 168, 4 338, 4 402, 4 475, 4 576, 4 616.
  Every one had a DPCM sample playing and the artefact class included frames with none, which is
  what pointed this lot at the DMA; frame 4 475 even carries frame 4666's four bytes with the signs
  reversed. Item 1c moved the set rather than emptying it (a cluster at 1 751-1 763 is new and
  heals), and **no lot has explained a self-healing one-frame divergence**.
* **A frame whose work RAM agrees byte for byte can still differ by four pixels.** Frame 8 500 is
  the one disagreement among the 15 matching screenshots and its line is `live=0`, as frame 400's
  single pixel was in §4f. Two instances now, both on frames with no live difference at all, so it
  is a PPU or sprite-evaluation difference rather than a timing one — and it is the only thing in
  this series that the work-RAM instrument cannot see at all.

---

## 6. Is a third engine worth it?

**No.** The recommendation is to integrate neither `NesHawk` nor any other core, and the reasons
are the measurements above rather than a preference.

**1. The job `NesHawk` would be integrated to do, Zaxnes already does.** The case for a third
engine was: `.bk2` movies are published by TAS sites, they are recorded on a cycle-accurate core,
and Zaxnes's own engine is cycle-accurate too, so they ought to sync. They do — §4 is that, at
83 218 frames, on two dumps, with the lag count matching to the frame. Adding `NesHawk` to play
movies Zaxnes already plays exactly would buy nothing.

> **Stronger still, NESHAWK-PROFILE.** The one movie that did not sync now does, for the whole
> 1 100 frames any reference covers, and it took **four constants** rather than an engine: the
> four NesHawk timing choices of §4f, behind a timing profile of Zaxnes's own core. That settles
> the shape of the answer for good. What a `.bk2` needs from `NesHawk` is not `NesHawk`; it is the
> handful of places where `NesHawk` differs, and each of those can be read, written down, tested
> against the public ROMs on both profiles, and switched on by the movie's own header — at **no**
> cost to the standard chip (`golden-core` byte-identical, worst bench +0.6 %, savestates
> unchanged). A CLR host or a port would have bought the same sync and a licence question with it.

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
   *NESHAWK-PROFILE:* it must switch the **timing profile** instead, which is a different thing and
   already built — `Engine` is implemented for both profiles and `MovieFile::recorded_profile` is
   the one-line rule. As with FCEUX-48's engine switch it is for that playback only, is never
   written to the game's own setting, and is said on screen. Nothing in the application names
   `.bk2` today (§4f), so this is still the whole of the work.
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

### EVAL-SYNC-2: that movie arrived, and the recommendation is still no

*Hoshi no Kirby* is exactly the movie the paragraph above asked for — MMC3, seven resets, syncs
on `BizHawk` — and it does not sync on Zaxnes (§4b). So the question **is** open again, and this
is the answer with the new fact in it.

**Reason 1 no longer holds and reason 2 is unchanged.** "The job `NesHawk` would be integrated to
do, Zaxnes already does" is now "Zaxnes does it for one of the two `.bk2` movies in hand". The
`.fm2` half is untouched: those still fail because FCEUX is wrong and recorded them, and the
existing second engine already covers that.

**Reasons 3 and 4 are untouched, and they are the ones that decide it.** The cost of a C# core
inside a Rust desktop build has not changed, and neither has the menu a user has to be able to
read. What has changed is that there is now a **bug of ours to find**, and a third engine is the
one response that would make finding it *less* likely: the movie would play, the gap would stay,
and every future `.bk2` would route around an accuracy difference nobody had measured.

**So: no third engine — and the work it would have hidden becomes the first item below.** The
honest form of the recommendation is now "not on this evidence", not "and nothing would change
it". What would change it is a second negative result of a different kind: a `.bk2` that fails on
Zaxnes for a reason that turns out **not** to be fixable in `zax-nes` — a `NesHawk`-specific
behaviour rather than an accuracy gap. §4b found no such thing; it found a location it could not
reach.

### What to do instead — revised order (EVAL-SYNC-2)

0. **Find the *Kirby* divergence.** It is the only item here that is a defect rather than a
   feature. The next move is cheap and is the owner's: run the Lua script again on *Kirby* with
   `step = 1` over the first few hundred frames. That turns §4b's hundred-frame window into a
   single frame, and with a single frame the two surviving candidates separate at once. After it,
   or instead of it, the `cfg`-gated experiment of §4b (a power-on PPU dot and the 29 658-cycle
   write-inhibition window, twenty or thirty lines in `Ppu2C02`, feature off by default).
1. **The soft reset should leave the mapper alone** (§4b). One line of code and an amendment to
   REQ-CORE-122, which currently says the mapper follows a documented *after reset* state that
   does not exist — the cartridge connector has no reset pin. It is not *Kirby*'s cause, and it
   is wrong on every MMC3 game a player ever presses RESET on.
2. **Make the application play `.bk2`** — as before, and on the Zaxnes engine, with the two
   things EVAL-SYNC-2 added to the reader: the power-on work RAM is the
   `00 00 00 00 FF FF FF FF` pattern, not zeroes, and a non-`null`
   `InitialWRamStatePattern` is refused by name. Three lines and four lines inside that work.
   One caveat that was not there before: a `.bk2` of an MMC3 game may desync, so the feature
   should not be sold as "plays TAS movies" until item 0 is closed.
3. **The three header fields the application's Zaxnes `.fm2` path ignores**, unchanged.
4. **Say the `.fm2` situation out loud in the interface**, unchanged.

### EVAL-SYNC-3: reason 1 is restored, and the answer is still no

The movie EVAL-SYNC-2 called "a bug of ours to find" was a bug of ours — in the **instrument**, not
the engine (§4c). So the paragraphs above need correcting in the place a reader met them.

**Reason 1 holds again.** "The job `NesHawk` would be integrated to do, Zaxnes already does" is
once more the plain statement it was in §6: *Faxanadu* replays identically at every frame compared
on both dumps with the lag count exact, and *Kirby* now matches `NesHawk`'s work RAM **in every
live address at every one of the 400 frames** the owner's finest reference covers, with its own lag
count over 156 312 frames reading **24** — `BizHawk`'s figure to the frame. What is left of
*Kirby*'s divergence is one object on screen from somewhere in frames 401-1 000, and §4c names the
single measurement that would locate it.

**Reasons 2, 3 and 4 are untouched.** FCEUX is still the engine the `.fm2` movies need and still
the one that recorded them; a C# core inside a Rust desktop build still costs what it cost; and the
menu a user has to be able to read is unchanged.

**And the argument has gained one reason it did not have.** EVAL-SYNC-2's own sentence — "a third
engine is the one response that would make finding it *less* likely: the movie would play, the gap
would stay, and every future `.bk2` would route around an accuracy difference nobody had measured"
— is now a **demonstrated** claim rather than a prediction. The difference `NesHawk` would have
routed around was not in the emulation at all: it was `movie-sync` handing the core a widescreen
hack. A third engine would have replayed the movie perfectly and buried that, and the two earlier
lots would have had nothing to find.

**So: no third engine.** The thing that would change it is unchanged from §6's own formulation — a
`.bk2` that fails on Zaxnes for a reason that turns out **not** to be fixable on this side. Three
lots have now looked for one and found: one class (b) family in FCEUX, one class (c) in the reader,
one class (c) in the instrument, one class (a) in `zax-nes` (fixed, §4d), and one window of 600
frames still open.

### EVAL-SYNC-4: the window is closed, and the answer is still no

The measurement item 0 asked for arrived, and it moved *Kirby* from "in step for 400 frames" to
**"in step for 925, with ten screenshots matching exactly"** — and then named the frame, the byte,
the instruction and the branch at which it parts (§4e).

**Reason 1 is stronger than it was.** *Faxanadu* replays identically at every frame compared on
both dumps with the lag count exact; *Kirby* now agrees with `NesHawk`'s work RAM in **every live
address at 517 of the 701 frames** the second reference covers, with no live difference at all
through frame 925, **all 701 lag counts** agreeing, and ten of eleven screenshots in that stretch
matching under the colour-bijection rule. What is left is **three bytes at frame 926**, two of them
a reading artefact now measured at five to seventeen cycles.

**Reasons 2, 3 and 4 are untouched**, and reason 4 gains a sharper form: what remains is a
difference of **tens of CPU cycles inside one frame**, on a mechanism where Zaxnes passes 52 of 58
public ROMs and fails none, and `NesHawk`'s own figure is unpublished. Integrating a third engine
to make that difference go away would hide the one open accuracy question on the list rather than
answer it — which is EVAL-SYNC-2's own argument, now at its third demonstration.

### What to do instead — revised order (EVAL-SYNC-4)

0. **Close frame 926, and it is four lines of Lua rather than a lot.** §4e names them: the reading
   instant (`emu.totalexecutedcycles`, `emu.getregister("PC")`) beside each `ram=` line; the
   frame-sync point (`event.onmemoryexecute` on `$C06E` and `$C37C`); the `$4015` read
   (`event.onmemoryread`); and the per-frame cycle total (`event.onframeend`). The first of them
   alone settles the whole artefact class and gives frame 926 its sign. **Nothing is needed from
   this side first, and in particular no change to `zax-nes`.**
1. **Decide whether the post-halt DMA abort is worth a lot of its own** (§4e). It is a named
   Zaxnes gap with a written diagnosis (MF-4a §Step 3), two failing `AccuracyCoin` rows, a fix that
   was built and withdrawn, and an open ticket (`Q-119`). It is **not** frame 926's cause — it is
   twenty times too small — but it is the only DMA behaviour this build is known to get wrong, and
   it has never been measured on a game.

### What to do instead — revised order (EVAL-SYNC-3)

0. ~~**Finish locating the *Kirby* divergence.** Still the only defect on the list, and still the
   owner's cheapest move: the same Lua script over frames **400 to 1 100** with `step = 1`
   (§4c).~~ **Done, EVAL-SYNC-4 §4e**: the owner ran it and the divergence is the start of frame
   926. The list above replaces this one.
   The `cfg`-gated experiment of §4b is **struck**: its two candidates are eliminated.
1. ~~**The soft reset should leave the mapper alone.**~~ **Done, EVAL-SYNC-3 §4d**, with the
   proposed amendment of REQ-CORE-122 written up in
   `docs/review/AMD-REQ-CORE-122-mapper-reset.md` for the spec owner to adopt. golden-core is
   byte-identical.
2. **Make the application play `.bk2`** — as before, on the Zaxnes engine, with the reader's two
   EVAL-SYNC-2 additions and one of EVAL-SYNC-3's: a `.bk2`'s `Reset` happens **after** the frame
   whose bit is set. The caveat softens but does not go away: an MMC3 `.bk2` now holds for at
   least 400 frames rather than fewer than 100, and item 0 is what would let the feature be sold
   as "plays TAS movies".
3. **A note for whoever next measures accuracy against a reference**, and it is not about movies:
   `NesCore` powers on with the 3D built-in patches **enabled**, so any harness that loads a
   recognised commercial ROM and does not turn them off is measuring the widescreen hack.
   `test-roms` turns them off, `movie-sync` and `frame-trace` now do, and anything written next
   should too.
4. **The three header fields the application's Zaxnes `.fm2` path ignores**, unchanged.
5. **Say the `.fm2` situation out loud in the interface**, unchanged.

### The commands (EVAL-SYNC-2)

```sh
H=./target-ev/release/zax-harness.exe
M=G:/3DSen/test-movies
K="$M/bk2/keyword-kirbysadventure-noabilities.bk2"
KR="$M/roms/Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes"
B=$M/bizhawk-runs

# the calibration, and what a run in step looks like
$H movie-sync "$M/bk2/invariel-faxanadu.bk2" "$M/roms/Faxanadu [USA][!!].nes" \
    --engine zaxnes --lag --trace fax.txt --trace-ref "$B/fax-usa.txt"

# the whole Kirby movie against BizHawk's own run
$H movie-sync "$K" "$KR" --engine zaxnes --lag --trace kirby.txt \
    --trace-ref "$B/kirby-jp.txt" --progress 50000

# the six eliminations, one line each
for r in on off early late;          do $H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 3000 --resets   $r --trace-ref "$B/kirby-jp.txt"; done
for p in default zero ff;            do $H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 3000 --ram      $p --trace-ref "$B/kirby-jp.txt"; done
for p in default zero ff;            do $H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 3000 --prg-ram  $p --trace-ref "$B/kirby-jp.txt"; done
for p in default zero ff random:0;   do $H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 3000 --ppu-ram  $p --trace-ref "$B/kirby-jp.txt"; done
for w in 1 2 3 4;                    do $H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 3000 --warm-up zaxnes:$w --trace-ref "$B/kirby-jp.txt"; done
$H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 3000 --ppu-warmup --trace-ref "$B/kirby-jp.txt"

# the pictures, for the six screenshots
$H movie-sync "$K" "$KR" --engine zaxnes --no-lag --png <dir> \
    --png-frames 1000,10000,30000,60000,83000,156312

# the class (a), with no movie and no reference
cargo test --release -p zax-harness --test soft_reset_mapper_tests
```

The screenshot comparison is the same short script as §4 and is not committed (it reads pictures
of a commercial game); the rule is the paragraph in §4.

### The commands (EVAL-SYNC-3)

```sh
H=./target-ev/release/zax-harness.exe
M=G:/3DSen/test-movies
B=$M/bizhawk-runs
K="$M/bk2/keyword-kirbysadventure-noabilities.bk2"
KR="$M/roms/Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes"
R=$B/kirby-jp-step1.txt      # the per-frame reference: checksum, lag AND the 2 KiB

# the first 400 frames, byte for byte and lag for lag -- the whole of 4c's first table
$H movie-sync "$K" "$KR" --engine zaxnes --lag --frames 400 --trace-step 1 \
    --trace-ref "$R" --ram-ref "$R"
# and what it was before the fix, which is the other half of that table
$H movie-sync "$K" "$KR" --engine zaxnes --lag --frames 400 --trace-step 1 \
    --trace-ref "$R" --ram-ref "$R" --builtin-patches

# the Reset semantics: the default against the three alternatives
for r in on early late off; do
  $H movie-sync "$K" "$KR" --engine zaxnes --lag --frames 400 --trace-step 1 --resets $r \
      --trace-ref "$R" --ram-ref "$R"
done

# frame 247, instruction by instruction, with $004B watched and the convergence probe
$H frame-trace "$K" "$KR" --frame 247 --watch 004B --out f247.txt --context 16 \
    --converge "$R" --extra 600
# the same with the hack on, which is where the 9 689 free instructions are
$H frame-trace "$K" "$KR" --frame 244 --frames 9 --converge "$R" --extra 1500 --builtin-patches
# what the game uses mid-frame: no $2002 read, one NMI, one MMC3 IRQ at scanline 174
$H frame-trace "$K" "$KR" --frame 1000 --out f1000.txt
grep -c '\$2002' f1000.txt; grep handler f1000.txt

# the whole movie, the lag count and the six pictures
$H movie-sync "$K" "$KR" --engine zaxnes --lag --trace kirby.txt \
    --trace-ref "$B/kirby-jp.txt" --progress 50000 \
    --png shots --png-frames 1000,10000,30000,60000,83000,156312

# Faxanadu, the control: no built-in patch, so nothing about it can change
for d in "USA" "USA][Rev 1"; do
  $H movie-sync "$M/bk2/invariel-faxanadu.bk2" "$M/roms/Faxanadu [$d][!!].nes" \
      --engine zaxnes --lag --trace-ref "$B/fax-usa.txt"
done

# the .fm2 half: FCEUX's own movie mode as the reference for its own command timing
#   (a synthetic 60-frame movie on nestest.nes with one soft reset; the test does this too)
$H movie-sync reset20.fm2 "$ROMS/nestest.nes" --engine fceux --movie-mode --no-lag \
    --trace-step 1 --trace own.txt
for r in early on late off; do
  $H movie-sync reset20.fm2 "$ROMS/nestest.nes" --engine fceux --no-lag \
      --trace-step 1 --resets $r --trace fed-$r.txt
  diff <(grep -E '^[0-9]' own.txt) <(grep -E '^[0-9]' fed-$r.txt) | grep -c '^<'
done

# the hardware side: the public suites that cover what Kirby uses
for f in mmc3_test mmc3_test_2 ppu_vbl_nmi instr_timing cpu_timing_test dma; do
  $H test-roms G:/3dsen/cleanroom/assets-free/test-roms "tr-$f.json" \
      --filter "$f" --no-shots --quiet
done

# the mapper's soft reset
cargo test --release -p zax-nes --test soft_reset_tests
cargo test --release -p zax-harness --test soft_reset_mapper_tests
$H golden-core "G:/3DSen/test-originals" golden.jsonl 1200   # 8d50807085...
```

The screenshot comparison is the same short script as §4 and is still not committed (it reads
pictures of a commercial game). EVAL-SYNC-3 validated it before believing it: on *Faxanadu*
`[USA][!!]` it reports MATCH at all six frames with §4's own colour counts (9, 14, 11, 13, 12, 12),
and it reports the count of differing pixels so that "275 of 57 344" and "50 537 of 57 344" can be
told apart.

### The commands (EVAL-SYNC-4)

```sh
H=./target-ev/release/zax-harness.exe
M=G:/3DSen/test-movies
B=$M/bizhawk-runs
K="$M/bk2/keyword-kirbysadventure-noabilities.bk2"
KR="$M/roms/Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes"
R=$B/kirby-jp-step1b.txt      # the second per-frame reference: frames 400 to 1100

# the whole window, byte for byte and lag for lag, with the per-frame table
$H movie-sync "$K" "$KR" --engine zaxnes --lag --frames 1100 --trace-step 1 \
    --trace-ref "$R" --ram-ref "$R" --ram-ref-table table.txt
awk '!/^#/ && $3 != "live=0"' table.txt        # the 184 frames, nine of them isolated

# the pictures, at the owner's fifty-frame grid and one frame either side of it, so that the
# pairing of `s<f>.png` is measured rather than assumed
$H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 1100 --png shots \
    --png-frames 399,400,449,450,...,1099,1100

# the eight isolated frames: the instruction, and the cycles the reference reads early
for fa in 645:04BE 680:04E0 714:0405 740:04AE 796:0490 821:044F 837:04FE 875:042C; do
  f=${fa%%:*}; a=${fa##*:}
  $H frame-trace "$K" "$KR" --frame $f --watch $a --converge "$R" --extra 1500 --out f$f.txt
done
$H frame-trace "$K" "$KR" --frame 733 --watch 0040 --watch 0048 --watch 0493 --watch 0498 \
    --converge "$R" --extra 1500 --out f733.txt
$H frame-trace "$K" "$KR" --frame 733 --watch 0004 --converge "$R" --extra 10

# frame 926: the three bytes, then the live difference either side of the boundary
$H frame-trace "$K" "$KR" --frame 926 --watch 0038 --watch 0039 --watch 0049 \
    --converge "$R" --extra 2000 --context 14 --out f926.txt
$H frame-trace "$K" "$KR" --frame 926 --converge "$R" --extra 200 --converge-window 26
$H frame-trace "$K" "$KR" --frame 925 --frames 3 --converge "$R" --extra 60 --converge-window 4

# what $0039 is, and the one frame of twelve that never clears it
$H frame-trace "$K" "$KR" --frame 918 --frames 12 --watch 0039 --watch 01A5 --context 2

# the DPCM census of the whole window -- 701 frames, five seconds
$H frame-trace "$K" "$KR" --frame 400 --frames 701 | grep -E '^  frame|DMC sample'

# the $4015 read and the branch it decides
for f in 925 926 927 928; do
  $H frame-trace "$K" "$KR" --frame $f --out g$f.txt >/dev/null
  grep -A4 'LDA \$4015' g$f.txt | head -5
done

# the sample the game starts at frame 923, and its rate
$H frame-trace "$K" "$KR" --frame 923 --out f923.txt
grep -E 'ST[AXY] \$40(10|11|12|13|15)' f923.txt

# the hardware side: every public suite that covers DMC DMA, the APU and DMA cycles
export ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms
for f in dmc_dma_during_read4 sprdma_and_dmc_dma dmc_tests apu_test blargg_apu_2005 \
         apu_reset cpu_interrupts_v2 instr_misc AccuracyCoin; do
  $H test-roms "$ZAXNES_TEST_ROMS" "tr-$f.json" --filter "$f" --no-shots --quiet
done
```

The screenshot comparison is still the same uncommitted script, and EVAL-SYNC-4 revalidated it a
second way: it scores **10** MATCHes of 15 on one pairing of the owner's screenshots and **1** on
the other, which a script that said MATCH indiscriminately could not do.

### The commands (NESHAWK-PROFILE-2)

```sh
H=./target-ev/release/zax-harness.exe
M=G:/3DSen/test-movies
B=$M/bizhawk-runs
K="$M/bk2/keyword-kirbysadventure-noabilities.bk2"
KR="$M/roms/Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes"
R=$B/kirby-jp-step1c.txt       # the third per-frame reference: frames 1 100 to 10 000

# the window, byte for byte and lag for lag, with the per-frame table (no --profile: the .bk2's
# own Core header picks the NesHawk chip, and the run prints `zaxnes/neshawk`)
$H movie-sync "$K" "$KR" --engine zaxnes --lag --frames 10001 --trace-step 1 \
    --trace-ref "$R" --ram-ref "$R" --ram-ref-table c-table.txt
awk '!/^#/ && $3 != "live=0"' c-table.txt        # the differing frames

# each isolated frame: artefact or real
for f in $(awk '!/^#/ && $3 != "live=0" && $1 < 9354 {print $1}' c-table.txt); do
  $H frame-trace "$K" "$KR" --frame $f --converge "$R"
done

# frame 4666: the live difference either side of the boundary, and the semaphore
$H frame-trace "$K" "$KR" --frame 4666 --converge "$R" --converge-window 40 \
    --watch 0038 --watch 0039 --watch 0048 --watch 0049
$H frame-trace "$K" "$KR" --frame 4664 --frames 4 --out t4664.txt
grep -E 'C06E  85 39|F06F  85 38' t4664.txt       # the deadline, and where the frame ran out

# the DMA census -- an instrument behind a feature that is off by default
cargo build -p zax-harness --release --features dma-census
$H frame-trace "$K" "$KR" --frame 4666 --dmc-census
$H frame-trace "$K" "$KR" --frame 4640 --frames 31 --dmc-census   # a line per frame

# the pictures, at the owner's 500-frame grid, both profiles from the same command
for p in neshawk standard; do
  $H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 10000 --profile $p \
      --png shots-$p --png-frames 1500,2000,2500,3000,3500,4000,4500,5000,5500,6000,6500,7000,7500,8000,8500,9000,9500,10000
done

# the regression set
$H golden-core "G:/3DSen/test-originals" golden.jsonl 1200 && sha256sum golden.jsonl
for p in standard neshawk; do
  for f in dmc_dma_during_read4 sprdma_and_dmc_dma dmc_tests apu_test AccuracyCoin; do
    $H test-roms "$ZAXNES_TEST_ROMS" "tr-$f-$p.json" --filter "$f" --no-shots --quiet --profile $p
  done
done
$H bench-core "G:/3DSen/test-originals/Super Mario Bros. 3 [USA][!!].nes" --frames 1200 --warmup 120
F="$M/bk2/invariel-faxanadu.bk2"
$H movie-sync "$F" "$M/roms/Faxanadu [USA][!!].nes" --engine zaxnes --lag --trace-step 100 \
    --trace-ref "$B/fax-usa.txt" --profile neshawk
```

The screenshot comparison is still **not committed** — it reads pictures of a commercial game. It
is EVAL-SYNC §4's colour-bijection rule with the eight overscan rows cropped top and bottom
(BizHawk writes 256×224, this core the whole 256×240, REQ-CORE-048). NESHAWK-PROFILE-2 revalidated
it a third way: **15 MATCHes of 18 on the NesHawk profile and 0 of 18 on the standard one**, whose
eighteen misses run from 38 914 to 54 963 pixels — which a script that said MATCH indiscriminately
could not produce.
