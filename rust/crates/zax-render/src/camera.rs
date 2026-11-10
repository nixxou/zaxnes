//! The camera of SPEC-06 §3.2 to §3.7: two modes with their own setpoints, critical smoothing,
//! the corrected aim point of REQ-SCN-008 and the HUD plane of REQ-SCN-045.

use zax_profiles::values::Vec4F;

use crate::geometry::{self, VisibleRect};
use crate::numerics::{Matrix4x4, Vector3};
use crate::smoothing::CriticalSmoothing;

/// Camera mode, with the numeric values Lua sees (SPEC-06 REQ-SCN-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CameraMode {
    /// Orbital view around the NES screen, the default mode.
    #[default]
    Tpv = 0,
    /// View that follows a character.
    Fpv = 1,
}

/// The setpoints of one camera mode: distance, angles in degrees and target point
/// (SPEC-06 REQ-SCN-006).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSetpoint {
    distance: f32,
    /// Setpoint angles `(rx, ry, rz)` in degrees; never bounded (REQ-SCN-011).
    pub angles: Vector3,
    /// Setpoint target `T` in scene coordinates.
    pub target: Vector3,
}

impl Default for CameraSetpoint {
    fn default() -> Self {
        Self {
            distance: 0.0,
            angles: Vector3::ZERO,
            target: geometry::CENTER,
        }
    }
}

impl CameraSetpoint {
    /// Setpoint distance, bounded to [0, 512] (REQ-SCN-011).
    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    /// Sets the setpoint distance, clamped to [0, 512] (REQ-SCN-011).
    pub fn set_distance(&mut self, value: f32) {
        self.distance = value.clamp(0.0, geometry::MAX_DISTANCE);
    }
}

/// The setpoints of one camera mode, as a savestate carries them (REQ-SCN-024).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraModeState {
    /// Setpoint distance.
    pub distance: f32,
    /// Setpoint angles `(rx, ry, rz)` in degrees.
    pub angles: Vector3,
    /// Setpoint target `T` in scene coordinates.
    pub target: Vector3,
}

/// The camera state a savestate holds (SPEC-06 REQ-SCN-024): the current mode, the setpoints of
/// **both** modes and what is left of the mode transition window.
///
/// This is data only: writing it to a savestate file is SPEC-07's business. On restore the
/// setpoints go back as they are and the displayed camera takes them immediately, with its
/// smoothing velocities reset (AMD-045 REQ-SCN-073); the window comes back as a remaining time
/// bounded to `[0, 3]` s, which is how AMD-042 amends REQ-SCN-024 — it spoke of a deadline. It
/// is also the single reference for what a rollback snapshot carries of the camera
/// (REQ-LOOP-069).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraStateData {
    /// The current mode.
    pub mode: CameraMode,
    /// Setpoints of the orbital mode.
    pub tpv: CameraModeState,
    /// Setpoints of the subjective mode.
    pub fpv: CameraModeState,
    /// Seconds left in the 3-second transition window.
    pub transition_remaining: f32,
}

/// Everything a frame needs from the camera: the eye, the scene-frame basis, the aimed point,
/// the view-projection matrix and the HUD plane (SPEC-06 §3.2, §3.3, §3.12).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CameraView {
    /// The eye `E` in scene coordinates.
    pub eye: Vector3,
    /// Camera right axis, scene frame.
    pub right: Vector3,
    /// Camera up axis, scene frame.
    pub up: Vector3,
    /// Camera forward axis, scene frame.
    pub forward: Vector3,
    /// The aimed point `P*` (REQ-SCN-008), or the target in FPV.
    pub aim_point: Vector3,
    /// The effective target `T′` (REQ-SCN-007 step 2).
    pub target: Vector3,
    /// The displayed distance `d`.
    pub distance: f32,
    /// Vertical field of view in degrees.
    pub fov: f32,
    /// Aspect ratio of the render target.
    pub aspect: f32,
    /// View matrix, OpenGL frame, row-vector convention.
    pub view: Matrix4x4,
    /// Projection matrix, row-vector convention.
    pub projection: Matrix4x4,
    /// Distance of the HUD plane, `d_HUD` (REQ-SCN-045).
    pub hud_distance: f32,
}

