# RU-3b-fix — the guard-rail of Q-103

Branch `rustupdated`, on top of RU-3b (`d01addd`). Scope: the **answer** of
`spec/changes/Q-103.md` (section « Réponse », R0, 2026-11-02) and nothing else. A profile whose
scripts change the core from a frame hook runs at an **effective** run-ahead of 0, the user's
setting kept; the user is told in both interfaces and in the log.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | The guard: detection at profile load, the effective N, the log line, the tests | done — `8ee28a1` |
| 2 | The note beside the setting in both interfaces, both languages, and its tests | done — `e3e15ec` |
| 3 | Closing: full tests, fmt, clippy, `PERFORMANCE-HISTORY.md` §5.5, the `// Q-103` comments | done |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Notes

### What the answer asks for, and where each piece goes

| Point of the answer | Where |
|---|---|
| 1. a frame hook's write is not replayed | nothing to do — RU-3b already behaves that way (`put_the_real_frame_back`) |
| 2. the effective run-ahead of such a game is 0, the setting kept | `GameSession::apply_run_ahead_setting`, called by the one site that reads the setting (`EmulationHost::run_due_frames`) |
| 3. detected once at profile load, nothing in the frame loop | `scripts_change_the_core` in `game_session.rs`, called in `GameSession::create` beside `scripts_leave_the_core_alone` |
| 4. a note in both interfaces and both languages, plus a log line | `ui/text.rs` + `ui/zax_ui/options.rs` (desktop), `ui/handheld/options.rs` + `ui/handheld/draw.rs` (console), `app_log` at load |
| 5. no shipped profile is concerned | R0's own count; nothing to do here |

### Which `CORE_MEMBERS` qualify (the answer leaves it to the implementer)

`CORE_MEMBERS` (the "emulation ahead" rule) holds six names; the guard-rail needs those through
which a **frame** hook can change the state the restore of step 5 throws away:

| Member | Qualifies? | Why |
|---|---|---|
| `WriteMem` | **yes** | `ctx.machine.write` → the bus, which is in the in-memory state of REQ-CORE-121 |
| `SetInputType` | **yes** | `ctx.machine.set_port_types` → `InputPorts`, also in that state; worse than a lost write, the settings half of the call would persist while the core half was rolled back |
| `GetInput`, `GetInputType`, `ZapperEnable` | no | read only (`port_byte`, `port_type`) |
| `InputProcess` | no | the input hook runs at step (a), **before** the real frame, so what it writes is part of R — the answer says so itself |

The Zapper aim a script writes (`Setting.ZapperPos`, `set_zapper_offset`) is not concerned either:
it lives in `ScriptInputState` on the session's side, not in the core's state.

So `CORE_CHANGING_MEMBERS` is `["WriteMem", "SetInputType"]` — a documented subset of
`CORE_MEMBERS`, kept beside it so the two lists are read together.

### Where the guard is applied, and why not in `set_run_ahead`

`GameSession::set_run_ahead` stays the **raw** setter the amendment's own criterion tests (a write
of 7 reads 3, a write of −1 reads 0, REQ-LOOP-064). The guard sits in
`apply_run_ahead_setting`, the method the loop calls where it already reads the setting once per
emulated frame. Two reasons:

1. The pinning test of Q-103 has to be able to run a `WriteMem` profile at N ≥ 1 to show that the
   write is still lost — the behaviour point 1 of the answer keeps. Its RU-3b comment asks for
   exactly this ("drive the session's run-ahead directly"), which only works if `set_run_ahead`
   is not itself the guard.
2. Frame stepping already calls `set_run_ahead(0)` (`step_without_run_ahead`); a guard inside the
   setter would be dead weight there.

Cost: one `bool` test per emulated frame, next to the three settings the loop already reads there
(catch-up, speed, run-ahead). Nothing is added to the core, to the conversion, to the draw list or
to the frame hooks.

