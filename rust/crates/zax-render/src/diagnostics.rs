//! The bounded diagnostics of AMD-051: a renderer that degrades instead of failing still has to
//! say so, and not drown the log doing it (REQ-E3D-644, REQ-E3D-646).
//!
//! The renderer writes no file and knows no logger (REQ-SCN-057), so the lines are collected
//! here and handed to the host, which logs them. Both requirements use the same rule:
//!
//! - at most one line per subject (a pattern for REQ-E3D-644, a 3D object for REQ-E3D-646);
//! - at most [`LINE_CAP`] lines per game;
//! - beyond that, one summary line per [`SUMMARY_FRAMES`] rendered frames, giving the running
//!   total of subjects concerned.
//!
//! **Nothing here runs while everything works.** A frame on which no texture is refused and no
//! object is late touches neither set and allocates nothing: [`RenderDiagnostics::take`] hands
//! back a `Vec::new()`, which does not allocate.

use std::fmt::Write as _;

use zax_engine3d::hashing::FastSet;
use zax_engine3d::registry::PatternId;

/// Lines one game may log before the summary form takes over (REQ-E3D-644: "au plus 100 fois par
/// partie"). Patterns are also created *during* play (REQ-E3D-008, REQ-E3D-453) and nothing
/// bounds how many, so the cap is what keeps a pathological ROM from being its own degradation.
pub const LINE_CAP: usize = 100;

/// Rendered frames between two summary lines: one minute of play at the nominal 60 frames per
/// second of SPEC-07 §2 ("au plus une par minute de jeu").
pub const SUMMARY_FRAMES: u64 = 3600;

/// One subject the diagnostics count, with its own set and its own total.
#[derive(Debug)]
struct Subject<K> {
    reported: FastSet<K>,
    /// Subjects seen since the last reset, reported or not.
    total: u64,
}

// Derived `Default` would ask `K: Default`, which an identifier has no reason to be.
impl<K> Default for Subject<K> {
    fn default() -> Self {
        Self {
            reported: FastSet::default(),
            total: 0,
        }
    }
}

/// The diagnostics of one renderer, reset when a profile is unloaded.
#[derive(Debug, Default)]
pub struct RenderDiagnostics {
    textures: Subject<PatternId>,
    late_objects: Subject<(PatternId, usize)>,
    /// Whether the one line of AMD-055 REQ-SCN-080 has been written (see
    /// [`Self::screen_effects_unavailable`]). One subject, so one boolean rather than a set.
    screen_effects_reported: bool,
    /// Whether the one line of REQ-SCN-083's fallback **(D)** has been written (see
    /// [`Self::msvo_unavailable`]). One subject, so one boolean rather than a set.
    msvo_reported: bool,
    /// Lines handed out since the last reset, against [`LINE_CAP`].
    lines_emitted: usize,
    /// The lines waiting for the host.
    pending: Vec<String>,
    /// The rendered frame the last summary was written on.
    last_summary: u64,
}

impl RenderDiagnostics {
    /// Reports that no texture could be allocated for an object of `pattern` on frame `frame`,
    /// so its tiles are not drawn (REQ-E3D-644).
    pub fn texture_unavailable(&mut self, pattern: PatternId, frame: u64) {
        if self.textures.reported.insert(pattern) {
            self.textures.total += 1;
            self.push(
                frame,
                format!(
                    "the texture of a 3D object of pattern {} could not be allocated; \
                     its tiles are not drawn until it is available (REQ-E3D-644)",
                    pattern.slot()
                ),
            );
        }
    }

    /// Reports that the 3D object `index` of `pattern` has been waiting for its meshes for the
    /// whole window of REQ-E3D-646.
    pub fn object_late(&mut self, pattern: PatternId, index: usize, frames: u32, frame: u64) {
        if self.late_objects.reported.insert((pattern, index)) {
            self.late_objects.total += 1;
            self.push(
                frame,
                format!(
                    "3D object {index} of pattern {} has not been drawable for {frames} \
                     emulated frames (REQ-E3D-646)",
                    pattern.slot()
                ),
            );
        }
    }