impl CameraView {
    /// The same view narrowed on `corners` (points of the scene): the projection keeps the
    /// camera and its aspect but shows the smallest centred rectangle of that aspect holding
    /// every corner, as a zoom into the picture (Rust-only, owner request: the 3D scene framed
    /// on the game's picture for a slang preset). A corner behind the eye leaves it whole.
    #[must_use]
    pub fn framed_on(mut self, corners: &[Vector3]) -> Self {
        let m = Matrix4x4::multiply(&self.view, &self.projection);
        let (mut low, mut high) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
        for p in corners {
            let x = p.x * m.m11 + p.y * m.m21 + p.z * m.m31 + m.m41;
            let y = p.x * m.m12 + p.y * m.m22 + p.z * m.m32 + m.m42;
            let w = p.x * m.m14 + p.y * m.m24 + p.z * m.m34 + m.m44;
            if w.is_nan() || w <= 1e-4 {
                return self;
            }
            low = (low.0.min(x / w), low.1.min(y / w));
            high = (high.0.max(x / w), high.1.max(y / w));
        }
        // Half the side of the square in device units, which the aspect makes a rectangle of
        // its own shape on the target.
        let half = ((high.0 - low.0).max(high.1 - low.1) / 2.0).max(1e-4);
        if !half.is_finite() {
            return self;
        }
        let centre = (f32::midpoint(low.0, high.0), f32::midpoint(low.1, high.1));
        let zoom = Matrix4x4::from_rows([
            [1.0 / half, 0.0, 0.0, 0.0],
            [0.0, 1.0 / half, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-centre.0 / half, -centre.1 / half, 0.0, 1.0],
        ]);
        self.projection = Matrix4x4::multiply(&self.projection, &zoom);
        self
    }

    /// The combined view-projection matrix, OpenGL frame.
    #[must_use]
    pub fn view_projection(&self) -> Matrix4x4 {
        Matrix4x4::multiply(&self.view, &self.projection)
    }

    /// The scene point a HUD element placed at `p` in the NES plane occupies:
    /// `E + d_HUD·f + 0.5 × ((p − C) along right, up, forward)` (REQ-SCN-045).
    #[must_use]
    pub fn hud_point(&self, p: Vector3) -> Vector3 {
        let offset = (p - geometry::CENTER) * geometry::HUD_SCALE;
        self.eye
            + (self.forward * self.hud_distance)
            + (self.right * offset.x)
            + (self.up * offset.y)
            + (self.forward * offset.z)
    }

    /// The light propagation direction of the HUD (REQ-SCN-029): `Light.Direction` read in the
    /// camera frame, so that orbiting does not change the HUD's shading.
    #[must_use]
    pub fn hud_light_direction(&self, direction: Vector3) -> Vector3 {
        (self.right * direction.x) + (self.up * direction.y) + (self.forward * direction.z)
    }
}

/// The camera of SPEC-06 §3.2 to §3.7.
///
/// The camera is driven by real time, never by the emulated frame rate (REQ-SCN-059), and keeps
/// being smoothed while the game is paused — only the scene stops being drawn (REQ-SCN-047).
/// FPV target tracking and the occlusion transparency of REQ-SCN-022 belong to the Lua layer
/// and the host; this type only holds the state and the geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneCamera {
    setpoints: [CameraSetpoint; 2],
    mode: CameraMode,
    fov: f32,
    distance: f32,
    angles: Vector3,
    target: Vector3,
    transition_remaining: f32,
    /// Characteristic smoothing time `Tₛ`, 0.4 s by default (REQ-SCN-019).
    pub smooth_time: f32,
    /// The rotation a script forces, or none (AMD-063 REQ-LUA-119). A session value: no
    /// savestate carries it, the rollback snapshots do.
    rot_override: Option<Vector3>,
    /// The local camera profile setting (REQ-SCN-009): the rotation becomes `R_Z(ry) · R_X(rx)`
    /// and the camera no longer aims at `P*`.
    pub local_camera: bool,
    distance_filter: CriticalSmoothing,
    rx_filter: CriticalSmoothing,
    ry_filter: CriticalSmoothing,
    rz_filter: CriticalSmoothing,
    target_x_filter: CriticalSmoothing,
    target_y_filter: CriticalSmoothing,
    target_z_filter: CriticalSmoothing,
}

