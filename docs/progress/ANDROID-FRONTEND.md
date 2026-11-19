# Lot ANDROID-FRONTEND: three ideas of the Gin649/Xnes3D-plus fork, redone

Branch `feat/android-frontend` (worktree `G:\3DSen\wt-android-frontend`), from local `dev`
eb484c33. Owner request 2026-11-19. Plan: G:\3DSen\PROMPT-ANDROID-FORK-IDEAS.md.

Rules: safe Rust, no new unsafe at the SDL / Android border; no JNI (or the safe `jni` crate,
asked first). Tests: `cargo test -p zax-app --features bundled` with
`ZAX_FCEUX_DIR=G:\3DSen\cleanroom\fceux-zax`; the gitignored `crates/zax-app/demo` copied from
`wt-cleanup`.

## Done

- 1. Key capture (Controls screen of the handheld interface): the press that ends a capture is
  no longer a press of the screen too (B left the list, A started another capture). `update`
  notes that a capture was running before `update_capture` and drops the presses of the frame
  it ends in. Test `the_press_that_ends_a_capture_is_not_a_press_of_the_screen` (fails without
  the fix). Desktop console mode shares the code.

## Next

2. Launch from an Android frontend (issue #1): request file written by Java, read by Rust.
3. 3dSen `.bundle` import on the device.
