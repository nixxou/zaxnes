# Handoff prompt for a local Claude Code session

Paste everything below the line into Claude Code, opened at the root of a clone of
`nixxou/zaxnes`.

---

You are continuing the Rust port of Zaxnes (GPL-3.0 NES emulator that renders games in 3D).
The C# reference is in `src/` and `tools/` (do not modify it). The Rust port is in `rust/` on
branch `rust-port`. The port itself is complete (1 700 tests, 291/291 core goldens, Lua API
identical, renderer validated on an RTX 3060). Current focus: performance on an Anbernic
RG40XXV handheld (PortMaster port).

## 0. First, establish the exact state (do not trust this text blindly)

1. `git fetch origin && git checkout rust-port && git pull origin rust-port`.
2. `git status` (must be clean) and `git log --oneline -30`. The last commits may be
   "Rust perf (core): ..." or "(wip): checkpoint" commits from the previous session. A wip
   commit may be unverified: if HEAD is a wip commit, check that it builds and that
   `cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored` passes; if not,
   find the last good commit and revert the wip changes in a new commit (never rewrite history).
3. Read, in this order: `rust/docs/HANDHELD-PERFORMANCE.md` (measurements, tools, next steps),
   `rust/docs/ROADMAP.md` (ordered plan and the setting rule), the end of `rust/PROGRESS.md`
   (latest lines under the table, "Rust-only changes"), `rust/LOCAL-CHECKS.md` (sections 7-8),
   `rust/RUST-PORT-PLAN.md` section 2 (Rust rules).
4. Report the state in a few lines (last good commit, what is verified, what is not) before
   starting any work.

## 1. Rules (always)

- Commits: `git -c user.name=nixxou -c user.email=nixx@nixx.com commit ...`, message
  `Rust perf: <what>` (or `Rust fix:` / `Rust handheld:`), push to `rust-port` only, never to
  `main`, no pull request. No AI attribution anywhere (no Co-Authored-By, no mention of
  Claude/Anthropic in commits or files). Commit and push after each verified step.
- Never commit ROMs, profiles (`.3dn`), 3dSen files, captures of commercial games, binaries or
  `target/`.
- Clean code: fix root causes; never weaken, skip, `#[ignore]` or rewrite tests' expectations to
  get green; no `#[allow(clippy::...)]` for fixable lints; no `Rc<RefCell>`/`Rc<Cell>` sharing
  between app and scripts (scripts borrow core/scene per hook call by design); `unsafe` only in
  zax-render / zax-platform documented modules with `// SAFETY:`; `zax-nes` stays
  `forbid(unsafe_code)`; `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.
- Output rule: an optimisation with identical output needs no switch but must pass the goldens;
  anything that changes output (fast PPU, external core, catch-up limit, 30 fps mode, GC tuning
  visible to scripts...) is a setting with a platform default and must never change the desktop
  default without the owner's approval.
- After any change to `zax-nes`: full core goldens (above) + `cargo test -p zax-nes --release`.
  After any change to scripting: `rust/check.sh` and `zax-harness lua-api` byte-identical to
  `rust/goldens/lua-api.txt`. After rendering changes: render synthetic goldens (zax-harness
  tests with `--all-features`) and pixel-identical captures vs the previous commit.
- Keep the workspace green: `rust/check.sh` before the final push of each step.
- Token economy: read only what you need (`grep -n`, ranged reads); pipe build/test output
  through `tail -n 30`; run the affected crates' tests, the whole workspace at milestones.

## 2. Device loop (the owner tests on the RG40XXV)

1. Build: `bash rust/build/package-linux-aarch64.sh` (needs Linux or WSL with zig,
   cargo-zigbuild, cmake, zip; see HANDHELD-PERFORMANCE.md section 6) ->
   `rust/target/package/Zaxnes.zip`.
2. The owner unzips it into the SD card `ports/` folder (keeping `ports/zaxnes/roms` and
   `ports/zaxnes/profiles`), plays 30 s of a 2D game and 30 s of a heavy 3D profile, and gives you
   `ports/zaxnes/logs/zaxnes.log` and `ports/zaxnes/log.txt`.
3. Read the profiler lines (core, conv, lua, scene, emulated per displayed frame) and compare
   with the table in HANDHELD-PERFORMANCE.md section 2; add the new numbers there.

## 3. Work, in order (one step at a time; verify; commit; device check)

Priority changed after the last device measurement (core ~15.6 ms per NES frame on the A53 after
the batched PPU, `91123ec`; x86 gains of 1.4-2x gave only ~11 % on the device): the core now fits
the 16.6 ms budget on its own, so multi-core comes first.

1. Multi-core emulation thread (HANDHELD-PERFORMANCE.md 5.2), as a setting ("emulation on a
   separate core"), default on for handheld, off on desktop until the owner approves. Emulation
   of frame N on its own thread; conversion, Lua and render of frame N-1 on the main thread; Lua
   reads a per-frame snapshot of NES memory (check SPEC-05 / SPEC-07 and the C# for when hooks see
   memory and how savestates, rewind, input latency and pause interact). Expected: close to 60
   emulated frames/s on the device for light profiles.
2. Handheld pacing settings (5.5): regular 30 fps mode and capped catch-up, so a late frame no
   longer collapses into 5 emulated frames per displayed frame.
3. Draw-call batching (5.3), pixel-identical, measured with `render --explain`, `drawCalls` and
   `--bench` (heavy 3D profile: scene 70-95 ms per displayed frame on the device).
4. Lua costs (5.4): profiling per hook/script/API, incremental GC per frame (spikes up to
   500 ms seen), dedicated userdata.
5. More structural core work, bit-identical (5.1 and section 4 "Recommended next core steps"),
   measured with callgrind `--cache-sim=yes` and on the device, not only x86 wall time.
6. Only if still too slow: fast PPU or external core (5.6), as settings, after asking the owner.

For each step: say what you will do and the expected gain, do it, report measured before/after
(x86, qemu, then device), update HANDHELD-PERFORMANCE.md (table + work done) and PROGRESS.md
(one line), commit, push, build the zip for the owner.

## 4. Open items outside performance (lower priority)

- Windows checks listed in `rust/LOCAL-CHECKS.md` not yet confirmed: `dumpbin /dependents
  zaxnes.exe` without VCRUNTIME140.dll, the AMD-037 real-install test (101 games, 45 derived ids),
  keyboard in handheld mode, Alt+Enter / Ctrl+, fixes.
- muOS `.muxapp` packaging, PortMaster screenshot, ask the PortMaster crew whether an NES
  emulator needing user ROMs is accepted.
- Postponed ideas in `rust/docs/ROADMAP.md` (Vulkan/DX12 via wgpu, ReShade depth, visual
  effects, display modes): do not start them without the owner's request.