impl Default for SceneCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneCamera {
    /// Creates the camera in its launch state (REQ-SCN-012, REQ-SCN-013). At application launch
    /// the FPV distance is 0 (REQ-SCN-012).
    #[must_use]
    pub fn new() -> Self {
        let mut camera = Self {
            setpoints: [CameraSetpoint::default(); 2],
            mode: CameraMode::Tpv,
            fov: geometry::DEFAULT_FOV,
            distance: 0.0,
            angles: Vector3::ZERO,
            target: geometry::CENTER,
            transition_remaining: 0.0,
            smooth_time: CriticalSmoothing::DEFAULT_SMOOTH_TIME,
            local_camera: false,
            distance_filter: CriticalSmoothing::default(),
            rx_filter: CriticalSmoothing::default(),
            ry_filter: CriticalSmoothing::default(),
            rz_filter: CriticalSmoothing::default(),
            target_x_filter: CriticalSmoothing::default(),
            target_y_filter: CriticalSmoothing::default(),
            target_z_filter: CriticalSmoothing::default(),
            rot_override: None,
        };
        camera.reset_view();
        camera.snap_to_setpoints();
        camera
    }

    /// Current mode (REQ-SCN-006).
    #[must_use]
    pub const fn mode(&self) -> CameraMode {
        self.mode
    }

    /// The setpoints of the current mode.
    #[must_use]
    pub const fn current(&self) -> &CameraSetpoint {
        &self.setpoints[self.mode as usize]
    }

    /// The setpoints of the current mode, for writing.
    pub fn current_mut(&mut self) -> &mut CameraSetpoint {
        &mut self.setpoints[self.mode as usize]
    }

    /// The setpoints of `mode`.
    #[must_use]
    pub const fn setpoints(&self, mode: CameraMode) -> &CameraSetpoint {
        &self.setpoints[mode as usize]
    }

    /// The setpoints of `mode`, for writing.
    pub fn setpoints_mut(&mut self, mode: CameraMode) -> &mut CameraSetpoint {
        &mut self.setpoints[mode as usize]
    }

    /// Field of view in degrees, a user setting of 45 to 95 by steps of 5 (REQ-SCN-005).
    #[must_use]
    pub const fn fov(&self) -> f32 {
        self.fov
    }

    /// Sets the field of view, clamped to 45..95. Any change applies at once and triggers a
    /// view reset (REQ-SCN-012).
    pub fn set_fov(&mut self, value: f32) {
        let clamped = value.clamp(geometry::MIN_FOV, geometry::MAX_FOV);
        if clamped.to_bits() == self.fov.to_bits() {
            return;
        }

        self.fov = clamped;
        self.reset_view();
    }

    /// Seconds left in the mode transition window (REQ-SCN-006, REQ-SCN-020).
    #[must_use]
    pub const fn transition_remaining(&self) -> f32 {
        self.transition_remaining
    }

    /// The rotation a script forces on the camera, in degrees, or none for the normal behaviour
    /// (AMD-063 REQ-LUA-119, `CS.Nes3D.CameraControl.RotOverride`).
    #[must_use]
    pub const fn rot_override(&self) -> Option<Vector3> {
        self.rot_override
    }

    /// Forces the rotation of the current mode, or gives the camera its normal behaviour back.
    ///
    /// While it is set, every camera update writes it into the rotation setpoint of the
    /// **current** mode and shows it at once, without the smoothing of REQ-SCN-019. Put back to
    /// none, the camera carries on from the setpoint it was left at, which is the last forced
    /// value.
    ///
    /// It is a session value: it is not persisted, it is **not** part of the camera state a
    /// savestate carries (REQ-SCN-024 unchanged, so neither [`Self::capture`] nor
    /// [`Self::restore`] touches it), and the host puts it back to none when a game is launched
    /// and when the profile changes. A rollback snapshot does carry it (AMD-042 REQ-LOOP-069 and
    /// REQ-LOOP-070 as AMD-063 amends them), which is why the host reads and writes it around
    /// the capture and the restore rather than through them.
    pub const fn set_rot_override(&mut self, angles: Option<Vector3>) {
        self.rot_override = angles;
    }