    /// Reports that the screen-space effects of the Enhanced style cannot be prepared on this
    /// implementation, so the frame is drawn without them (AMD-055 REQ-SCN-080, and point 3 of the
    /// RU-5 brief: "a fallback that is cheaper, never more expensive").
    ///
    /// **Once per game**, like the two subjects above, because the renderer gives the effects up
    /// for the session rather than trying them again on every frame: the two formats they need are
    /// the two the renderer already depends on (`R8` and `RGBA16F`, see `gl::screen_effects`), so
    /// an implementation that refuses them will refuse them for ever.
    pub fn screen_effects_unavailable(&mut self, reason: &str, frame: u64) {
        if self.screen_effects_reported {
            return;
        }
        self.screen_effects_reported = true;
        self.push(
            frame,
            format!(
                "the screen-space effects of the Enhanced style could not be prepared \
                 ({reason}); the scene is drawn without them (AMD-055 REQ-SCN-080)"
            ),
        );
    }

    /// Reports that the multi-scale estimator of AMD-055 REQ-SCN-083's *Mode MSVO* cannot run on
    /// this implementation, so `spl.ao` = 1 is rendered as `spl.ao` = 2 — the declared fallback
    /// **(D)** of that requirement, the label `MSVO` staying exposed and read back.
    ///
    /// **Once per game**, and for the same reason as [`Self::screen_effects_unavailable`]: the one
    /// thing the mode needs that the rest of the renderer does not is an `R16F` colour target, and
    /// an implementation that refuses it will refuse it for ever.
    pub fn msvo_unavailable(&mut self, reason: &str, frame: u64) {
        if self.msvo_reported {
            return;
        }
        self.msvo_reported = true;
        self.push(
            frame,
            format!(
                "the multi-scale ambient occlusion (MSVO) could not be prepared ({reason}); \
                 the scene is drawn with the Depth occlusion instead (AMD-055 REQ-SCN-083)"
            ),
        );
    }

    /// The lines the host should log, and nothing on a frame where nothing went wrong.
    pub fn take(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending)
    }

    /// Whether anything has ever been reported since the last [`Self::reset`] — the one test the
    /// host makes per frame, so that a healthy frame costs a single comparison.
    #[must_use]
    pub fn is_quiet(&self) -> bool {
        self.pending.is_empty()
    }

    /// Distinct patterns whose texture could not be allocated (REQ-E3D-644's running total).
    #[must_use]
    pub const fn patterns_without_texture(&self) -> u64 {
        self.textures.total
    }

    /// Distinct 3D objects that outstayed the window of REQ-E3D-646.
    #[must_use]
    pub const fn late_objects(&self) -> u64 {
        self.late_objects.total
    }

    /// Forgets everything: a new game starts with its own budget of lines (REQ-E3D-644 counts
    /// "par partie").
    pub fn reset(&mut self) {
        self.textures.reported.clear();
        self.textures.total = 0;
        self.late_objects.reported.clear();
        self.late_objects.total = 0;
        self.screen_effects_reported = false;
        self.msvo_reported = false;
        self.lines_emitted = 0;
        self.pending.clear();
        self.last_summary = 0;
    }

    /// Emits `line` while the cap allows it, and the periodic summary once it does not.
    fn push(&mut self, frame: u64, line: String) {
        if self.lines_emitted < LINE_CAP {
            self.lines_emitted += 1;
            self.pending.push(line);
            return;
        }

        if frame.saturating_sub(self.last_summary) < SUMMARY_FRAMES {
            return;
        }
        self.last_summary = frame;
        let mut summary = String::new();
        // `write!` into a `String` cannot fail.
        let _ = write!(
            summary,
            "degraded rendering so far: {} pattern(s) without a texture",
            self.textures.total
        );
        if self.late_objects.total > 0 {
            let _ = write!(
                summary,
                ", {} 3D object(s) late (REQ-E3D-644, REQ-E3D-646)",
                self.late_objects.total
            );
        } else {
            summary.push_str(" (REQ-E3D-644)");
        }
        self.pending.push(summary);
    }
}