**The harness keeps its explicit `--run-ahead`.** `zax-harness` has its own small host
(`render/rom.rs`) and its flag is R0's measurement instrument; forcing it to 0 behind R0's back
would make a measurement lie about itself. The guard-rail is about the player's setting, which the
harness does not read.

### Step 1 — the guard

`GameSession::run_ahead_forced_to_zero`, a `bool` set once in `create` from
`scripts_change_the_core(profile)`, published by `forces_run_ahead_to_zero()` (what the two
interfaces ask) and honoured by `apply_run_ahead_setting`. The log line at load, in English like
every other `app_log` line:

    Zaxnes: run-ahead: the scripts of this profile change the core from a frame hook, so the
    game runs at run-ahead 0 (Q-103); the setting is kept for the next game.

The user's setting is never written: the model is untouched, so `AppSettings::run_ahead()` still
reads what the player chose and the next game gets it back. `GameSession::run_ahead()` reports
the **effective** value, which is what the core runs and what the loop had already been reporting.

Tests: four more in `zax-app/tests/run_ahead_tests.rs` (18 in all there), one more unit test
beside `scripts_change_the_core`, and the pinning test adapted.

| Test | What it holds |
|---|---|
| `q_103_a_profile_that_writes_the_core_runs_at_zero` | a profile naming `WriteMem` runs at an effective 0 while the setting says 1, and the hook's write now reaches the real state |
| `q_103_a_profile_that_only_reads_the_core_runs_at_the_setting` | `ReadMem` alone is not concerned — N = 1, as set |
| `q_103_the_guard_leaves_the_user_setting_alone` | the model still reads 2 during the forced game, and the **next** game (a profile without the member) runs at 2 |
| `q_103_the_guard_is_decided_when_the_profile_loads` | the verdict holds before a frame has run, and `apply_run_ahead_setting` honours it where `set_run_ahead` does not — including the bounds of REQ-LOOP-064 through the guarded path |
| `only_the_members_that_change_the_core_force_the_run_ahead_to_zero` (unit) | the member list itself, over the six `CORE_MEMBERS` |
| `req_loop_067_a_frame_hook_write_does_not_reach_the_real_state` (adapted) | the pinning test of point 1, now driving `set_run_ahead` directly and running frames with `run_exact_frames`, which bypasses the loop the guard sits in |

**The instrument for "the effective N".** `NesCore::frame_count` cannot measure it — REQ-LOOP-066
makes it count **real** frames only, which is precisely what RU-3b's own tests assert. So
`effective_run_ahead` reads it out of the vector ROM instead: a real frame leaves the counter at
k + 1 and the picture showing k + N, so N is `displayed() + 1 − counter`. That is the acceptance
criterion of REQ-LOOP-065 used as a measurement.

`cargo test --release -p zax-app --features bundled` **647 passed** (the 642 of RU-3b plus these
five), `cargo fmt --check` and workspace clippy clean.

### Step 2 — the note

Desktop (`hint.run_ahead_forced`), beside the row, greyed, the idiom of
`hint.res_scale_direct`:

| Language | Text |
|---|---|
| English | `(forced to 0: this game's scripts write the core)` |
| French | `(forcé à 0 : les scripts de ce jeu écrivent dans le cœur)` |

Console (`handheld.options.run_ahead_forced`), after the value of the Run-ahead row of the
advanced options page, short enough for a handheld row:

| Language | Text |
|---|---|
| English | `(forced to 0: scripts)` |
| French | `(forcé à 0 : scripts)` |

Both come from one pure function per interface, so the visibility is tested without a window:
`ui::options::forced_run_ahead_note(id, forced)` and
`ui::handheld::options::forced_run_ahead_note(row, forced)`, each `None` for any other row and
when nothing forces the value. The two drawing sites read the verdict **once per page** —
`host.session().is_some_and(GameSession::forces_run_ahead_to_zero)` — and not once per row; the
desktop note sits where `hint.res_scale_direct` already sits (`same_line` + `text_disabled`), the
console one after the value of the row.