    /// Displayed distance, after smoothing.
    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    /// Displayed angles in degrees, after smoothing.
    #[must_use]
    pub const fn angles(&self) -> Vector3 {
        self.angles
    }

    /// Displayed target, after smoothing.
    #[must_use]
    pub const fn target(&self) -> Vector3 {
        self.target
    }

    /// The view reset of REQ-SCN-012: the current FOV is applied, both modes get the angles
    /// `(0.81°, 0.81°, 0)` and the target `C`, and the **current** mode gets
    /// `d₀ = 132.7964 / tan(FOV / 2) × s` (AMD-045 REQ-SCN-069, which replaces the
    /// `104 / atan(FOV_rad / 2)` of REQ-SCN-012). The other mode keeps its distance.
    ///
    /// The distance goes through the setpoint, so it is bounded to `[0, 512]` as REQ-SCN-069
    /// asks (REQ-SCN-011). The unbounded value is [`SceneCamera::reset_distance`], what
    /// `Camera.ResetDistance` reads (AMD-040 REQ-LUA-113).
    pub fn reset_view(&mut self) {
        for setpoint in &mut self.setpoints {
            setpoint.angles = geometry::DEFAULT_ANGLES;
            setpoint.target = geometry::CENTER;
        }

        let distance = self.reset_distance();
        self.current_mut().set_distance(distance);
    }

    /// The framed distance `d₀` a view reset would write at the current field of view,
    /// **before** the bound of REQ-SCN-011 (AMD-045 REQ-SCN-069). It follows every change of
    /// field of view, even without a reset, which is what `Camera.ResetDistance` exposes to Lua
    /// (AMD-040 REQ-LUA-113).
    #[must_use]
    pub fn reset_distance(&self) -> f32 {
        geometry::reset_distance(self.fov)
    }

    /// The head-on view of the 2D picture (Rust-only, owner request: the "2D from the front" of
    /// the handheld): straight ahead, the rows of the picture that `clipping` leaves visible
    /// (REQ-SCN-036) filling the height of a view of `aspect`. `clipping` is the game's, without
    /// the widescreen margins, which stay out of sight above and below. It ignores the setpoints,
    /// which a script may move, and leaves them alone for the return to 3D. `tilt` (degrees, as
    /// the angles) turns it about the picture's centre: the pull of the head-on picture.
    #[must_use]
    pub fn picture_view(&self, clipping: Vec4F, aspect: f32, tilt: Vector3) -> CameraView {
        let mut flat = self.clone();
        // The subjective view looks exactly along its axis, with no aimed point (REQ-SCN-010).
        flat.mode = CameraMode::Fpv;
        flat.local_camera = false;
        flat.angles = tilt;
        (flat.target, flat.distance) = self.picture_framing(clipping);
        flat.build_view(Vec4F::new(0.0, 0.0, 0.0, 0.0), aspect)
    }

    /// The target and the distance of the head-on view of the 2D picture (see
    /// [`Self::picture_view`]).
    fn picture_framing(&self, clipping: Vec4F) -> (Vector3, f32) {
        // NES row 0 lies at y = 247..248 and row 239 at 8..9 (REQ-E3D-410); the visible
        // rectangle cuts the picture in the same scene coordinates.
        let rect = VisibleRect::from_clipping(clipping);
        let top = rect.max_y.min(geometry::SCENE_HEIGHT);
        let bottom = rect
            .min_y
            .max(geometry::SCENE_HEIGHT - geometry::FRAME_HEIGHT);
        let target = Vector3::new(rect.center().x, f32::midpoint(top, bottom), 0.0);
        let half_angle = self.fov * std::f32::consts::PI / 180.0 / 2.0;
        (target, (top - bottom).max(1.0) / 2.0 / half_angle.tan())
    }

