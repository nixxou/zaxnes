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

- 2. Launch from an Android frontend (issue #1). One channel, no unsafe: ZaxnesActivity writes
  `launch/request` (`rom=<path>`, `return=1`; `.part` renamed) in getExternalFilesDir, from
  onCreate (before SDL starts) and onNewIntent; a `content://` game is copied first to
  `launch/roms/<DISPLAY_NAME>`, a `file://` path kept, the extra "ROM" taken too. The handheld
  interface (`zax_app::launch_request`) looks twice a second, plays the game as from the list
  (matching profile, else 2D), in place of the one playing; Quit to ROM list then quits the app,
  which hands back to the frontend. Manifest: VIEW filter (content/file,
  application/octet-stream and x-nes-rom). Tests: launch_request unit tests,
  a_game_a_frontend_asks_for_plays_and_quitting_goes_back_to_it. Device check: pending.
  Limit: a `file://` path outside the app's folder needs a storage permission Zaxnes does not
  ask for; frontends that send `content://` (ES-DE's provider launch) work.

- 3. 3dSen `.bundle` import on the device. A `.bundle` opened with (VIEW) or shared to (SEND,
  EXTRA_STREAM) Zaxnes is copied by the activity, under its own name, into `import/` of the user
  data (a `.part` renamed); one can be copied there from a computer too. The console interface,
  outside a game, imports each waiting bundle with the desktop's importer
  (`import_from_three_d_sen`, 3dSen 1.x to 2.2), shows the counts or the error, removes the
  bundle and refreshes the list. No JNI. Instead of a menu entry listing the bundles (the plan),
  the import happens on arrival: one gesture fewer, same importer. Tests:
  the_waiting_bundles_are_the_bundle_files_of_the_import_folder,
  a_bundle_in_the_import_folder_is_imported_and_removed. Device check: pending.

## Also on this branch

- 3D framing (owner request, app-wide): `camera.frame_picture`, on by default, makes the view
  reset frame the picture's visible rows as the head-on 2D view does (2D to 3D no longer draws
  back); off is 3dSen's framing. A script's `Camera.ResetView` / `Camera.ResetDistance` keep
  3dSen's framing. b9b5cad9, 1d0319ee.
- Device: Retroid Pocket 5, APK installed; cold start and game replaced while running checked
  (file:// request). content:// launch, return on quit, capture fix, bundle import and framing
  left to the owner's hands-on check.
