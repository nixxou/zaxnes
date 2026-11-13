//! The English table of AMD-035, the default language of a fresh user folder and the fallback of every
//! other language (AMD-035 §1, §3).

use crate::ui::keys;

pub(super) const ENGLISH: &[(&str, &str)] = &[
    // --------------------------------------------------------- menu bar
    (keys::MENU_FILE, "File"),
    (keys::MENU_EMULATION, "Emulation"),
    (keys::MENU_VIDEO, "Video"),
    (keys::MENU_AUDIO, "Audio"),
    (keys::MENU_TOOLS, "Tools"),
    (keys::MENU_RECENT_ROMS, "Recent ROMs"),
    (keys::MENU_LOAD_STATE, "Load a state"),
    (keys::MENU_SAVE_STATE, "Save the state"),
    (keys::MENU_DELETE_STATE, "Delete a state"),
    (keys::MENU_SPEED, "Speed"),
    (keys::MENU_REGION, "Region"),
    (keys::MENU_PORTS, "Ports"),
    (keys::MENU_CAMERA_MODE, "Camera mode"),
    (keys::MENU_VOLUME, "Volume"),
    (keys::MENU_NOTHING_HERE, "(empty)"),
    (keys::MENU_REGION_AUTO, "Auto"),
    (keys::MENU_PORT, "Port"),
    (keys::MENU_PORT_CONTROLLER, "Controller"),
    (keys::MENU_PORT_ZAPPER, "Zapper"),
    (keys::MENU_CAMERA_ORBITAL, "Orbit (TPV)"),
    (keys::MENU_CAMERA_FIRST_PERSON, "First person (FPV)"),
    (keys::SLOT_EMPTY, "empty"),
    (keys::SLOT_OTHER_PATCH, "made with the built-in patches set the other way"),
    (keys::SLOT_OTHER_HACK_VERSION, "made with another version of the built-in hack"),
    (keys::SLOT_ENGINE, "engine: {engine}"),
    (keys::SLOT_OTHER_ENGINE, "made with the {engine} engine: set this game's engine to {engine} to load it"),
    (keys::MENU_MOVIES, "Play a movie"),
    (keys::WINDOW_MOVIE_NOTE, "Note on this frame"),
    (keys::MOVIE_NOTE_WRITE, "Write"),
    (keys::WINDOW_ROMS_FOLDER, "ROM folder of the console mode"),
    (keys::BUTTON_BROWSE, "Browse..."),
    (keys::ROMS_FOLDER_HINT, "The games the console mode lists. Empty: {0}"),
    (keys::SLOT_QUICK, "Quick state"),
    (keys::SLOT_PREFIX, "Slot"),
    // ----------------------------------------------------- window titles
    (keys::WINDOW_OPTIONS, "Options"),
    (keys::WINDOW_KEY_CONFIG, "Key configuration"),
    (keys::WINDOW_ABOUT, "About"),
    (keys::WINDOW_PROFILE_CHOOSER, "Choose the 3D profile"),
    (keys::WINDOW_OPEN_ROM, "Open a ROM"),
    (keys::WINDOW_OPEN_MOVIE, "Open a movie"),
    (keys::WINDOW_IMPORT_PROFILE, "Import a 3D profile"),
    (keys::WINDOW_IMPORT_COVER, "Import a cover"),
    (keys::WINDOW_IMPORT_THREE_DSEN, "Import from 3dSen"),
    // ----------------------------------------------------------- buttons
    (keys::BUTTON_CLOSE, "Close"),
    (keys::BUTTON_CANCEL, "Cancel"),
    (keys::BUTTON_OPEN, "Open"),
    (keys::BUTTON_LAUNCH, "Launch"),
    (keys::BUTTON_PARENT_FOLDER, "Parent folder"),
    (keys::BUTTON_RESET_DEFAULTS, "Defaults"),
    (keys::BUTTON_RESET_BINDINGS, "Reset every key"),
    (keys::BUTTON_STOP_BINDING, "Stop the capture"),
    (keys::BUTTON_YES, "Yes"),
    (keys::BUTTON_NO, "No"),
    // AMD-056 REQ-UI-214: the buttons of the three import dialogues.
    (keys::BUTTON_OK, "OK"),
    (keys::BUTTON_REPLACE, "Replace"),
    // ------------------- the import dialogues of AMD-056 REQ-UI-214, word for word what the
    //                     amendment's table writes. There is nothing for a successful import:
    //                     it shows neither notification nor dialogue.
    (keys::DIALOG_IMPORT_REFUSED_TITLE, "Cannot import this file"),
    (keys::DIALOG_IMPORT_REFUSED_BODY, "{0}: {1}"),
    (keys::DIALOG_IMPORT_BUILT_IN_TITLE, "Already in Zaxnes"),
    (keys::DIALOG_IMPORT_BUILT_IN_BODY, "{0} is a built-in profile, so {1} was not imported."),
    (keys::DIALOG_IMPORT_REPLACE_TITLE, "Replace {0}?"),
    (keys::DIALOG_IMPORT_REPLACE_BODY, "The profile will be replaced with the one in {0}. The ROM, cover and save states will be kept."),
    // ------------------------ notifications of SPEC-10 REQ-UI-082 to REQ-UI-084
    (keys::NOTIFY_ROLL_BACK, "Roll Back"),
    (keys::NOTIFY_FAST_FORWARD, "Fast Forward"),
    (keys::NOTIFY_QUICK_SAVE, "Quick Save"),
    (keys::NOTIFY_QUICK_LOAD, "Quick Load"),
    (keys::NOTIFY_CORRUPTED_ZIP, "Corrupted zip file"),
    (keys::NOTIFY_STATE_OTHER_PATCH, "This save state was made with the built-in ROM patches set the other way."),
    (keys::NOTIFY_STATE_OTHER_HACK_VERSION, "This save state was made with another version of the built-in hack: it cannot be loaded."),
    (keys::CONFIRM_TURN_HACK_ON, "To play this save state, the built-in hack will be turned on: the game reloads, then the save state loads. Continue?"),
    (keys::CONFIRM_TURN_HACK_OFF, "To play this save state, the built-in hack will be turned off: the game reloads, then the save state loads. Continue?"),
    (keys::CONFIRM_OTHER_COMPONENTS, "This save state was made with other parts of the built-in hack (the widescreen, the aids) than those set now. Load it with the parts set now, or set them as the save state had them (the game reloads)?"),
    (keys::BUTTON_LOAD_AS_SET, "Parts set now"),
    (keys::BUTTON_LOAD_ALIGNED, "Parts of the save"),
    (keys::NOTIFY_UNCOMPRESSION_FAILED, "Uncompression is not successful"),
    (keys::NOTIFY_INVALID_ROM, "Invalid rom file."),
    (keys::NOTIFY_GAME_MISSING, "The game profile doesn't exist anymore."),
    (keys::NOTIFY_LOADING_TIMED_OUT, "Loading timed out."),
    (keys::NOTIFY_FOREIGN_CORE, "Save state used a different core."),
    (keys::NOTIFY_NOT_A3DN_FILE, "This file is not a 3dn file"),
    (keys::NOTIFY_UNSUPPORTED3DN_VERSION, "3dn format version {0}  is not supported"),
    (keys::NOTIFY_OBSOLETE_PROFILE_FORMAT, "{0} has an obsolete or wrong profile format. This profile is not compatible with the current version of Zaxnes."),
    // ------------- notifications of the persisted files (REQ-PERS-022, REQ-PERS-024)
    (keys::NOTIFY_SETTINGS_UNREADABLE, "Settings file was unreadable and has been reset."),
    (keys::NOTIFY_SETTINGS_NEWER_VERSION, "Settings were created by a newer version."),
    (keys::NOTIFY_GAME_LIST_UNREADABLE, "Game list was unreadable and has been reset."),
    (keys::NOTIFY_GAME_LIST_NEWER_VERSION, "Game list was created by a newer version."),
    (keys::NOTIFY_BINDINGS_UNREADABLE, "Input bindings were unreadable and have been reset."),
    (keys::NOTIFY_BINDINGS_NEWER_VERSION, "Input bindings were created by a newer version."),
    (keys::NOTIFY_THREE_DSEN_SUMMARY, "3dSen: {0} profile(s) imported, {1} already present, {2} failed."),
    // ------------------- confirmations of REQ-PERS-042 and REQ-PERS-054, with the
    (keys::CONFIRM_DELETE_GAME, "Do you want to delete the game and its save states ?"),
    (keys::CONFIRM_DELETE_SLOT, "Do you want to delete the slot {0} ?"),
    // ------------------------------------------------------------- hints
    (keys::HINT_IMPORT_THREE_DSEN, "Choose the installation folder of 3dSen (the one holding 3dSen.exe), or the .bundle profile file itself."),
    (keys::HINT_RES_SCALE_DIRECT, "(no effect in direct rendering: native resolution)"),
    (keys::HINT_RUN_AHEAD_FORCED, "(forced to 0: this game's scripts write the core)"),
    (keys::HINT_PRESS_ACONTROL, "Press a key or a button…"),
    (keys::HINT_CLICK_ACELL, "Click a cell to reassign it."),
    (keys::HINT_AT_NEXT_START, "(at the next start)"),
    (keys::HINT_AT_NEXT_LOAD, "(at the next load)"),
    (keys::HINT_PROFILE_SETTING, "(profile setting)"),
    // ---------------------------- RO-1: the groups of the options window
    (keys::GROUP_RESOLUTION, "Resolution"),
    (keys::GROUP_PERFORMANCE, "Performance"),
    (keys::GROUP_OVERSCAN, "Overscan"),
    (keys::GROUP_WIDESCREEN, "Widescreen"),
    (keys::GROUP_REGION, "Video region"),
    (keys::GROUP_ADVANCED, "Advanced"),
    // ---------------------- RO-3: the pages and the group titles of the console option tree
    (keys::PAGE_ENHANCED_EFFECTS, "Enhanced effects"),
    (keys::PAGE_GEOMETRY, "Geometry (curvature)"),
    (keys::GROUP_REFLECTIONS, "Reflections"),
    (keys::GROUP_AMBIENT_OCCLUSION, "Ambient occlusion"),
    // -------------- RO-1 point 6: the two compact rows of the overscan margins, and the caption
    //                of each of the four values a compact row holds side by side.
    (keys::COMPACT_OVERSCAN_NTSC, "Overscan NTSC"),
    (keys::COMPACT_OVERSCAN_PAL, "Overscan PAL"),
    (keys::CAPTION_TOP, "top"),
    (keys::CAPTION_BOTTOM, "bottom"),
    (keys::CAPTION_LEFT, "left"),
    (keys::CAPTION_RIGHT, "right"),
    // ------------- RO-1 point 1: why a row is greyed out. Each one names the setting that has
    //               to change for the row to act again; the value itself is kept either way.
    (
        keys::HINT_RES_SCALE_AUTOMATIC,
        "(no effect: the automatic resolution chooses it)",
    ),
    (keys::HINT_NEEDS_SHADOWS, "(no effect: the shadows are off)"),
    (
        keys::HINT_NEEDS_LUA_OPTIMIZED,
        "(no effect: Optimized Lua is off)",
    ),
    (
        keys::HINT_BG_COLOR_UNUSED,
        "(no effect: this background is drawn from the game's own colours)",
    ),
    (keys::HINT_NEEDS_ARC, "(no effect: Arc Horizontal is 0)"),
    (
        keys::HINT_NEEDS_CURVATURE,
        "(no effect: no curvature is set)",
    ),
    (
        keys::HINT_REGION_AUTOMATIC,
        "(set by the detection at the next ROM load)",
    ),
    (
        keys::HINT_SINGLE_CORE,
        "(this version has one core: the value is always 0)",
    ),
    (
        keys::HINT_FCEUX_NO_WIDESCREEN,
        "(this engine has no widescreen: choose the Zaxnes engine for this game)",
    ),
    (
        keys::HINT_FCEUX_NO_HACKS,
        "(this engine applies no hack pack: choose the Zaxnes engine for this game)",
    ),
    (
        keys::HINT_FCEUX_NO_SPRITE_LIMIT,
        "(this engine keeps the 8 sprites a line: choose the Zaxnes engine for this game)",
    ),
    (
        keys::HINT_FCEUX_NO_RUN_AHEAD,
        "(run-ahead is not implemented on this engine)",
    ),
    (keys::STATUS_ENGINE_FCEUX, "FCEUX engine"),
    (keys::STATUS_ENGINE_NESHAWK, "NesHawk engine"),
    (
        keys::HINT_FCEUX_VALIDATED,
        "(this ROM is validated for 3D on the FCEUX engine)",
    ),
    (
        keys::HINT_FCEUX_NOT_VALIDATED,
        "(this ROM is not validated for 3D on the FCEUX engine: the 3D may differ from Zaxnes's)",
    ),
    (
        keys::HINT_CLASSIC_AUDIO,
        "(no effect: the classic audio output has its own fixed buffer)",
    ),
    (
        keys::HINT_NEEDS_ACHIEVEMENTS,
        "(no effect: RetroAchievements is off)",
    ),
    (
        keys::HINT_STYLE_CLASSIC,
        "Style = Classic: these effects are off.",
    ),
    (
        keys::HINT_MENU_BUTTON_UNUSED,
        "(kept for 3dSen: Zaxnes does not act on it)",
    ),
    // ---------------------------------------------------- fields, columns
    (keys::FIELD_SEARCH, "Search"),
    (keys::COLUMN_ACTION, "Action"),
    (keys::COLUMN_KEYBOARD, "Keyboard"),
    (keys::COLUMN_GAMEPAD, "Gamepad"),
    (keys::VALUE_NO_GAMEPAD, "(no gamepad)"),
    // ---------------------------------------------------- welcome screen
    (keys::WELCOME_HINT, "File ▸ Open a ROM"),
    (keys::WELCOME_RECENT, "Recent ROMs"),
    (keys::WELCOME_NO_RECENT, "No ROM has been opened yet."),
    // -------------------------------------------------------- about block
    (keys::ABOUT_LICENCE, "GPL v3"),
    (keys::ABOUT_CREDIT_INSPIRED, "Independent project inspired by 3dSen (Geod Studio);"),
    (keys::ABOUT_CREDIT_ADAPTED, "contains code adapted from 3DNes_Engine (Geod Studio, GPL v3)."),
    (keys::ABOUT_CREDIT_DISCLAIMER, "Not affiliated with Geod Studio or Nintendo."),
    (keys::ABOUT_VERSION_PREFIX, "Version"),
    // -------------------------------------------------- options window tabs
    (keys::TAB_DISPLAY, "Display"),
    (keys::TAB_NES_PICTURE, "NES picture"),
    (keys::TAB_RENDER_3D, "3D rendering"),
    (keys::TAB_CAMERA, "Camera"),
    (keys::TAB_LIGHT, "Light and shadows"),
    (keys::TAB_AUDIO, "Audio"),
    (keys::TAB_EMULATION, "Emulation"),
    (keys::TAB_INTERFACE, "Interface"),
    // ---------------------------------------- key configuration sections
    (keys::KEYS_PLAYER_ONE, "Player 1 — NES buttons"),
    (keys::KEYS_PLAYER_TWO, "Player 2 — NES buttons"),
    (keys::KEYS_CAMERA, "Camera"),
    (keys::KEYS_HOTKEYS, "System hotkeys"),
    (keys::KEYS_COMMANDS, "Menu commands"),
    // ------------------------------------------------------ file browser
    (keys::BROWSER_COMPUTER, "This computer"),
    (keys::BROWSER_FILTER_NES_ROMS, "NES ROMs (*.nes, *.zip)"),
    (keys::BROWSER_FILTER_NES_FILES, ".nes files"),
    (keys::BROWSER_FILTER_MOVIES, "Movies (.fm2, .fcm, .bk2, .mv)"),
    (keys::MOVIE_BAR_PLAY, "Play"),
    (keys::MOVIE_BAR_PAUSE, "Pause"),
    (keys::MOVIE_BAR_SEEKING, "going to"),
    (keys::PENS_HAND, "Hand: the pointer turns the camera"),
    (keys::PENS_UNDO, "Undo"),
    (keys::PENS_CLEAR, "Clear"),
    (keys::PENS_RED, "Red pen"),
    (keys::PENS_BLUE, "Blue pen"),
    (keys::PENS_BLACK, "Black pen"),
    (keys::PENS_WHITE, "White pen"),
    (keys::PENS_VIEW_2D, "2D: the NES picture"),
    (keys::PENS_VIEW_ZAXNES, "3D as Zaxnes draws it"),
    (keys::PENS_VIEW_3DSEN, "3D as 3dSen draws it (the Zaxnes blocks of the scripts left out)"),
    (keys::PENS_VIEW_HANDHELD, "3D as Zaxnes draws it on a handheld"),
    (keys::BROWSER_FILTER_ZIP, ".zip archives"),
    (keys::BROWSER_FILTER_PROFILES, "3D profiles (*.3dn, *.3dh)"),
    (keys::BROWSER_FILTER_BUNDLE, "3dSen profile bundle (*.bundle)"),
    (keys::BROWSER_FILTER_PNG, "PNG images (*.png)"),
    (keys::BROWSER_FILTER_ALL, "All files"),
    // ---------------------------------------------------- profile chooser
    (keys::PROFILE_TWO_DIMENSIONAL, "2D Mode"),
    (keys::PROFILE_SOURCE_TWO_DIMENSIONAL, "no 3D conversion"),
    (keys::PROFILE_SOURCE_DEFAULT_THREE_D, "default 3D geometry"),
    (keys::PROFILE_SOURCE_BUILT_IN, "built-in profile"),
    (keys::PROFILE_SOURCE_PROGRAM_FOLDER, "profile from the program folder"),
    (keys::PROFILE_SOURCE_USER, "user profile"),
    // ---------------------------------------------------------- shortcuts
    (keys::SHORTCUT_ALT_ENTER, "Alt+Enter"),
    (keys::SHORTCUT_SHIFT_PREFIX, "Shift"),
    (keys::SHORTCUT_QUICK_SAVE, "QuickSave shortcut"),
    (keys::SHORTCUT_QUICK_LOAD, "QuickLoad shortcut"),
    // The default keys of the FastForward and RollBack actions (owner review before 0.5.0: the
    // action names were shown instead), as the --help text gives them.
    (keys::SHORTCUT_FAST_FORWARD, "."),
    (keys::SHORTCUT_ROLLBACK, ","),
    (keys::SHORTCUT_SAVE_SLOTS, "Shift+F1…F10"),
    (keys::SHORTCUT_LOAD_SLOTS, "Ctrl+F1…F10"),
    (keys::SHORTCUT_LONG_PRESS_MENU, "hold Esc"),
    // ------------------------------------------- commands of REQ-UI-205
    ("cmd.file.open_rom", "Open a ROM…"),
    ("cmd.file.open_recent_rom", "Recent ROMs"),
    ("cmd.file.choose_profile", "Choose the 3D profile…"),
    ("cmd.file.import_profile", "Import a profile…"),
    ("cmd.file.import_3dsen", "Import from 3dSen…"),
    ("cmd.file.launch_game", "Launch the game"),
    ("cmd.file.delete_game", "Delete the game…"),
    ("cmd.file.create_game_art", "Create the cover"),
    ("cmd.file.import_cover", "Import a cover…"),
    // AMD-053 §4: the four commands of REQ-UI-210, REQ-UI-211 and REQ-LOOP-076.
    ("cmd.resume.last", "Resume"),
    ("cmd.favorite.toggle", "Favorite"),
    ("cmd.library.filter_favorites", "Favorites only"),
    ("cmd.library.search", "Search"),
    ("cmd.app.quit", "Quit"),
    ("cmd.state.save_slot", "Save the state"),
    ("cmd.state.load_slot", "Load a state"),
    ("cmd.state.quick_save", "Quick state: save"),
    ("cmd.state.quick_load", "Quick state: load"),
    ("cmd.state.delete_slot", "Delete the state"),
    ("cmd.emulation.pause_resume", "Pause / Resume"),
    ("cmd.emulation.pause", "Pause"),
    ("cmd.emulation.resume", "Resume"),
    // AMD-044 §3: the two resets now sit side by side, so the hard one is named as such.
    ("cmd.emulation.reset", "Hard Reset"),
    ("cmd.emulation.soft_reset", "Soft Reset"),
    ("cmd.emulation.stop", "Stop"),
    ("cmd.emulation.fast_forward", "Fast forward"),
    ("cmd.emulation.rollback", "Rollback"),
    ("cmd.emulation.frame_back", "Previous frame"),
    ("cmd.emulation.frame_advance", "Next frame"),
    ("cmd.emulation.set_speed", "Speed"),
    ("cmd.emulation.speed_up", "Faster"),
    ("cmd.emulation.speed_down", "Slower"),
    ("cmd.emulation.set_region", "Region"),
    ("cmd.emulation.auto_region", "Region: Auto"),
    ("cmd.emulation.set_port_type", "Port type"),
    ("cmd.video.toggle_2d", "3D mode / 2D mode"),
    ("cmd.video.reset_view", "Reset the view"),
    ("cmd.video.set_camera_mode", "Camera mode"),
    ("cmd.video.toggle_local_camera", "Top-down view"),
    ("cmd.video.set_skybox", "Skybox"),
    ("cmd.video.toggle_shadows", "Shadows"),
    ("cmd.video.toggle_reflections", "Reflections"),
    ("cmd.video.toggle_hack_option", "Hack option"),
    ("cmd.video.toggle_fullscreen", "Full screen"),
    ("cmd.video.toggle_fps", "Show the FPS"),
    ("cmd.audio.set_volume", "Volume"),
    ("cmd.audio.volume_up", "Volume +"),
    ("cmd.audio.volume_down", "Volume −"),
    ("cmd.audio.toggle_mute", "Mute"),
    ("cmd.audio.toggle_menu_music", "Menu music"),
    ("cmd.tools.options", "Options…"),
    ("cmd.tools.key_config", "Key configuration…"),
    ("cmd.tools.data_folder", "Data folder"),
    ("cmd.tools.about", "About"),
    ("cmd.tools.record_movie", "Record a movie"),
    ("cmd.tools.play_movie", "Play a movie"),
    ("cmd.tools.stop_movie", "Stop the movie"),
    ("cmd.tools.seek_movie", "Go to a frame of the movie"),
    ("cmd.tools.reload_script", "Reload the profile's scripts"),
    ("cmd.tools.open_movie", "Open a movie file..."),
    ("cmd.tools.annotate_movie", "Note on this frame…"),
    ("cmd.tools.editor_mode", "Editor mode"),
    ("cmd.tools.new_lua_profile", "New Lua profile for this game"),
    ("cmd.tools.copy_agent_prompt", "Copy the prompt for the AI agent"),
    ("cmd.ui.cancel_modal", "Close the modal window"),
    ("cmd.ui.toggle_debug_overlay", "Statistics panel"),
    ("cmd.ui.close_windows", "Close the windows"),
    ("cmd.ui.toggle_console", "Console mode"),
    ("cmd.ui.save_states", "Save a state…"),
    ("cmd.ui.load_states", "Load a state…"),
    // RO-2 part 1: the entry names the feature, the menu it is in no longer does.
    ("cmd.ui.achievements_account", "RetroAchievements account and options…"),
    // ------------------------------------------- labels of SPEC-09 settings
    ("setting.video.frame_rate", "Render rate"),
    // AMD-052 §3 point 10: right after "Render Rate", in the Video group.
    ("setting.video.max_queued_frames", "Max Queued Frames"),
    ("setting.video.skip_frame", "Accelerated rendering (frame skip)"),
    ("setting.video.vsync", "Vertical synchronisation"),
    ("setting.video.render_method", "Render method"),
    ("setting.video.res_scale", "Resolution scale"),
    ("setting.video.anti_alias", "Anti-aliasing"),
    ("setting.render.mesh_algo", "Mesh algorithm"),
    ("setting.render.shadow", "Shadows"),
    // AMD-049 REQ-SCN-075, a setting of Zaxnes's own: the backdrop a profile's script puts behind
    // the scene is lit with the almost-matt model of the original unless this is turned off.
    ("setting.render.backdrop_lighting", "Backdrop lighting"),
    (
        "help.render.backdrop_lighting",
        "Lights the backdrop behind the scene with a faint sheen, as 3dSen does. Turn it off for \
         plain diffuse lighting, which costs a little less on a small machine. Only some profiles \
         show a backdrop at all.",
    ),
    ("setting.camera.fov", "Field of view (degrees)"),
    ("setting.light.intensity", "Light intensity"),
    ("setting.scene.skybox", "Background (skybox)"),
    ("setting.scene.custom_bg_color", "Background colour"),
    // The "Style" group of AMD-055 §3 point 6: the thirteen labels and the thirteen help
    // sentences of its §4 table, in the amendment's own words. The group comes right after the
    // Skybox row, which is where SPEC-10 REQ-UI-061 as amended puts it.
    ("group.style", "Style"),
    ("setting.render.visual_style", "Style"),
    (
        "help.render.visual_style",
        "Classic keeps the original look. Enhanced adds optional effects.",
    ),
    ("setting.render.style.ao", "Ambient Occlusion"),
    (
        "help.render.style.ao",
        "Soft shade in corners and hollows.",
    ),
    ("option.MSVO", "MSVO"),
    ("option.Depth", "Depth"),
    ("setting.render.style.ao_strength", "Strength"),
    (
        "help.render.style.ao_strength",
        "How dark the shading gets.",
    ),
    ("setting.render.style.ao_radius", "Radius"),
    ("help.render.style.ao_radius", "How far the shading reaches."),
    ("setting.render.style.ao_thickness", "Thickness"),
    (
        "help.render.style.ao_thickness",
        "No effect with Depth occlusion.",
    ),
    (
        "hint.style.thickness_no_effect",
        "no effect with Depth occlusion",
    ),
    ("setting.render.style.ao_quality", "AO Quality"),
    (
        "help.render.style.ao_quality",
        "More samples, smoother shading.",
    ),
    ("option.Low", "Low"),
    ("option.Medium", "Medium"),
    ("option.High", "High"),
    ("option.Ultra", "Ultra"),
    ("setting.render.style.pixel_shadow", "Pixel Shadows"),
    (
        "help.render.style.pixel_shadow",
        "Shadows that follow the game's pixel grid.",
    ),
    ("setting.render.style.voxel_grid", "Voxel Grid"),
    (
        "help.render.style.voxel_grid",
        "Grooves every 2, 4 or 8 voxels, like toy bricks.",
    ),
    ("option.Every 2 voxels", "Every 2 voxels"),
    ("option.Every 4 voxels", "Every 4 voxels"),
    ("option.Every 8 voxels", "Every 8 voxels"),
    ("setting.render.style.reflections", "Reflections"),
    (
        "help.render.style.reflections",
        "The floor reflects the scene like a polished surface.",
    ),
    ("setting.render.style.ssr_shine", "Floor Shine"),
    (
        "help.render.style.ssr_shine",
        "How strong the reflection is.",
    ),
    ("setting.render.style.ssr_clarity", "Clarity"),
    (
        "help.render.style.ssr_clarity",
        "Higher values keep the reflection sharp.",
    ),
    ("setting.render.style.ssr_length", "Length"),
    (
        "help.render.style.ssr_length",
        "How far the reflection reaches.",
    ),
    ("setting.render.style.ssr_quality", "Reflection Quality"),
    (
        "help.render.style.ssr_quality",
        "More steps, fewer gaps in the reflection.",
    ),
    // The "Geometry" group of AMD-043 §6 and AMD-046 §3: the labels and the help sentences, as
    // the two amendments write them.
    ("group.geometry", "Geometry"),
    ("setting.scene.bend_lr", "Bend Left-Right"),
    (
        "help.scene.bend_lr",
        "Bends the scene horizontally. Positive values bulge it outwards like a CRT screen; \
         negative values curve it around the player.",
    ),
    ("setting.scene.bend_tb", "Bend Top-Bottom"),
    (
        "help.scene.bend_tb",
        "Bends the scene vertically. Positive values bulge it outwards; negative values curve \
         it around the player.",
    ),
    ("setting.scene.arc_h", "Arc Horizontal"),
    (
        "help.scene.arc_h",
        "Rolls the level onto an arc, as if it stood on a tiny planet. Works best with \
         side-scrollers.",
    ),
    ("setting.scene.arc_center", "Arc Center"),
    (
        "help.scene.arc_center",
        // AMD-043 §6's own sentence, with the quarters RO-5 part 2 adds appended to it rather
        // than replacing it.
        "Where the arc bends around: the centre of the screen, or its bottom edge, or one of the \
         quarters between the two.",
    ),
    ("option.Screen Center", "Screen Center"),
    ("option.Screen Edge", "Screen Edge"),
    // RO-5 part 2: the three quarters between those two heights. The label the domain and the
    // libretro option carry is "25%"; what the user reads has a space, as a percentage should.
    ("option.25%", "25 %"),
    ("option.50%", "50 %"),
    ("option.75%", "75 %"),
    // AMD-046 §3, a setting of Zaxnes's own: the flat picture of the 2D mode follows the
    // curvature of AMD-043 unless this is turned off.
    ("setting.scene.bend_2d_mode", "Bend 2D Mode"),
    (
        "help.scene.bend_2d_mode",
        "Also applies the bend settings to the flat picture of 2D mode. Turn it off to keep the \
         picture flat.",
    ),
    ("setting.emu.game_speed", "Game speed"),
    ("setting.emu.auto_tv_mode", "Region detected automatically"),
    ("setting.emu.tv_mode", "Video region"),
    ("setting.emu.core", "Emulation core"),
    ("setting.emu.engine", "Emulation engine (this game)"),
    ("setting.emu.cartridge_db", "Cartridge database"),
    ("setting.emu.oam_corruption", "OAM corruption"),
    ("setting.emu.ppu_bus_conflicts", "PPU bus conflicts"),
    ("setting.emu.oamaddr_bug", "OAMADDR defect (2C02G)"),
    ("setting.emu.oam_decay", "OAM decay"),
    ("setting.emu.power_on_top", "Power on at the top of the picture"),
    ("setting.emu.power_on_ram", "Work RAM at power-on"),
    ("option.Zeros", "Zeros"),
    ("option.FCEUX pattern", "FCEUX pattern"),
    ("option.All $FF", "All $FF"),
    ("option.Random", "Random"),
    ("setting.emu.catch_up", "Catch-up when late (frames)"),
    // AMD-041 §3: the title and the four values of the Run-ahead list.
    ("setting.emu.run_ahead", "Run-ahead"),
    ("option.Run-ahead off", "Off"),
    ("option.1 frame (Recommended)", "1 frame (Recommended)"),
    ("option.2 frames", "2 frames"),
    ("option.3 frames", "3 frames"),
    ("option.10 frames", "10 frames"),
    ("option.9 frames", "9 frames"),
    ("option.8 frames", "8 frames"),
    ("option.7 frames", "7 frames"),
    ("option.6 frames", "6 frames"),
    ("option.5 frames", "5 frames"),
    ("option.4 frames", "4 frames"),
    ("setting.emu.lua_smooth_gc", "Smooth Lua memory collection"),
    ("setting.emu.lua_optimized", "Optimized Lua (unsafe)"),
    (
        "setting.emu.lua_experimental",
        "Experimental Lua optimization (needs Optimized Lua)",
    ),
    ("setting.emu.pipeline", "Scripts in parallel with the drawing"),
    ("setting.emu.pause_3d_in_2d", "Pause the 3D in 2D"),
    ("setting.emu.catch_up_last_3d", "Catch-up: 3D of the shown frame only"),
    ("setting.video.half_rate_3d", "3D at 30 fps (game at full speed)"),
    ("setting.video.dynamic_resolution", "Automatic resolution"),
    ("setting.video.shader_resolution", "Shader resolution"),
    (
        "detail.video.shader_resolution",
        "The share of the screen the slang shaders draw at, stretched over it afterwards: 50 % \
         costs the GPU about a quarter as much (Mega Bezel in time on a phone), for a softer CRT \
         mask and scanlines. Default: 100 %.",
    ),
    ("setting.video.backend", "Graphics API (at the next start)"),
    ("setting.video.widescreen", "Widescreen (ROMs made for it)"),
    ("setting.video.widescreen_extra", "Widescreen extra width"),
    (
        "setting.video.widescreen_strict_edges",
        "Widescreen: wait for whole objects at the drawn edges",
    ),
    ("setting.video.remove_sprite_limit", "Remove the limit of 8 sprites per line"),
    (
        "setting.emu.smooth_scroll_ahead",
        "Smooth scrolling (interpolated from run-ahead)",
    ),
    (
        "setting.emu.smooth_scroll_view",
        "Show the smooth scrolling's band (red, still sprites green)",
    ),
    ("setting.video.overscan_ntsc_top", "Overscan NTSC: top"),
    ("setting.video.overscan_ntsc_bottom", "Overscan NTSC: bottom"),
    ("setting.video.overscan_ntsc_left", "Overscan NTSC: left"),
    ("setting.video.overscan_ntsc_right", "Overscan NTSC: right"),
    ("setting.video.overscan_pal_top", "Overscan PAL: top"),
    ("setting.video.overscan_pal_bottom", "Overscan PAL: bottom"),
    ("setting.video.overscan_pal_left", "Overscan PAL: left"),
    ("setting.video.overscan_pal_right", "Overscan PAL: right"),
    ("setting.video.overscan_top", "Overscan: top"),
    ("setting.video.overscan_bottom", "Overscan: bottom"),
    ("setting.video.overscan_left", "Overscan: left"),
    ("setting.video.overscan_right", "Overscan: right"),
    ("setting.video.overscan_mode", "Overscan: fit (edges hidden) or resize (enlarged)"),
    ("setting.video.nes_palette", "NES colour palette"),
    ("setting.emu.builtin_patches", "Built-in ROM patches (at the next ROM load)"),
    ("setting.audio.volume", "Master volume"),
    ("setting.audio.bg_music", "Menu music"),
    ("setting.audio.mute", "Mute"),
    ("setting.audio.latency_ms", "Output latency (ms)"),
    (
        "setting.audio.classic_output",
        "Classic audio output (v0.4.2)",
    ),
    (
        "help.audio.classic_output",
        "Use if the new audio output misbehaves on this device; it has the old, shorter buffer \
         and can crackle.",
    ),
    ("setting.input.emulated_menu_button", "Menu = Select + Start"),
    ("setting.input.touch_pad", "Touch pad"),
    ("setting.input.touch_scale", "Touch pad size"),
    ("setting.input.touch_haptic_dpad", "Vibration of the cross"),
    ("setting.input.touch_haptic_buttons", "Vibration of the buttons"),
    ("setting.input.touch_extra", "Rewind and fast forward"),
    ("setting.ui.orientation", "Orientation"),
    ("setting.ui.fullscreen", "Full screen"),
    ("setting.ui.show_fps", "Show the FPS"),
    ("setting.ui.profiler", "Performance measures (log)"),
    ("setting.ui.editor_mode", "Editor mode (MCP server, pens, editor folder)"),
    ("setting.ui.roms_folder", "ROM folder"),
    ("setting.ui.sort_option", "Library sort order"),
    ("setting.ui.language", "Language"),
    // ----------------------------------- labels of the enumerated values
    ("option.Off", "Off"),
    ("option.Auto", "Auto"),
    ("option.Light", "Light"),
    ("option.Strong", "Strong"),
    ("option.Landscape", "Landscape"),
    ("option.Portrait", "Portrait"),
    ("option.Always", "Always"),
    ("option.Disabled", "Disabled"),
    ("option.Smart", "Smart"),
    ("option.Every refresh", "Every refresh"),
    ("option.Every other refresh", "Every other refresh"),
    ("option.Global", "Global"),
    ("option.Fit", "Fit"),
    ("option.Resize", "Resize"),
    ("option.Default", "Default"),
    ("option.Color", "Colour"),
    ("option.Gradient", "Gradient"),
    ("option.Fantasy", "Fantasy"),
    ("option.None", "None"),
    // RO-5 part 1 (owner request): the two levels of `video.anti_alias` are named FFAA and SMAA by
    // REQ-PERS-017, but AMD-032 §2 renders them as multisampling — 4 samples and 8. The interface
    // says what the user gets; the **domain** labels stay FFAA and SMAA, being what the settings
    // file and the libretro option values hold. Q-107 §7.
    ("option.FFAA", "MSAA 4×"),
    ("option.SMAA", "MSAA 8×"),
    // AMD-060 §2: the fourth level, "sous un libellé propre […] sans toucher aux trois libellés
    // existants". It is the only one of the four that really is SMAA, so the label says which SMAA
    // it is rather than how many samples it takes — there is one.
    ("option.SMAA_DEPTH", "SMAA (depth)"),
    ("option.Native", "Native"),
    ("option.enabled", "enabled"),
    ("option.disabled", "disabled"),
    ("option.A-Z", "A-Z"),
    ("option.Recent", "Recent"),
    ("option.en", "English"),
    ("option.fr", "Français"),
    // ================================================================= RO-1 point 3
    // The detailed tooltip of every row of the desktop options window: what the setting does, what
    // its values mean where they are not obvious, its default (and the handheld one where it
    // differs), what it costs, and what it depends on. `ui::options::tooltip_of` puts the short
    // `help.*` sentence of the amendment first, where there is one, and this paragraph after it.
    //
    // Written from `zax-settings/src/table.rs`, `ui/handheld/defaults.rs`,
    // `docs/PERFORMANCE-HISTORY.md` and the code path that reads each setting — never guessed.
    // Desktop only: the console pages and Android have no hover.

    // ------------------------------------------------------------- Display tab
    (
        "detail.ui.fullscreen",
        "Draws the window over the whole screen, without borders. Alt+Enter does the same thing at \
         any time, and so does Video ▸ Full screen. Default: off.",
    ),
    (
        "detail.video.vsync",
        "Ties the picture to the screen's refresh. Off hands each frame over as soon as it is \
         drawn: the lowest delay, and the picture can tear. Every refresh follows the screen, which \
         removes the tearing and caps the render rate at the screen's own (60 frames per second on \
         a 60 Hz panel). Every other refresh halves that again, for a screen whose refresh is far \
         above what the machine can draw. Default: Off on the desktop, Every refresh on a handheld.",
    ),
    (
        "detail.video.frame_rate",
        "How many times a second the 3D scene is redrawn. It is not the speed of the game: the \
         emulation keeps the console's own rate (about 60 frames per second for an NTSC game, 50 \
         for a PAL one) whatever is chosen here. A higher rate makes the camera and the scrolling \
         smoother and costs GPU time in proportion. Vertical synchronisation set to a refresh caps \
         it at the screen's rate. Default: 90.",
    ),
    (
        "detail.video.render_method",
        "Direct draws the 3D scene straight into the window, always at the window's own resolution. \
         Render Tex draws it into a texture first and then puts the texture on the screen, which is \
         what the resolution scale and the automatic resolution need; it costs one full-screen copy \
         per frame. Default: Direct, on both platforms.",
    ),
    (
        "detail.video.res_scale",
        "Draws the 3D scene smaller than the window and stretches it back: a cheap way to buy frame \
         rate with sharpness. 1/2 is a quarter of the pixels, 1/4 a sixteenth. Needs the Render Tex \
         method — under Direct the scene is always drawn at the window's resolution — and the \
         automatic resolution takes the choice over while it is on. Default: Native.",
    ),
    (
        "detail.video.dynamic_resolution",
        "Lets the scale of the scene follow the time the GPU is taking, between native and 1/2, \
         instead of the fixed resolution scale above: the picture softens where a scene is heavy \
         and sharpens again where it is not. Needs the Render Tex method, the OpenGL back end, and \
         a known frame budget — that is, vertical synchronisation or a frame cap. Default: off, on \
         both platforms.",
    ),
    (
        "detail.video.anti_alias",
        "Smooths the staircase on the slanted edges of the 3D shapes. The first two levels are \
         multisampling — 4 and 8 samples per pixel — so the pixel art of the textures stays \
         sharp. 3dSen calls them FFAA and SMAA, which is also what the settings file and the \
         libretro option hold; Zaxnes draws them as what they are, MSAA 4× and MSAA 8×. SMAA \
         (depth) is a filter instead of extra samples: it draws the scene once and finds the edges \
         in the depth of the picture, so it smooths the shapes without ever touching a texture, and \
         it costs about a third of MSAA 4×'s memory. It leaves the status bar's own outline alone. \
         The cost of all three grows with the resolution. Default: MSAA 4× on the desktop, None on \
         a handheld.",
    ),
    (
        "detail.video.skip_frame",
        "When the emulation has produced no new frame since the last time the scene was drawn, the \
         previous picture is kept instead of drawing the same scene again. Nothing on screen \
         changes — the camera is the one exception, so turn it off if a slow camera move has to be \
         smoother than the game's own rate. Default: on, on both platforms.",
    ),
    (
        "detail.emu.catch_up",
        "How many emulated frames one displayed frame may run when the game has fallen behind, so \
         that it catches up rather than running slow. None lets a displayed frame run only the \
         frames the two rates call for, so a machine that cannot keep up slows the game down \
         instead. The higher the limit, the more a late frame can cost at once. Default: 2 on the \
         desktop, 5 on a handheld.",
    ),
    (
        "detail.emu.catch_up_last_3d",
        "When one displayed frame runs several emulated ones, only the last of them is turned into \
         3D: the others are still emulated in full — the game and its sound are exact — but they \
         are not converted and their script frame hooks do not run. It saves the conversion and the \
         Lua of every frame skipped that way; a profile whose script animates something on every \
         frame can jerk. A profile whose scripts write to the console is left alone: every frame of \
         it is converted. Default: off on the desktop, on a handheld.",
    ),
    (
        "detail.emu.pause_3d_in_2d",
        "While the flat 2D picture is on screen, none of the 3D scene is, so neither the conversion \
         nor the script frame hooks are run; the frame shown when the 3D comes back is converted \
         first. A profile whose scripts write to the console is left alone. Default: off on the \
         desktop, on a handheld.",
    ),
    (
        "detail.emu.lua_smooth_gc",
        "Tunes the garbage collector of the profile scripts so that it does a little work on every \
         frame instead of stopping for one long collection now and then — collections of 100 to \
         300 ms were measured without it. The only difference a script can see is the figure \
         collectgarbage(\"count\") reports. Default: on, on both platforms.",
    ),
    (
        "detail.emu.lua_optimized",
        "Runs the script bindings through optimised paths that use unsafe code: the host's values \
         are reached through C metamethods instead of the safe wrappers, for the same results and \
         less time per access. Off, only the safe paths run. The pipelined frames and the \
         experimental optimisation below both need it. Takes effect at the next game load. \
         Default: off on the desktop, on a handheld.",
    ),
    (
        "detail.emu.pipeline",
        "Runs the scripts and the 3D conversion of a frame on a thread of their own while the frame \
         before it is being drawn, which shows the picture one displayed frame later. Off never \
         does it; Auto does it only while the frames are late; Always does it whenever the game \
         allows it. Needs Optimized Lua — a pipelined frame hands the script runtime to another \
         thread — and is simply off without it. Default: Off on the desktop, Auto on a handheld.",
    ),
    (
        "detail.emu.lua_experimental",
        "On top of Optimized Lua: an UpdateS hook whose source only ever reads fields of shape is \
         given a light shape and no shape2D. Which hooks those are is read from the script when the \
         profile loads, and that reading is a heuristic — a profile it judges wrongly can \
         misbehave, which is what makes this experimental. Takes effect at the next game load. \
         Needs Optimized Lua. Default: off on the desktop, on a handheld.",
    ),
    (
        "detail.video.backend",
        "Which graphics API the window draws with, read once when Zaxnes starts. OpenGL 3.3 is the \
         default and the only choice on a handheld. Vulkan lets post-processing tools such as \
         ReShade read the depth of the 3D scene; DirectX 12 is Windows only and does not work with \
         ReShade yet. Neither Vulkan nor DirectX 12 draws the effects of the Enhanced style, and \
         neither applies the automatic resolution. Default: OpenGL.",
    ),
    (
        "detail.video.max_queued_frames",
        "How many drawn frames the graphics driver may still be working on before Zaxnes waits for \
         the oldest to finish. 2 lets the processor run a frame ahead of the graphics card, which \
         is the smoothest; 1 waits for each frame, which costs frames per second and shortens the \
         delay between a button press and the picture. It applies to the OpenGL back end — Vulkan \
         and DirectX 12 keep a queue of their own — and on a handheld nothing is waited for at 2. \
         Default: 2.",
    ),

    // --------------------------------------------------------- NES picture tab
    (
        "detail.video.nes_palette",
        "Which table of RGB colours the console's 64 colours are shown in, in the flat picture and \
         in the 3D view alike. The console puts out a composite signal, which every emulator \
         decodes into a table of its own; the first entry is 3dSen's own table, the others are \
         those of the main emulators and of a few measured televisions. Nothing but the colours \
         changes. Default: Mesen.",
    ),
    (
        "detail.video.remove_sprite_limit",
        "The console could only draw eight sprites on one scan line, which is why some games \
         flicker: they show their sprites every other frame to share the line. Disabled keeps the \
         limit and the flicker, Always draws them all, and Smart keeps the limit on a line where a \
         game deliberately stacks eight sprites in order to hide the ones behind them — what Mesen \
         does. It changes the flat 2D picture only: the 3D view has every sprite whatever is chosen \
         here. Default: Smart.",
    ),
    (
        "detail.video.overscan_mode",
        "What becomes of the space the overscan margins free. Fit leaves the picture the size it \
         was and simply does not draw the cut edges, so the scene's background shows through there. \
         Resize enlarges what is left until it fills the view again, in 2D. Default: Resize.",
    ),
    (
        "detail.video.overscan_ntsc_top",
        "How many pixel rows are cut from the top of the picture of an NTSC game. A television hid \
         the outermost rows, and many games leave scrolling rubbish there. 0 to 32 pixels; 8 is the \
         usual value for the top and the bottom. What happens to the freed space is the Overscan \
         mode row above. A game can set its own four margins in its options, which then win over \
         these. Default: 0 — the whole picture.",
    ),
    (
        "detail.video.overscan_ntsc_bottom",
        "How many pixel rows are cut from the bottom of the picture of an NTSC game. A television \
         hid the outermost rows, and many games leave scrolling rubbish there. 0 to 32 pixels; 8 is \
         the usual value for the top and the bottom. A game can set its own four margins in its \
         options, which then win over these. Default: 0 — the whole picture.",
    ),
    (
        "detail.video.overscan_ntsc_left",
        "How many pixel columns are cut from the left of the picture of an NTSC game. 0 to 32 \
         pixels; the sides usually need less than the top and the bottom. A widescreen margin is \
         never cut: the extra picture goes on past this edge. A game can set its own four margins \
         in its options, which then win over these. Default: 0 — the whole picture.",
    ),
    (
        "detail.video.overscan_ntsc_right",
        "How many pixel columns are cut from the right of the picture of an NTSC game. 0 to 32 \
         pixels; the sides usually need less than the top and the bottom. A widescreen margin is \
         never cut: the extra picture goes on past this edge. A game can set its own four margins \
         in its options, which then win over these. Default: 0 — the whole picture.",
    ),
    (
        "detail.video.overscan_pal_top",
        "How many pixel rows are cut from the top of the picture of a PAL game. A PAL television \
         showed more of the picture than an NTSC one, so these four margins are kept apart from the \
         NTSC ones; which pair is used follows the video region on the Emulation tab. 0 to 32 \
         pixels. Default: 0 — the whole picture.",
    ),
    (
        "detail.video.overscan_pal_bottom",
        "How many pixel rows are cut from the bottom of the picture of a PAL game. A PAL television \
         showed more of the picture than an NTSC one, so these four margins are kept apart from the \
         NTSC ones; which pair is used follows the video region on the Emulation tab. 0 to 32 \
         pixels. Default: 0 — the whole picture.",
    ),
    (
        "detail.video.overscan_pal_left",
        "How many pixel columns are cut from the left of the picture of a PAL game. Which pair of \
         margins is used follows the video region on the Emulation tab. A widescreen margin is \
         never cut: the extra picture goes on past this edge. 0 to 32 pixels. Default: 0 — the \
         whole picture.",
    ),
    (
        "detail.video.overscan_pal_right",
        "How many pixel columns are cut from the right of the picture of a PAL game. Which pair of \
         margins is used follows the video region on the Emulation tab. A widescreen margin is \
         never cut: the extra picture goes on past this edge. 0 to 32 pixels. Default: 0 — the \
         whole picture.",
    ),
    (
        "detail.video.widescreen_strict_edges",
        "Only for a ROM patched for the widescreen extension. Where the game has stopped drawing a \
         band of the wider picture, the shapes cut there are left unfinished — drawn with the \
         temporary pattern — until the whole band is back, so that nothing pops into sight \
         half-built. It is off by default because a one-tile object standing in such a band would \
         stay temporary for as long as it is there. It does nothing while the game produces no \
         widescreen margin; the widescreen itself is set for each game, in that game's own options. \
         Default: off.",
    ),

    // -------------------------------------------------------- 3D rendering tab
    (
        "detail.scene.skybox",
        "What is behind the 3D scene. Default takes the game's own background colour, falling back \
         on the colour below for a profile that turns that off. Color fills the background with the \
         colour below. Gradient and Fantasy are drawn from the game's own background colour and \
         palette, so the colour below does not reach them. Default: Default.",
    ),
    (
        "detail.scene.custom_bg_color",
        "The colour behind the scene when the background above is set to Color, and under the \
         Default background for a profile that turns the game's own colour off. The gradient and \
         fantasy skies are drawn from the game's colours and ignore it. Default: black.",
    ),
    (
        "detail.render.visual_style",
        "Enhanced is what the groups below need: the voxel grid and the reflections here, the pixel \
         shadows and the ambient occlusion on the Light and shadows tab. At Classic not one of them \
         is compiled, no extra pass runs and no extra buffer is allocated, so the whole family \
         costs exactly nothing. The Vulkan and DirectX 12 back ends do not draw the Enhanced \
         effects at all. Default: Classic, on both platforms.",
    ),
    (
        "detail.render.style.voxel_grid",
        "Carves a groove into the faces of the scene every 2, 4 or 8 voxels, so that a shape reads \
         as a stack of blocks instead of one solid. The smaller the spacing, the more grooves. A \
         per-pixel cost over the whole picture. Needs the Enhanced style. Default: Off.",
    ),
    (
        "detail.render.style.reflections",
        "Reflects the scene on the floor by marching through the depth buffer. It is the most \
         expensive effect of the Enhanced style, and the four rows under it decide how expensive. \
         Needs the Enhanced style. Default: off.",
    ),
    (
        "detail.render.style.ssr_shine",
        "1 to 5: how much of the scene the floor gives back, from barely damp to a mirror. It \
         changes the look, not the cost. Needs Reflections. Default: 3.",
    ),
    (
        "detail.render.style.ssr_clarity",
        "1 to 5: the higher the value, the longer a reflection stays sharp as it moves away from \
         the object it belongs to; a low value blurs it into the floor. Needs Reflections. \
         Default: 3.",
    ),
    (
        "detail.render.style.ssr_length",
        "1 to 5: how far across the floor a reflection reaches. A longer reflection is more \
         marching steps, so this one does cost. Needs Reflections. Default: 3.",
    ),
    (
        "detail.render.style.ssr_quality",
        "How many steps the march takes and how many times it refines what it finds, which is what \
         decides how few gaps the reflection has: Low is 12 steps and 2 refinements, High 48 and 5, \
         Ultra more again. This is the row to lower first on a small machine. Needs Reflections. \
         Default: High on the desktop, Low on a handheld.",
    ),
    (
        "detail.scene.bend_lr",
        "Degrees; 0 is a flat scene. The steps are 5 degrees between −30 and 30, where a slight \
         bend is worth tuning, and 30 degrees beyond, out to ±180. The whole level is bent, and \
         the camera and the Zapper aim follow it. It costs a little per vertex while any of the \
         three angles is not 0, and nothing at all while all three are 0. A script reaches it as \
         spl.bend_zx, and a script's write lands on the same steps the menu offers. Default: 0.",
    ),
    (
        "detail.scene.bend_tb",
        "Degrees; 0 is a flat scene. The steps are 5 degrees between −30 and 30 and 30 degrees \
         beyond, out to ±180. It costs a little per vertex while any of the three angles is not 0, \
         and nothing at all while all three are 0. A script reaches it as spl.bend_zy, and a \
         script's write lands on the same steps the menu offers. Default: 0.",
    ),
    (
        "detail.scene.arc_h",
        "Degrees; 0 is a flat scene. The steps are 5 degrees between −30 and 30 and 30 degrees \
         beyond, out to ±180. The Arc Center row below chooses what the level bends around. A \
         script reaches it as spl.arc_yx, and a script's write lands on the same steps the menu \
         offers. Default: 0.",
    ),
    (
        "detail.scene.arc_center",
        "The height the arc bends around: the middle of the screen, its bottom edge — eight units \
         below the picture — or one of the three quarters between the two, which put the pivot \
         that fraction of the way down. It does nothing while Arc Horizontal is 0. A script \
         reaches it as spl.arc_center, where 0 and 1 are the two ends, exactly as in 3dSen, and a \
         value in between lands on the nearest quarter. Default: Screen Center.",
    ),
    (
        "detail.scene.bend_2d_mode",
        "A setting of Zaxnes's own: 3dSen always bends the flat rectangle of the 2D mode with the \
         scene, and this is how to keep that rectangle flat while the 3D scene stays curved. It \
         does nothing in 3D mode, and nothing while all three angles are 0. No script reaches it — \
         the original program has no such setting. Default: on, on both platforms.",
    ),
    (
        "detail.render.mesh_algo",
        "How the voxels of a tile are turned into a mesh. GREEDY merges neighbouring faces into \
         the largest rectangles it can without looking at their colour, which gives the fewest \
         triangles; GREEDY_MULTICOLOR merges only faces of the same colour; MARCHING builds a \
         smoothed surface with marching cubes, which rounds the shapes off and costs the most. A \
         profile's shape can ask for an algorithm of its own, which then wins over this. \
         Default: GREEDY.",
    ),

    // --------------------------------------------------- Light and shadows tab
    (
        "detail.light.intensity",
        "How bright the one directional light of the scene is. The slider covers 0.5 to 1.5, 1 \
         being the light as the profile set it; a script or an imported profile may ask for more, \
         and a value of 2 set here survives a profile load. The direction and the colour of the \
         light belong to the profile and have no row. Default: 1.",
    ),
    (
        "detail.render.shadow",
        "Casts the shadow of the scene's shapes from the light, through one shadow map. Off, the \
         shadow pass is not run at all and nothing receives a shadow, which also leaves the pixel \
         shadows below with nothing to change. Which layers cast and which receive belongs to the \
         profile. Default: on on the desktop, off on a handheld, where the pass is worth the frame \
         it costs.",
    ),
    (
        "detail.render.backdrop_lighting",
        "Only some profiles put a backdrop behind the scene at all — it is turned on by a script, \
         and off by default — so on most games this switch shows nothing. Where there is one, the \
         sheen lifts a black surface to about sRGB 26 and gives it the light's tint. It is a \
         per-fragment cost on a surface that can fill the picture: free within the noise on a \
         desktop GPU at 640 × 480 and 1280 × 720, which is why the handheld default is the cheaper \
         one. Default: on on the desktop, off on a handheld.",
    ),
    (
        "detail.render.style.pixel_shadow",
        "Snaps the shadow edge to the game's own pixel grid instead of letting it fall where the \
         geometry says, so a shadow looks drawn rather than computed. It does not add a shadow: it \
         changes how the shadows the switch above casts are drawn, and has nothing to do while \
         those are off. Needs the Enhanced style. Default: off.",
    ),
    (
        "detail.render.style.ao",
        "Darkens the corners and the hollows the light does not really reach, which is a different \
         effect from the shadows above — it needs no light direction and no shadow map. Off costs \
         nothing. MSVO is the original's own method and Depth a cheaper one that works from the \
         depth buffer; this version renders both as Depth, which is why Thickness below is greyed. \
         Needs the Enhanced style. Default: Off.",
    ),
    (
        "detail.render.style.ao_strength",
        "1 to 5: how dark the darkest corner gets. It changes the look, not the cost. Needs ambient \
         occlusion. Default: 3.",
    ),
    (
        "detail.render.style.ao_radius",
        "1 to 5: how far from a corner the shading still reaches. A larger radius spreads the same \
         samples wider, so the shading gets coarser rather than more expensive. Needs ambient \
         occlusion. Default: 3.",
    ),
    (
        "detail.render.style.ao_thickness",
        "1 to 5: how thick a surface is assumed to be when deciding whether it occludes what is \
         behind it. Only the MSVO method uses it, and this version renders MSVO as Depth, so the \
         row is greyed; the value is kept and persisted all the same. Default: 3.",
    ),
    (
        "detail.render.style.ao_quality",
        "How many samples each pixel takes and at what resolution, which is what the occlusion \
         actually costs: Low is 6 samples at half resolution, Medium 10. This is the row to lower \
         first on a small machine. Needs ambient occlusion. Default: Medium on the desktop, Low on \
         a handheld.",
    ),

    // -------------------------------------------------------------- Camera tab
    (
        "detail.camera.fov",
        "The field of view of the 3D camera, in degrees: a small angle flattens the scene and a \
         large one exaggerates its depth. The slider covers 45 to 95 in steps of 5; a script may \
         ask for anything from 10 to 150. Changing it plays no validation sound, the value moving \
         continuously under the mouse. Default: 70.",
    ),

    // --------------------------------------------------------------- Audio tab
    (
        "detail.audio.volume",
        "The gain applied to everything Zaxnes plays, from 0.00 to 1.00. Mute, in the Audio menu, \
         silences the sound without touching this value, so turning mute off brings the same volume \
         back. Changing it plays no validation sound. Default: 1.00.",
    ),
    (
        "detail.audio.latency_ms",
        "How much sound is kept buffered ahead of the device, from 20 to 200 milliseconds. A larger \
         value survives a longer hitch of the main thread without a crackle and delays the sound of \
         a button press by the same amount; a smaller one is more immediate and less forgiving. \
         Above what the core's sample ring can hold — about 138 ms at 48 kHz — it is clamped and \
         the log says so. Changing it empties the buffer and fills it again, which is a short dip. \
         It applies to the default output only — the classic one below has its own fixed buffer, \
         and the row is greyed while that is on. Default: 60 ms.",
    ),
    (
        "detail.audio.classic_output",
        "Puts back the audio output Zaxnes had in version 0.4.2: an output buffer of four blocks \
         of 512 frames, about 43 ms, topped up from the main thread once per drawn frame. Use it \
         if the new output misbehaves on this device. It is the shorter buffer of the two and it \
         sends out a block of silence whenever the emulation is late with its samples, so it can \
         crackle — which is what the new output was written to stop. The output latency setting \
         and the latency, rate and under-run counters of the performance overlay belong to the \
         new output and do nothing here. The choice takes effect when the sound device is \
         reopened, which ticking this box does at once. Default: off.",
    ),

    // ----------------------------------------------------------- Emulation tab
    (
        "detail.emu.game_speed",
        "Runs the game faster or slower, 0 being its normal speed. The scale is not symmetric: the \
         factor is 1 + s/4 above 0 and 1 / (1 − s/4) below, so +8 is three times as fast and −8 a \
         third as fast. The sound follows. There is no separate slow-motion mode — the negative \
         half is it — and the Fast forward shortcut always uses its own +2 (one and a half times), \
         whatever is set here. The value goes back to 0 when a game ends. Changing it plays no \
         validation sound. Default: 0.",
    ),
    (
        "detail.emu.run_ahead",
        "Hides the console's own input delay by emulating one, two or three frames beyond the one \
         on screen and showing the last of them, so a button press takes effect sooner. Each frame \
         asked for costs a whole extra emulated frame plus a state saved and loaded, every \
         displayed frame — which is why a handheld has it off. A game whose profile scripts write \
         to the console is held at 0 while it runs, and the row says so beside it. Default: 1 frame \
         on the desktop, off on a handheld.",
    ),
    (
        "detail.emu.smooth_scroll_ahead",
        "Experimental. On a display faster than the console, draws the scrolling part of the 2D \
         picture between two emulated frames: the picture is shown shifted back to the scroll the \
         run-ahead's real frame drew, and slides to its own over the frame, which spends one of \
         the run-ahead's frames of latency on smoother motion. Status bars and the sprites that \
         move with the camera stay where they are. Nothing happens without run-ahead, at 60 \
         frames a second or less, or in the 3D scene — only the 2D picture is interpolated. A \
         game whose hack pack reports its own camera uses that instead. Default: off.",
    ),
    (
        "detail.emu.smooth_scroll_view",
        "A debugging view of the setting above: the band of lines the smooth scrolling slides is \
         tinted red and the sprites it holds still are tinted green, so what it moves and what it \
         leaves alone can be seen at a glance. Nothing is tinted while the smooth scrolling does \
         nothing. Default: off.",
    ),
    (
        "detail.emu.auto_tv_mode",
        "Reads the region out of the ROM header when a game is loaded and writes it into the row \
         below, so that a European dump runs at 50 Hz on its own. It is Zaxnes's own default — \
         3dSen has it off — and turning it off is what makes the row below yours to choose. Takes \
         effect at the next ROM load. Default: on.",
    ),
    (
        "detail.emu.tv_mode",
        "Which console the game is emulated as. NTSC runs at about 60 frames per second and PAL at \
         50, with the different timing and sound that go with them; MULTI is run as NTSC; Dendy is \
         the Soviet-era clone, which runs at 50 with a timing of its own. It also chooses which of \
         the two sets of overscan margins applies. It is a profile setting, so loading a profile \
         overwrites it, and so does the automatic detection above while that is on.",
    ),
    (
        "detail.input.emulated_menu_button",
        "A 3dSen setting Zaxnes keeps so that a settings file and a script written for 3dSen read \
         back exactly what they wrote. Nothing in Zaxnes acts on it: the console interface reaches \
         its own menu by holding Select and Start, which is not this setting. Default: off.",
    ),
    (
        "detail.emu.core",
        "Which emulation core runs the game, in a version that has more than one. This one has a \
         single core, so the value is taken modulo 1: anything written here reads back as 0 and \
         nothing uses it. The row is kept because the key is part of the settings file and of the \
         script surface (Setting.Core). Default: 0.",
    ),
    (
        "detail.emu.cartridge_db",
        "Completes what an old iNES header cannot say about a cartridge, from a database of about 10 000 dumps: the chip revision (an MMC3A or an MMC1A, say), the submapper, the real save RAM, the battery, the region. It never changes the mapper number and never touches a NES 2.0 header. Takes effect at the next load. Default: on.",
    ),
    (
        "detail.emu.oam_corruption",
        "A defect of the real console: switching the picture off in the middle of a line damages one row of the sprite memory when the picture comes back, so a few sprites can go missing for a frame. Games written for the console avoid it or live with it. The known model is incomplete, so a game may show sprites flickering that it should not with this on. Takes effect at the next load. Default: off.",
    ),
    (
        "detail.emu.ppu_bus_conflicts",
        "A defect of the real console: a game that reads or moves the video address while the picture is being drawn collides with what the video chip is reading at that instant, and one background tile is read from the wrong place, which shows as eight wrong pixels on one line. Games written for the console avoid it. Turn this off if a game shows such a glitch and you would rather not see it. Takes effect at the next load. Default: off.",
    ),
    (
        "detail.emu.oamaddr_bug",
        "A defect of the NES and Famicom video chip (2C02G and later): if a game leaves the sprite memory address at 8 or more when the picture starts, eight bytes of sprite memory are copied over the first eight, so a couple of sprites can take the place of others for a frame. Games written for the console reset that address and never show it. A PAL console does not have this defect, so it never applies there. Takes effect at the next load. Default: on.",
    ),
    (
        "detail.emu.oam_decay",
        "The console keeps its sprites in a memory that fades unless the picture is being drawn: if a game leaves the screen blank for long enough (about 8 frames here, it varies from one console to another) and then shows sprites without sending them again, they come back scrambled. Games written for the console send their sprites every frame and never show it. Run-ahead and rewind restart the delay at every frame, so with them on it never happens. Takes effect at the next load. Default: off.",
    ),
    (
        "detail.emu.power_on_top",
        "Where the video chip is in its picture when the console is switched on. The real console starts at the top of the picture; Zaxnes has always started at the end of it, which shortens the wait of a game for its first frame by a fraction of a frame. On, games start as on the console, a fraction of a frame later than before, so recordings, saved states made right after switching on and some widescreen adjustments made the old way can drift. Takes effect at the next load. Default: off.",
    ),
    (
        "detail.emu.power_on_ram",
        "What the console's 2 KiB of work RAM hold when it is switched on. A real NES starts with whatever its memory settled to; most games clear it first, a few read it before. The FCEUX pattern (00 00 00 00 FF FF FF FF), the default, is what FCEUX and BizHawk use and what a few games need; Zeros is what Mesen uses; Random changes at every power-on. Takes effect at the next load. Default: FCEUX pattern.",
    ),
    (
        "detail.emu.engine",
        "Which emulator plays this game: Zaxnes's own core, FCEUX, or NesHawk. It is a choice per \
         game, not a preference, because the reason to reach for another engine is always a \
         particular game. FCEUX is for coverage — a game Zaxnes runs badly, or on a board it \
         does not have. NesHawk is the Rust port of BizHawk's own core, and it is what makes a \
         BizHawk movie replay exactly: a .bk2 recorded on NesHawk plays back on it frame for \
         frame, which no other engine can promise. It takes effect at the next load, since \
         changing engines means starting the machine again. Neither FCEUX nor NesHawk has \
         widescreen, hack packs, a sprite-limit control or run-ahead: those are part of Zaxnes's \
         own machine, and their rows say so while another engine is chosen. A save state made on \
         one engine cannot be loaded on another. Default: Zaxnes.",
    ),

    // ----------------------------------------------------------- Interface tab
    (
        "detail.ui.language",
        "The language of the whole interface — the menus, the windows, the notifications and this \
         tooltip. The change applies at once, with no restart, and is kept for the next launch. A \
         text a translation is missing falls back to English. The console log and the command line \
         stay in English whatever is chosen. Default: the language of the system on a fresh user \
         folder, English where that is not French.",
    ),
    (
        "detail.ui.roms_folder",
        "Which folder the console mode lists games from. Empty means the roms folder beside your \
         Zaxnes data, which is what the field shows in grey, and the --roms option on the command \
         line wins over both. It is written when you press Enter or leave the field; the button \
         beside it opens a folder picker. Nothing else in Zaxnes reads it.",
    ),
    (
        "detail.ui.sort_option",
        "The order the library lists games in: A-Z by title, Recent first by the last time each \
         was played, or Favorites first — the games marked as favourites at the top, each group \
         then by title. Default: A-Z.",
    ),
    (
        "detail.ui.show_fps",
        "Shows a frame counter in a corner of the picture: the displayed frames per second, which \
         is the render rate and not the speed of the game. Costs nothing worth measuring. \
         Default: off.",
    ),
    (
        "detail.ui.profiler",
        "Measures what each stage of a frame costs — the core, the conversion, the scripts, the \
         draw list, the drawing — and writes the figures beside the frame counter and into \
         logs/zaxnes.log every two seconds. It is a diagnostic, not a setting to play with: the \
         measuring itself takes a little time and the log grows while it is on. Default: off.",
    ),
    (
        "detail.ui.editor_mode",
        "Turns on what is needed to write a 3D profile with an AI agent: the editor folder of \
         your user data is laid out with its documents and templates, an MCP server listens on \
         the loopback so the agent can look at the running game, and the pens let you point at a \
         pattern on screen. It changes nothing about how a game plays or looks; it is for making \
         profiles, not for playing. See docs/EDITOR.md. Default: off.",
    ),

    // ---------------------------------------------------- RetroAchievements tab
    (
        "detail.achievements.enabled",
        "Turns RetroAchievements on: the ROM of each game you start is identified against the \
         server, its achievements are fetched, and unlocks are sent as you earn them. Zaxnes plays \
         in softcore only, so save states, rollback and fast forward all stay allowed and the \
         unlocks count as softcore. Nothing happens until you sign in, above. The four rows below \
         do nothing while this is off. Default: off.",
    ),
    (
        "detail.achievements.sound",
        "Plays a short sound when an achievement unlocks, over the pop-up that announces it. Needs \
         RetroAchievements. Default: on.",
    ),
    (
        "detail.achievements.challenges",
        "Shows a small icon in a corner of the picture for each challenge the game has started — a \
         timed or conditional achievement you are in the middle of. Needs RetroAchievements. \
         Default: off.",
    ),
    (
        "detail.achievements.unofficial",
        "Also loads the achievements the site keeps as unofficial: ones under test, or retired. \
         They can fire wrongly, which is why they are not part of a normal session. Read when a \
         game loads, so it takes a reload to change. Needs RetroAchievements. Default: off.",
    ),
    (
        "detail.achievements.encore",
        "Arms the achievements you have already unlocked again, so that they fire a second time as \
         you replay a game. The server is told as usual; nothing is lost. Read when a game loads, \
         so it takes a reload to change. Needs RetroAchievements. Default: off.",
    ),

    // ------------------------------------------- handheld interface (Rust-only)
    (keys::HANDHELD_GAMES_TITLE, "Games"),
    (keys::HANDHELD_ROMS_EMPTY, "No ROM found. Copy .nes files to {0}"),
    (keys::HANDHELD_TAG_2D, "2D"),
    (keys::HANDHELD_PROFILE_FOR, "Profile for {0}"),
    (keys::HANDHELD_HINT_PLAY, "Play"),
    (keys::HANDHELD_HINT_PROFILE, "3D Profile"),
    (keys::HANDHELD_HINT_OPTIONS, "Options"),
    (keys::HANDHELD_HINT_BACK, "Back"),
    (keys::HANDHELD_HINT_JUMP, "A-Z"),
    (keys::HANDHELD_HINT_CHOOSE, "Choose"),
    (keys::HANDHELD_HINT_CHANGE, "Change"),
    (keys::HANDHELD_HINT_REBIND, "Rebind"),
    (keys::HANDHELD_HINT_CANCEL, "Cancel"),
    (keys::HANDHELD_EXIT_HOLD, "Keep Select + Start held to quit"),
    (keys::HANDHELD_MENU_TITLE, "Menu"),
    (keys::HANDHELD_MENU_RESUME, "Resume"),
    (keys::HANDHELD_STATES_SAVE_TITLE, "SAVE — choose a slot"),
    (keys::HANDHELD_STATES_LOAD_TITLE, "LOAD — choose a slot"),
    (keys::HANDHELD_STATES_NO_PICTURE, "No picture"),
    (keys::HANDHELD_HINT_SAVE_HERE, "Save here"),
    (keys::HANDHELD_HINT_LOAD, "Load"),
    (keys::HANDHELD_HINT_PAGE, "Page"),
    (keys::HANDHELD_GAME_OPTIONS_TITLE, "Game options — {0}"),
    (keys::HANDHELD_GAME_OPTIONS_RESET, "Put everything back to global"),
    (keys::HANDHELD_USE_GLOBAL, "Use Global : {0}"),
    (keys::HANDHELD_GLOBAL_VALUE, "Global: {0}"),
    (keys::HANDHELD_OWN_COUNT, "{0} own"),
    (keys::HANDHELD_RUN_AHEAD_FORCED, "(forced to 0: scripts)"),
    (keys::HANDHELD_ZAXNES_ONLY, "(Zaxnes engine only)"),
    (keys::HANDHELD_HINT_GAME_OPTIONS, "Game options"),
    (keys::HANDHELD_HINT_FAVORITE, "Favorite"),
    (keys::HANDHELD_HINT_RESET_ALL, "Reset all"),
    (keys::HACKS_TITLE, "Hacks — {0}"),
    (keys::HACKS_NONE, "No hack is made for this game."),
    (keys::HACKS_PATCH_LIVE_NOTE, "The ROM patch follows the widescreen: turned on or off during a game, it switches at once and the game goes on where it was."),
    (keys::HACKS_PATCH_NOTE, "The ROM patch follows the widescreen when the game starts: turned on or off during a game, it takes effect at the next start."),
    (keys::HACK_WIDESCREEN, "Widescreen"),
    (keys::HACK_RATIO, "Ratio"),
    (keys::HACK_MARGIN, "Extra margin"),
    (keys::HACK_ADVANCED, "Advanced mode (enemies in the margins)"),
    (keys::HACK_ON, "on"),
    (keys::HACK_FORCE, "Force it"),
    (keys::HACKS_FORCED_NOTE, "The widescreen hack of {0}, made for the ROM \u{201c}{1}\u{201d} (CRC {2}), is forced on this one: {3} (CRC {4}). It takes effect when the game starts; if the game does not run with it, take the forcing back."),
    (keys::HACKS_FORCE_QUESTION, "Force it all the same? A ROM hack close to the original stands a good chance of running with it; otherwise the game may crash."),
    (keys::HACKS_FORCE_BUTTON, "Force"),
    (keys::HACKS_PACK_SIGNATURES, "Nor does it have the code the hack changes where it changes it: the hack cannot apply to it."),
    (keys::HACK_COMPAT, "Hack compatibility"),
    (keys::HACK_COMPAT_ORIGINAL, "Original ROM"),
    (keys::HACK_COMPAT_FULL, "Compatible"),
    (keys::HACK_COMPAT_PARTIAL, "Partial"),
    (keys::HACK_COMPAT_NONE, "Incompatible"),
    (keys::HACKS_COMPAT_ORIGINAL, "Original ROM: the hack is made for this one (CRC {0}). It applies whole."),
    (keys::HACKS_COMPAT_FULL, "Compatible - {0}/{1}"),
    (keys::HACKS_COMPAT_FULL_BODY, "Not the original ROM (CRC {0}), but all the code the hack changes is there."),
    (keys::HACKS_COMPAT_PARTIAL, "Partial - {0}/{1}"),
    (keys::HACKS_COMPAT_PARTIAL_BODY, "Some parts don't apply; the game keeps its own there."),
    (keys::HACKS_COMPAT_NONE, "The hack can't apply to this ROM."),
    (keys::HACKS_COMPAT_MAPPER, "Mapper {0}: the hack is made for {1}."),
    (keys::HACKS_COMPAT_REGION, "{0} ROM: this pack is made for a {1} ROM."),
    (keys::HACKS_COMPAT_PART_DIFFERS, "x {0} {1}: the game's code differs at {2}"),
    (keys::HACKS_COMPAT_PART_NEEDS, "x {0}: needs a part that doesn't apply"),
    (keys::HACKS_COMPAT_EXPECTED, "Expected ROM: {0}, CRC {1}"),
    (keys::HACKS_FORCE_SIZE, "Its size is not the one the hack is made for: it cannot be forced either."),
    (keys::HACKS_OTHER_ROM, "The widescreen hack of {0} is made for the ROM \u{201c}{1}\u{201d} (CRC {2}). This ROM does not match it: {3} (CRC {4}). The widescreen cannot be turned on for it."),
    (keys::HACK_OFF, "off"),
    (keys::MENU_HACKS, "Hacks..."),
    (keys::MENU_WIDESCREEN_SETTINGS, "Widescreen settings..."),
    (keys::MENU_AIDS, "Hacks"),
    (keys::HANDHELD_HOLD_FOR_SETTINGS, "hold A: settings"),
    (keys::HANDHELD_HINT_HACKS, "WideScreen & Hacks"),
    (keys::SHORTCUT_HOME, "Home"),
    (keys::HANDHELD_MENU_SAVE_STATE, "Save state"),
    (keys::HANDHELD_MENU_LOAD_STATE, "Load state"),
    (keys::HANDHELD_MENU_GAME_SPEED, "Game speed"),
    (keys::HANDHELD_MENU_QUIT_TO_LIST, "Quit to ROM list"),
    (keys::HANDHELD_MENU_EXIT, "Exit to launcher"),
    (keys::HANDHELD_OPTION_RENDER_SCALE, "Render scale"),
    (keys::HANDHELD_OPTION_FRAME_CAP, "Frame cap"),
    (keys::HANDHELD_OPTION_CAMERA, "Camera preset"),
    (keys::HANDHELD_OPTION_CONTROLS, "Controls"),
    (keys::HANDHELD_OPTION_REGION, "Region"),
    (keys::HANDHELD_OPTION_OVERSCAN, "Overscan"),
    (keys::HANDHELD_OPTION_CATCH_UP, "Catch-up"),
    (keys::HANDHELD_OPTION_PIPELINE, "Parallel scripts"),
    (keys::HANDHELD_OPTION_OPTIMIZATIONS, "Optimizations"),
    (keys::HANDHELD_REGION_AUTO, "Auto (ROM)"),
    (keys::HANDHELD_OPTION_ADVANCED, "Advanced options"),
    (keys::HANDHELD_OPTION_TOUCH_PAD, "Touch pad"),
    (keys::HANDHELD_OPTION_TOUCH_SCREEN, "Touch screen"),
    (keys::HANDHELD_TOUCH_PAD_ON, "Yes"),
    (keys::HANDHELD_TOUCH_PAD_OFF, "No"),
    (keys::HANDHELD_CAMERA_RESET, "Default view"),
    (keys::HANDHELD_CAMERA_KEPT, "Camera kept"),
    (keys::HANDHELD_CAMERA_RESTORED, "Camera restored"),
    (keys::HANDHELD_CONTROLS_PLAYER, "Player 1 gamepad"),
    // ----------------------------------- RetroAchievements (Rust-only)
    (keys::TAB_ACHIEVEMENTS, "RetroAchievements"),
    (keys::ACH_LIST, "Achievements of the game"),
    (keys::ACH_LIST_EMPTY, "No achievements: no game is running, or RetroAchievements doesn't know it."),
    (keys::ACH_GRID_STUDYING, "Asking RetroAchievements about this game..."),
    (keys::ACH_GRID_SIGNED_OUT, "Sign in to RetroAchievements (Options > RetroAchievements) to see the achievements."),
    (keys::ACH_GRID_NONE, "No achievements: RetroAchievements doesn't know this game, or it could not be reached."),
    (keys::ACH_ACCOUNT, "Account and options..."),
    (keys::ACH_SIGNED_OUT, "Not signed in."),
    (keys::ACH_SIGNING_IN, "Signing in..."),
    (keys::ACH_SIGNED_IN, "Signed in as {0} ({1} softcore points)."),
    (keys::ACH_USERNAME, "User name"),
    (keys::ACH_PASSWORD, "Password"),
    (keys::ACH_SIGN_IN, "Sign in"),
    (keys::ACH_SIGN_OUT, "Sign out"),
    (keys::ACH_PASSWORD_HINT, "The password goes once to retroachievements.org and is never saved: a token keeps you signed in."),
    (keys::ACH_SOFTCORE_HINT, "Softcore mode: save states, rollback and fast forward stay allowed; the unlocks count as softcore."),
    (keys::ACH_TURN_ON_HINT, "Enable RetroAchievements to sign in."),
    (keys::ACH_UNLOCKED, "Achievement unlocked"),
    (keys::ACH_UNLOCKED_TEXT, "{0} ({1} pts)"),
    (keys::ACH_COMPLETED, "Game completed!"),
    (keys::ACH_COMPLETED_TEXT, "Every achievement of {0} is unlocked."),
    (keys::ACH_GAME_SUMMARY, "{0} of {1} achievements unlocked ({2}/{3} points, softcore)"),
    (keys::ACH_PERCENT, "{0}%"),
    (keys::ACH_GAME_NO_ACHIEVEMENTS, "This game has no achievements yet."),
    (keys::ACH_GAME_UNKNOWN, "RetroAchievements doesn't know this ROM."),
    (keys::ACH_GAME_FAILED, "RetroAchievements: the game's achievements did not load ({0})."),
    (keys::ACH_WELCOME, "RetroAchievements: signed in as {0} ({1} softcore points)."),
    (keys::ACH_SIGN_IN_FAILED, "RetroAchievements: sign-in failed ({0})."),
    (keys::ACH_SERVER_ERROR, "RetroAchievements: {0}"),
    (keys::ACH_DISCONNECTED, "RetroAchievements: the server can't be reached, the unlocks will be sent later."),
    (keys::ACH_RECONNECTED, "RetroAchievements: the waiting unlocks are sent."),
    (keys::ACH_CHALLENGE, "Challenge"),
    (keys::ACH_POINTS, "{0} pts"),
    (keys::ACH_RARITY, "{0} % of players"),
    (keys::ACH_BUCKET_LOCKED, "Locked"),
    (keys::ACH_BUCKET_UNLOCKED, "Unlocked"),
    (keys::ACH_BUCKET_RECENT, "Unlocked this session"),
    (keys::ACH_BUCKET_CHALLENGE, "Challenges under way"),
    (keys::ACH_BUCKET_ALMOST, "Almost there"),
    (keys::ACH_BUCKET_UNOFFICIAL, "Unofficial"),
    (keys::ACH_BUCKET_UNSUPPORTED, "Not supported"),
    (keys::ACH_BUCKET_UNSYNCED, "Waiting for the server"),
    (keys::ACH_KEY_TYPE, "Type"),
    (keys::ACH_KEY_DELETE, "Delete"),
    (keys::ACH_KEY_SHIFT, "Shift"),
    (keys::ACH_KEY_SPACE, "Space"),
    (keys::ACH_KEY_DONE, "OK"),
    ("setting.achievements.enabled", "Enable RetroAchievements"),
    ("setting.achievements.encore", "Encore mode (unlocked achievements fire again)"),
    ("setting.achievements.unofficial", "Test the unofficial achievements"),
    ("setting.achievements.sound", "Unlock sound"),
    ("setting.achievements.challenges", "Show the challenges under way"),
    // --------------------------------------------------------- shaders window
    ("cmd.ui.show_shaders", "Shaders…"),
    ("shaders.title", "Shaders"),
    ("shaders.not_built", "This build of Zaxnes has no slang shaders."),
    ("shaders.pack_missing", "This build of Zaxnes was made without its shader pack."),
    ("shaders.needs_wgpu", "The shaders need the Vulkan or DirectX 12 renderer (Graphics API option)."),
    ("shaders.use_vulkan", "Use Vulkan (after a restart)"),
    ("shaders.restart", "Restart Zaxnes to use the new renderer."),
    ("shaders.slot.2d", "2D"),
    ("shaders.slot.2d_ws", "2D widescreen"),
    ("shaders.slot.3d", "3D"),
    ("shaders.slot.3d_ws", "3D widescreen"),
    ("shaders.slot.on_screen", "{0} (on screen)"),
    ("shaders.mode", "Shader"),
    ("shaders.mode.off", "None"),
    ("shaders.mode.preset", "Preset"),
    ("shaders.mode.follow", "Same as {0}"),
    ("shaders.lines", "Picture lines"),
    ("shaders.lines.window", "The window's"),
    ("shaders.lines.console", "240 (the console's pixels)"),
    ("shaders.lines.help", "The lines of the picture the preset reads: 240 give it the console's own pixels, which CRT and pixel-art presets are written for; the window's keep the detail of the 3D scene."),
    ("shaders.frame", "Framing"),
    ("shaders.frame.auto", "Auto"),
    ("shaders.frame.window", "Whole window"),
    ("shaders.frame.picture", "Game picture"),
    ("shaders.frame.help", "What of the 3D scene the preset reads: the whole window, or the scene framed on the game's picture (the NES picture's shape, about 4:3), so that a TV holds the game alone at full size. Auto frames on the picture for the presets that draw a TV or a bezel."),
    ("shaders.follows", "This mode uses the shader of the {0} tab."),
    ("shaders.none", "No shader in this mode."),
    ("shaders.copy", "Copy from"),
    ("shaders.copy.none", "—"),
    ("shaders.copy.help", "Takes the preset, its parameters and its area from another tab; the picture lines stay this tab's."),
    ("shaders.quick", "Recommended:"),
    ("shaders.preset", "Preset"),
    ("shaders.preset.choose", "Choose a preset"),
    ("shaders.menu.recommended", "Recommended for this mode"),
    ("shaders.menu.all", "Every preset of the pack"),
    ("shaders.menu.empty", "The shader pack has not been written to the user folder."),
    ("shaders.family.crt", "CRT TV"),
    ("shaders.family.composite", "Composite signal (NTSC)"),
    ("shaders.family.bezel", "TV with bezel (Mega Bezel)"),
    ("shaders.family.lcd", "Handheld LCD"),
    ("shaders.family.smoothing", "Smoothing / sharp pixels"),
    ("shaders.family.combination", "Combinations"),
    ("shaders.prefilter", "Pre-filter"),
    ("shaders.prefilter.help", "Enlarges or smooths the picture before the preset, as RetroArch's Append does: the passes of both, one after the other."),
    ("shaders.prefilter.none", "None"),
    ("shaders.prefilter.xbrz2", "xBRZ ×2"),
    ("shaders.prefilter.xbrz4", "xBRZ ×4"),
    ("shaders.prefilter.scalefx", "ScaleFX ×3"),
    ("shaders.prefilter.nnedi3_2", "nnedi3 ×2"),
    ("shaders.prefilter.nnedi3_4", "nnedi3 ×4"),
    ("shaders.backend", "Graphics API (at the next start)"),
    ("shaders.backend.gl", "OpenGL: no shaders"),
    ("shaders.backend.vulkan", "Vulkan: shaders"),
    ("shaders.backend.not_built", "OpenGL (this build has no Vulkan)"),
    ("shaders.cost.light", "light"),
    ("shaders.cost.medium", "medium"),
    ("shaders.cost.heavy", "heavy"),
    ("shaders.preset.guest", "CRT Guest Advanced"),
    ("shaders.preset.guest.help", "The most complete CRT tube: scanlines, mask, glow, curvature."),
    ("shaders.preset.easymode", "CRT Easymode"),
    ("shaders.preset.easymode.help", "A simple, light CRT."),
    ("shaders.preset.zfast", "zfast CRT"),
    ("shaders.preset.zfast.help", "The lightest CRT, for weak cards."),
    ("shaders.preset.crt_pi", "CRT Pi"),
    ("shaders.preset.crt_pi.help", "A light CRT written for the Raspberry Pi."),
    ("shaders.preset.nes_mini", "CRT NES Mini"),
    ("shaders.preset.nes_mini.help", "The scanlines of the NES Classic Mini."),
    ("shaders.preset.lottes", "CRT Lottes"),
    ("shaders.preset.lottes.help", "Timothy Lottes's CRT: sharp scanlines and a shadow mask."),
    ("shaders.preset.hyllian", "CRT Hyllian"),
    ("shaders.preset.hyllian.help", "A sharp CRT with bright colours."),
    ("shaders.preset.geom", "CRT Geom"),
    ("shaders.preset.geom.help", "A curved CRT screen."),
    ("shaders.preset.royale", "CRT Royale"),
    ("shaders.preset.royale.help", "A very faithful phosphor mask, for strong cards."),
    ("shaders.preset.guest_ntsc", "CRT Guest NTSC"),
    ("shaders.preset.guest_ntsc.help", "The Guest tube fed by a composite signal, as the NES was plugged in."),
    ("shaders.preset.ntsc_nes", "NES composite"),
    ("shaders.preset.ntsc_nes.help", "The colour bleed of a 256-pixel composite signal, with scanlines."),
    ("shaders.preset.blargg", "Blargg NTSC"),
    ("shaders.preset.blargg.help", "Blargg's NTSC filter, the one of many NES emulators."),
    ("shaders.preset.mega_bezel", "Mega Bezel"),
    ("shaders.preset.mega_bezel.help", "A TV with its bezel, its reflection and its room, at the picture's aspect."),
    ("shaders.preset.mega_bezel_light", "Mega Bezel (light)"),
    ("shaders.preset.mega_bezel_light.help", "The Mega Bezel TV without the reflection, for smaller cards."),
    ("shaders.preset.mega_bezel_reflect", "Mega Bezel (advanced)"),
    ("shaders.preset.mega_bezel_reflect.help", "The Mega Bezel TV with every effect, for strong cards."),
    ("shaders.preset.mega_bezel_screen", "Mega Bezel screen"),
    ("shaders.preset.mega_bezel_screen.help", "The Mega Bezel tube alone over the whole picture, of any width: no 4:3 frame around a widescreen game."),
    ("shaders.preset.mega_bezel_screen_ntsc", "Mega Bezel screen NTSC"),
    ("shaders.preset.mega_bezel_screen_ntsc.help", "The Mega Bezel tube alone, fed by a composite signal."),
    ("shaders.preset.mega_bezel_screen_light", "Mega Bezel screen (light)"),
    ("shaders.preset.mega_bezel_screen_light.help", "The light Mega Bezel tube alone over the whole picture."),
    ("shaders.preset.lcd", "LCD grid"),
    ("shaders.preset.lcd.help", "The pixel grid of a handheld's LCD."),
    ("shaders.preset.sharp", "Sharp pixels"),
    ("shaders.preset.sharp.help", "Square pixels without shimmering at any size."),
    ("shaders.preset.xbrz", "xBRZ"),
    ("shaders.preset.xbrz.help", "Pixel art smoothed into curves."),
    ("shaders.preset.scalefx", "ScaleFX"),
    ("shaders.preset.scalefx.help", "Pixel art smoothed, edges kept sharp."),
    ("shaders.preset.scalefx_guest_hd", "ScaleFX + CRT Guest HD"),
    ("shaders.preset.scalefx_guest_hd.help", "Pixel art smoothed by ScaleFX, then the Guest tube made for a smoothed picture."),
    ("shaders.preset.nnedi3_hyllian", "nnedi3 + CRT Hyllian"),
    ("shaders.preset.nnedi3_hyllian.help", "The picture enlarged four times by nnedi3, then the sharp Hyllian tube."),
    ("shaders.preset.nnedi3_easymode", "nnedi3 + CRT Easymode"),
    ("shaders.preset.nnedi3_easymode.help", "The picture enlarged four times by nnedi3, then the simple Easymode tube."),
    ("shaders.preset.nnedi3_mattias", "nnedi3 + CRT Mattias + scanlines (4K TV)"),
    ("shaders.preset.nnedi3_mattias.help", "The picture enlarged four times by nnedi3, the soft Mattias tube and scanlines over it: for a 4K screen."),
    ("shaders.preset.fakelottes", "Fake Lottes"),
    ("shaders.preset.fakelottes.help", "A light take on the Lottes CRT."),
    ("shaders.preset.mattias", "CRT Mattias"),
    ("shaders.preset.mattias.help", "A soft CRT with a gentle glow."),
    ("shaders.preset.zfast_composite", "zfast CRT composite"),
    ("shaders.preset.zfast_composite.help", "The light zfast CRT with the blur of a composite signal."),
    ("shaders.preset.fakelottes_ntsc", "Fake Lottes NTSC"),
    ("shaders.preset.fakelottes_ntsc.help", "The light Lottes CRT fed by a composite signal."),
    ("shaders.preset.royale_nes_rf", "CRT Royale NES (RF)"),
    ("shaders.preset.royale_nes_rf.help", "The Royale tube fed by the NES's RF output, slot mask: the most faithful, for strong cards."),
    ("shaders.preset.mega_bezel_screen_smooth", "Mega Bezel screen (ScaleFX)"),
    ("shaders.preset.mega_bezel_screen_smooth.help", "The Mega Bezel tube alone, the pixel art first smoothed by ScaleFX."),
    ("shaders.area", "Area"),
    ("shaders.area.auto", "Auto"),
    ("shaders.area.picture", "Picture"),
    ("shaders.area.window", "Whole window"),
    ("shaders.area.help", "Where the preset draws: the picture's rectangle, or the whole window for the presets that draw a TV or a bezel around it (Auto picks the whole window for the Mega Bezel TVs, koko-aio and uborder)."),
    ("shaders.state.compiling", "Compiling…"),
    ("shaders.state.active", "In use"),
    ("shaders.state.refused", "This preset could not be used:"),
    ("shaders.parameters", "Parameters ({0} changed)"),
    ("shaders.parameters.reading", "Reading the parameters…"),
    ("shaders.parameters.none", "This preset has no parameter."),
    ("shaders.parameters.default", "Starting value:"),
    ("shaders.filter", "Filter"),
    ("shaders.reset_all", "Reset all"),
    ("shaders.reset", "Reset"),
];