    /// The head-on picture gives way to the camera (Rust-only, owner request): the camera is
    /// put at once where [`Self::picture_view`] looked from, tilted by `tilt`, and heads for the
    /// angles `angles` through the smoothing.
    pub fn leave_picture_view(&mut self, clipping: Vec4F, tilt: Vector3, angles: Vector3) {
        let (target, distance) = self.picture_framing(clipping);
        let setpoint = self.current_mut();
        setpoint.target = target;
        setpoint.set_distance(distance);
        setpoint.angles = tilt;
        self.snap_to_setpoints();
        self.current_mut().angles = angles;
    }

    /// Ends a game (REQ-SCN-013): back to TPV, smoothing time back to 0.4 s, then a view reset.
    /// The speed setting is the host's business (SPEC-07).
    pub fn end_game(&mut self) {
        // AMD-063 REQ-LUA-119: a session value, back to none when the game stops.
        self.rot_override = None;
        self.mode = CameraMode::Tpv;
        self.transition_remaining = 0.0;
        self.smooth_time = CriticalSmoothing::DEFAULT_SMOOTH_TIME;
        self.reset_view();
    }

    /// Switches mode, which opens the 3-second transition window (REQ-SCN-006). Setting the
    /// same mode has no effect.
    pub fn set_mode(&mut self, mode: CameraMode) {
        if mode == self.mode {
            return;
        }

        self.mode = mode;
        self.transition_remaining = geometry::MODE_TRANSITION_SECONDS;
    }

