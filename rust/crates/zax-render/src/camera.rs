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
/// **both** modes and the deadline of the mode transition window.
///
/// This is data only: writing it to a savestate file is SPEC-07's business. On restore the
/// setpoints go back as they are and the displayed camera joins them through the smoothing of
/// REQ-SCN-019 — it is not snapped.
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
    /// `d₀ = 104 / atan(FOV_rad / 2)`. The other mode keeps its distance.
    pub fn reset_view(&mut self) {
        for setpoint in &mut self.setpoints {
            setpoint.angles = geometry::DEFAULT_ANGLES;
            setpoint.target = geometry::CENTER;
        }

        let distance = geometry::default_distance(self.fov);
        self.current_mut().set_distance(distance);
    }

    /// The head-on view of the 2D picture (Rust-only, owner request: the "2D from the front" of
    /// the handheld): straight ahead, the rows of the picture that `clipping` leaves visible
    /// (REQ-SCN-036) filling the height of a view of `aspect`. `clipping` is the game's, without
    /// the widescreen margins, which stay out of sight above and below. It ignores the setpoints,
    /// which a script may move, and leaves them alone for the return to 3D.
    #[must_use]
    pub fn picture_view(&self, clipping: Vec4F, aspect: f32) -> CameraView {
        let mut flat = self.clone();
        // The subjective view looks exactly along its axis, with no aimed point (REQ-SCN-010).
        flat.mode = CameraMode::Fpv;
        flat.local_camera = false;
        flat.angles = Vector3::ZERO;
        // NES row 0 lies at y = 247..248 and row 239 at 8..9 (REQ-E3D-410); the visible
        // rectangle cuts the picture in the same scene coordinates.
        let rect = VisibleRect::from_clipping(clipping);
        let top = rect.max_y.min(geometry::SCENE_HEIGHT);
        let bottom = rect
            .min_y
            .max(geometry::SCENE_HEIGHT - geometry::FRAME_HEIGHT);
        flat.target = Vector3::new(rect.center().x, f32::midpoint(top, bottom), 0.0);
        let half_angle = self.fov * std::f32::consts::PI / 180.0 / 2.0;
        flat.distance = (top - bottom).max(1.0) / 2.0 / half_angle.tan();
        flat.build_view(Vec4F::new(0.0, 0.0, 0.0, 0.0), aspect)
    }

    /// Ends a game (REQ-SCN-013): back to TPV, smoothing time back to 0.4 s, then a view reset.
    /// The speed setting is the host's business (SPEC-07).
    pub fn end_game(&mut self) {
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
    /// are **not** snapped: they join the restored setpoints through the smoothing of
    /// REQ-SCN-019.
    pub fn restore(&mut self, state: &CameraStateData) {
        self.apply(CameraMode::Tpv, &state.tpv);
        self.apply(CameraMode::Fpv, &state.fpv);
        self.mode = state.mode;
        self.transition_remaining = state.transition_remaining.max(0.0);
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
            // REQ-SCN-008: the camera is oriented towards P*, with world Y as the vertical.
            let aim = geometry::aim_point(eye, rect, target);
            let towards = aim - eye;
            let forward = if towards.length_squared() > 0.0 {
                towards.normalize()
            } else {
                q.forward
            };
            let (right, up) = basis_from_forward(forward);
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

/// The camera basis that completes `forward` with world Y as the vertical, in the left-handed
/// scene frame (REQ-SCN-007 step 4: no roll).
#[must_use]
pub fn basis_from_forward(forward: Vector3) -> (Vector3, Vector3) {
    let mut right = Vector3::cross(Vector3::UNIT_Y, forward);
    if right.length_squared() < 1e-8 {
        // Looking straight up or down: any horizontal axis will do.
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