Neither row is greyed out: the setting is kept and applies again to the next game, so there is
something to say and nothing to disable.

Tests: two more in `run_ahead_tests.rs` (20 in all there), each over a **running** forced game, so
the wiring from the session to the note is covered and not only the strings:
`q_103_the_options_window_notes_the_forced_run_ahead` and
`q_103_the_handheld_page_notes_the_forced_run_ahead` — the note on that row, on no other row, on
neither when nothing forces it, and its text in both languages.

`cargo test --release -p zax-app --features bundled` **649 passed**, `cargo fmt --check` and
workspace clippy clean.

### Step 3 — closing

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --release --features zax-app/bundled -- -D warnings` | clean |
| `cargo test --release -p zax-app --features bundled` | 649 passed |
| `cargo test --release --workspace --features zax-app/bundled` | **2138 passed, 0 failed** |

2138 against RU-3b's 2131: **7 new tests** — 6 in `zax-app/tests/run_ahead_tests.rs` (14 → 20)
and 1 unit test in `game_session.rs`. No test moved, which includes the 20 of
`run_ahead_tests.rs` and the 5 of `separate_core_tests.rs`: every loop test runs on a profile
with no script at all or a script that names nothing, so the guard-rail never fires on them.

`zax-nes`, `zax-settings` and `zax-harness` are untouched by this lot, so the PPU goldens cannot
have moved; they were last reported at 291 of 291 in RU-3b.

**No performance regression, by construction.** Nothing new runs in a per-frame path: the
detection is one text scan of the profile's sources at load, the guard one `bool` test at the site
that already read the setting once per emulated frame, and the two notes are built only while an
options page is drawing. The core, the conversion, the Lua hooks, the draw list and the render are
not touched at all — `git diff` of this lot reaches `zax-app` only, and inside it
`game_session.rs`, `emulation/host/mod.rs`, its run-ahead tests and eight UI files (the keys, the
two string tables, `text.rs`, the two options models and the two drawing sites).
`PERFORMANCE-HISTORY.md` §5.5 loses
its "**that guard-rail is not in yet**" line and says what landed instead.

### The `// Q-103` comments as they now read

| Site | What it says |
|---|---|
| `CORE_CHANGING_MEMBERS`, `scripts_change_the_core` | the member list and why each of the six is in or out |
| `GameSession::create` | the verdict is reached here, once |
| `GameSession::set_run_ahead` | this is the raw setter; the guard is in the method beside it |
| `GameSession::apply_run_ahead_setting`, `forces_run_ahead_to_zero` | the guard itself and what the interfaces read |
| `GameSession::put_the_real_frame_back` | the write is still not replayed, and the restore now only loses the writes of a profile the text test did not see |
| `EmulationHost::run_due_frames` | why the guard is applied there and not in the setter |
| `ui::options::forced_run_ahead_note`, `ui::handheld::options::forced_run_ahead_note`, `text::run_ahead_forced_to_zero`, the two keys, the two drawing sites | the note, and that the row stays live |

### What a later lot must not undo

1. **`set_run_ahead` is the raw setter and `apply_run_ahead_setting` is the guarded one.** A new
   site that pushes the user's setting onto the session must call the second, or the guard-rail
   stops holding for that path.
2. **The detection stays at profile load.** Moving it into the frame loop would put a text scan,
   or a per-frame test of the scripts, in a per-frame path.
3. **The member list is `CORE_MEMBERS`'s change-the-core subset.** A new script member that writes
   the core from a frame hook belongs in `CORE_CHANGING_MEMBERS` as well as in `CORE_MEMBERS`.

### Spec copies

`spec/changes/Q-103.md` (answer included) is already byte-identical in
`impl/docs/spec/changes/Q-103.md`, copied by RU-3b after the answer came in — nothing to copy.
The answer amends no SPEC chapter: it decides a point AMD-041 left open, and SPEC-07 §3.7 already
carries REQ-LOOP-064 to 068 as RU-3b wrote them.