    /// Switches to the other mode (REQ-SCN-021, `Camera:ToggleMode()`).
    pub fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            CameraMode::Tpv => CameraMode::Fpv,
            CameraMode::Fpv => CameraMode::Tpv,
        });
    }

    /// The camera state a savestate carries (REQ-SCN-024): the current mode, the setpoints of
    /// both modes and what is left of the transition window.
    #[must_use]
    pub fn capture(&self) -> CameraStateData {
        CameraStateData {
            mode: self.mode,
            tpv: self.snapshot(CameraMode::Tpv),
            fpv: self.snapshot(CameraMode::Fpv),
            transition_remaining: self.transition_remaining,
        }
    }

    /// Puts back a state [`capture`](Self::capture) produced (REQ-SCN-024). The displayed values
    /// are snapped onto the restored setpoints at once and the smoothing velocities go back to
    /// zero, so the first picture after the load already shows the saved view (AMD-045
    /// REQ-SCN-073, which replaces « the displayed camera joins them through the smoothing »).
    /// What is left of the transition window is restored as a remaining time bounded to
    /// `[0, 3]` s (AMD-042, which amends REQ-SCN-024 — it spoke of a deadline).
    ///
    /// This is the single entry point for every reload of the camera — a savestate, a quick save
    /// and a rollback snapshot (AMD-042 REQ-LOOP-070) — so they all snap the same way. A file
    /// written by another build, or one predating the amendment, can therefore never leave the
    /// window stuck open either.
    pub fn restore(&mut self, state: &CameraStateData) {
        self.apply(CameraMode::Tpv, &state.tpv);
        self.apply(CameraMode::Fpv, &state.fpv);
        self.mode = state.mode;
        self.transition_remaining = state
            .transition_remaining
            .clamp(0.0, geometry::MODE_TRANSITION_SECONDS);
        self.snap_to_setpoints();
    }

    /// Puts the displayed values on the setpoints at once, without smoothing.
    pub fn snap_to_setpoints(&mut self) {
        let current = *self.current();
        self.distance = current.distance;
        self.angles = current.angles;
        self.target = current.target;
        for filter in [
            &mut self.distance_filter,
            &mut self.rx_filter,
            &mut self.ry_filter,
            &mut self.rz_filter,
            &mut self.target_x_filter,
            &mut self.target_y_filter,
            &mut self.target_z_filter,
        ] {
            filter.reset();
        }
    }

    /// Advances the smoothing by `real_seconds` of real time (REQ-SCN-019, REQ-SCN-059). In TPV
    /// the target is always smoothed; in FPV it is smoothed only during the transition window,
    /// and followed instantly afterwards (REQ-SCN-020).
    pub fn advance(&mut self, real_seconds: f32) {
        // AMD-063 REQ-LUA-119, before anything else and whatever the elapsed time: the forced
        // rotation becomes the setpoint of the current mode and the displayed rotation at once,
        // the three smoothing filters starting again from there. It is written before the wrap of
        // AMD-045 REQ-SCN-071, so a forced angle outside ]-180, 180] is brought back by whole
        // turns like any other setpoint and the orientation shown is the one asked for.
        if let Some(forced) = self.rot_override {
            self.current_mut().angles = forced;
            self.angles = forced;
            self.rx_filter.reset();
            self.ry_filter.reset();
            self.rz_filter.reset();
        }

        self.wrap_setpoint_angles();
        if real_seconds <= 0.0 {
            return;
        }

        if self.transition_remaining > 0.0 {
            self.transition_remaining = (self.transition_remaining - real_seconds).max(0.0);
        }

        let setpoint = *self.current();
        let time = self.smooth_time;
        self.distance =
            self.distance_filter
                .step(self.distance, setpoint.distance, time, real_seconds);
        self.angles = Vector3::new(
            self.rx_filter
                .step(self.angles.x, setpoint.angles.x, time, real_seconds),
            self.ry_filter
                .step(self.angles.y, setpoint.angles.y, time, real_seconds),
            self.rz_filter
                .step(self.angles.z, setpoint.angles.z, time, real_seconds),
        );

        let smooth_target = self.mode == CameraMode::Tpv || self.transition_remaining > 0.0;
        if smooth_target {
            self.target = Vector3::new(
                self.target_x_filter
                    .step(self.target.x, setpoint.target.x, time, real_seconds),
                self.target_y_filter
                    .step(self.target.y, setpoint.target.y, time, real_seconds),
                self.target_z_filter
                    .step(self.target.z, setpoint.target.z, time, real_seconds),
            );
        } else {
            self.target = setpoint.target;
            self.target_x_filter.reset();
            self.target_y_filter.reset();
            self.target_z_filter.reset();
        }
    }

    /// Builds the view of the frame from the smoothed values, the clip margins and the aspect
    /// ratio (width divided by height) of the render target (REQ-SCN-004, REQ-SCN-007 to
    /// REQ-SCN-010).
    #[must_use]
    pub fn build_view(&self, clipping: Vec4F, aspect: f32) -> CameraView {
        let rect = VisibleRect::from_clipping(clipping);
        let q = geometry::rotation(self.angles, self.local_camera);

        // REQ-SCN-007 step 2: TPV recentres the target on the visible rectangle, FPV does not
        // (REQ-SCN-010).
        let target = if self.mode == CameraMode::Tpv {
            geometry::effective_target(self.target, clipping)
        } else {
            self.target
        };
        let eye = target - (q.forward * self.distance);

        let (right, up, forward, aim) = if self.mode == CameraMode::Tpv && !self.local_camera {
            // REQ-SCN-008: the camera is oriented towards P*. The vertical is the camera's own,
            // q·(0, 1, 0), not the world's Y axis (AMD-045 REQ-SCN-072, which replaces step 4 of
            // REQ-SCN-007): rz therefore rolls the picture in TPV as it does in FPV, and the
            // basis stays continuous when the camera looks straight down.
            let aim = geometry::aim_point(eye, rect, target);
            let towards = aim - eye;
            let forward = if towards.length_squared() > 0.0 {
                towards.normalize()
            } else {
                q.forward
            };
            let (right, up) = basis_from_forward(forward, q.up);
            (right, up, forward, aim)
        } else {
            // REQ-SCN-009 and REQ-SCN-010: the camera looks exactly along q·(0,0,1), with the
            // vertical q·(0,1,0). In FPV without the local camera that includes the roll rz.
            (q.right, q.up, q.forward, eye + q.forward)
        };

        CameraView {
            eye,
            right,
            up,
            forward,
            aim_point: aim,
            target,
            distance: self.distance,
            fov: self.fov,
            aspect,
            view: look_at(eye, forward, up),
            projection: Matrix4x4::create_perspective_field_of_view(
                self.fov * std::f32::consts::PI / 180.0,
                aspect.max(1e-3),
                geometry::NEAR_PLANE,
                geometry::FAR_PLANE,
            ),
            hud_distance: geometry::hud_distance(self.fov),
        }
    }

    /// Brings the three setpoint angles of **each** mode back into `]−180°, 180°]` by whole
    /// turns, and shifts the displayed angles of the current mode by the very same turns
    /// (AMD-045 REQ-SCN-071, which replaces the second sentence of REQ-SCN-011 and the
    /// "without folding modulo 360°" of REQ-SCN-019).
    ///
    /// The orientation is therefore unchanged, and the smoothing velocities are left alone: the
    /// gap between a displayed angle and its setpoint, which is what the filter works on, is the
    /// same number before and after. The user can still orbit without limit.
    fn wrap_setpoint_angles(&mut self) {
        for mode in 0..self.setpoints.len() {
            let angles = self.setpoints[mode].angles;
            let (rx, shift_x) = geometry::wrap_angle(angles.x);
            let (ry, shift_y) = geometry::wrap_angle(angles.y);
            let (rz, shift_z) = geometry::wrap_angle(angles.z);
            if shift_x == 0.0 && shift_y == 0.0 && shift_z == 0.0 {
                continue;
            }

            self.setpoints[mode].angles = Vector3::new(rx, ry, rz);

            // The displayed angle only exists for the current mode (REQ-SCN-019).
            if mode == self.mode as usize {
                self.angles += Vector3::new(shift_x, shift_y, shift_z);
            }
        }
    }

    fn snapshot(&self, mode: CameraMode) -> CameraModeState {
        let setpoint = self.setpoints(mode);
        CameraModeState {
            distance: setpoint.distance,
            angles: setpoint.angles,
            target: setpoint.target,
        }
    }

    fn apply(&mut self, mode: CameraMode, state: &CameraModeState) {
        let setpoint = self.setpoints_mut(mode);
        setpoint.set_distance(state.distance);
        setpoint.angles = state.angles;
        setpoint.target = state.target;
    }
}

/// The camera basis that completes `forward` with `up` as the vertical, in the left-handed scene
/// frame. In TPV the vertical handed in is the camera's own, `q·(0, 1, 0)` (AMD-045
/// REQ-SCN-072), which replaces the world Y axis of REQ-SCN-007 step 4.
///
/// `forward` is the aimed direction, normalised; `up` need not be orthogonal to it.
#[must_use]
pub fn basis_from_forward(forward: Vector3, up: Vector3) -> (Vector3, Vector3) {
    let mut right = Vector3::cross(up, forward);
    if right.length_squared() < 1e-8 {
        // The vertical and the direction are parallel: any axis across them will do.
        right = Vector3::cross(Vector3::UNIT_Z, forward);
        if right.length_squared() < 1e-8 {
            right = Vector3::UNIT_X;
        }
    }

    let right = right.normalize();
    (right, Vector3::cross(forward, right))
}

/// The OpenGL view matrix of a scene-frame camera. The scene frame is left-handed, so the
/// conversion `(x, y, z) → (x, y, −z)` of REQ-SCN-001 is applied to the eye and to the basis
/// before the right-handed look-at is built.
#[must_use]
pub fn look_at(eye: Vector3, forward: Vector3, up: Vector3) -> Matrix4x4 {
    let eye_gl = to_gl(eye);
    let mut forward_gl = to_gl(forward);
    let mut up_gl = to_gl(up);
    if forward_gl.length_squared() < 1e-8 {
        forward_gl = -Vector3::UNIT_Z;
    }

    if up_gl.length_squared() < 1e-8 {
        up_gl = Vector3::UNIT_Y;
    }

    Matrix4x4::create_look_at(eye_gl, eye_gl + forward_gl, up_gl)
}

/// Converts a scene-frame point or vector to the OpenGL frame (REQ-SCN-001).
#[must_use]
pub const fn to_gl(scene: Vector3) -> Vector3 {
    Vector3::new(scene.x, scene.y, -scene.z)
}
