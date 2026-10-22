using System.Numerics;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.Nes;
using Zax.Profiles;
using Zax.Engine3D;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// The mouse Zapper of SPEC-08 §3.7 and its test vectors of §5.3: the ray cast onto the aim
/// plane, the NES pixel it reads back, the trigger polarity and the port-type reset
/// (REQ-INP-060 to REQ-INP-066, DEC-08-4, DEC-02-Q1).
/// </summary>
public sealed class ZapperInputTests
{
    /// <summary>The render target the vectors are computed in; the aspect ratio is what matters.</summary>
    private static readonly Vector2 Viewport = new(1280f, 720f);

    /// <summary>
    /// REQ-INP-063 and §5.3: with the default camera, no offset and scale 1, pointing at a scene
    /// point of the aim plane gives the NES pixel displayed there — <c>C = (128, 120)</c> reads
    /// (128, 128) and the centre of the NES image (128, 128) reads (128, 120).
    /// </summary>
    [Theory]
    // Scene point x, scene point y, expected px, expected py (SPEC-08 §5.3).
    [InlineData(128f, 120f, 128, 128)]
    [InlineData(128f, 128f, 128, 120)]
    [InlineData(0f, 248f, 0, 0)]
    [InlineData(0f, 8f, 0, 240)]
    [InlineData(256f, 248f, 256, 0)]
    public void REQ_INP_063_the_aim_is_the_nes_pixel_under_the_pointer(
        float sceneX,
        float sceneY,
        int expectedX,
        int expectedY)
    {
        CameraView view = DefaultView();
        Vector2 pointer = Project(view, new Vector3(sceneX, sceneY, 0f));

        Assert.True(ZapperAim.TryAim(
            view,
            pointer,
            Viewport,
            Vector3.Zero,
            sceneScale: 1f,
            out ZapperPosition position));

        Assert.InRange(position.X, expectedX - 1, expectedX + 1);
        Assert.InRange(position.Y, expectedY - 1, expectedY + 1);
    }

    /// <summary>
    /// SPEC-08 §5.3, fourth row: with the clip margins (0 ; 7.9 ; 0 ; 0.1), the top-left corner
    /// of the visible rectangle is the scene point (0 ; 239.9) and reads (0, 8).
    /// </summary>
    [Fact]
    public void Section_5_3_the_top_left_corner_of_the_visible_rectangle()
    {
        Vec4F clipping = new(0f, 7.9f, 0f, 0.1f);
        CameraView view = DefaultView(clipping);
        Vector2 pointer = Project(view, new Vector3(0f, 239.9f, 0f));

        Assert.True(ZapperAim.TryAim(
            view,
            pointer,
            Viewport,
            Vector3.Zero,
            sceneScale: 1f,
            out ZapperPosition position));

        Assert.InRange(position.X, -1, 1);
        Assert.InRange(position.Y, 7, 9);
    }

    /// <summary>
    /// SPEC-08 §5.3, last row: <c>ZapperOffset = (10, 0, 0)</c> aimed at <c>C</c> reads
    /// (118, 128) — the offset is taken out of the scene point before the conversion.
    /// </summary>
    [Fact]
    public void REQ_INP_063_the_zapper_offset_shifts_the_aimed_pixel()
    {
        CameraView view = DefaultView();
        Vector3 offset = new(10f, 0f, 0f);
        Vector2 pointer = Project(view, new Vector3(128f, 120f, 0f));

        Assert.True(ZapperAim.TryAim(view, pointer, Viewport, offset, 1f, out ZapperPosition position));
        Assert.InRange(position.X, 117, 119);
        Assert.InRange(position.Y, 127, 129);

        // REQ-INP-063, notes: a positive ZapperOffset.y means the image moved up, so the same
        // screen point aims at a NES pixel further down.
        Vector2 centre = Project(view, new Vector3(128f, 128f, 0f));
        Assert.True(ZapperAim.TryAim(
            view,
            centre,
            Viewport,
            new Vector3(0f, 10f, 0f),
            1f,
            out ZapperPosition shifted));

        Assert.InRange(shifted.Y, 129, 131);
    }

    /// <summary>
    /// REQ-INP-063 and DEC-08-4 (divergence D): the aim takes the scene scale into account, so
    /// the pixel under the pointer is correct at any scale.
    /// </summary>
    [Fact]
    public void DEC_08_4_the_aim_accounts_for_the_scene_scale()
    {
        // At scale 2, the scene point (256, 240) is the pixel (128, 128): the division by e
        // happens before the flip constant is applied.
        ZapperPosition scaled = ZapperAim.ToNesPixel(
            new Vector2(256f, 240f),
            Vector3.Zero,
            sceneScale: 2f);

        Assert.Equal(128, scaled.X);
        Assert.Equal(128, scaled.Y);

        // At scale 1 the same point reads (256, 8).
        ZapperPosition unscaled = ZapperAim.ToNesPixel(new Vector2(256f, 240f), Vector3.Zero, 1f);
        Assert.Equal(256, unscaled.X);
        Assert.Equal(8, unscaled.Y);
    }

    /// <summary>
    /// REQ-INP-063, last criterion: with the camera turned by 30°, the transmitted position is
    /// still the one the relation gives for the scene point under the pointer (±1).
    /// </summary>
    [Fact]
    public void REQ_INP_063_a_rotated_camera_still_aims_at_the_scene_point()
    {
        SceneCamera camera = new();
        camera.ResetView();
        camera.Current.Angles = new Vector3(0f, 30f, 0f);
        camera.SnapToSetpoints();

        CameraView view = camera.BuildView(ConversionSettings.DefaultClipping, Viewport.X / Viewport.Y);

        foreach (Vector3 point in (Vector3[])
            [
                new(128f, 120f, 0f),
                new(128f, 128f, 0f),
                new(64f, 180f, 0f),
                new(200f, 40f, 0f),
            ])
        {
            Vector2 pointer = Project(view, point);
            Assert.True(ZapperAim.TryAim(view, pointer, Viewport, Vector3.Zero, 1f, out ZapperPosition position));

            ZapperPosition expected = ZapperAim.ToNesPixel(new Vector2(point.X, point.Y), Vector3.Zero, 1f);
            Assert.InRange(position.X, expected.X - 1, expected.X + 1);
            Assert.InRange(position.Y, expected.Y - 1, expected.Y + 1);
        }
    }

    /// <summary>
    /// REQ-INP-064: an off-screen shot is transmitted unclamped — 20 pixels left of the image
    /// gives x = −20 — and the core then detects no light.
    /// </summary>
    [Fact]
    public void REQ_INP_064_an_off_screen_shot_is_not_clamped()
    {
        CameraView view = DefaultView();
        Vector2 pointer = Project(view, new Vector3(-20f, 120f, 0f));

        Assert.True(ZapperAim.TryAim(view, pointer, Viewport, Vector3.Zero, 1f, out ZapperPosition position));
        Assert.InRange(position.X, -21, -19);
        Assert.False(position.IsOnScreen);

        // Below the image too: y = 248 − (−1) = 249, outside [0, 240).
        ZapperPosition below = ZapperAim.ToNesPixel(new Vector2(10f, -1f), Vector3.Zero, 1f);
        Assert.Equal(249, below.Y);
        Assert.False(below.IsOnScreen);
    }

    /// <summary>
    /// REQ-INP-064: a ray parallel to the plane, or pointing away from it, keeps the last valid
    /// position.
    /// </summary>
    [Fact]
    public void REQ_INP_064_a_ray_that_misses_the_plane_keeps_the_last_position()
    {
        ZapperState zapper = new();
        CameraView view = DefaultView();
        Vector2 pointer = Project(view, new Vector3(128f, 120f, 0f));

        zapper.Update(active: true, view, pointer, Viewport, rightButton: false, Vector3.Zero);
        ZapperPosition valid = zapper.Position;
        Assert.True(zapper.HasPosition);

        // A camera looking along the plane: the forward axis has no z component at all.
        SceneCamera sideways = new();
        sideways.ResetView();
        sideways.Current.Angles = new Vector3(0f, 90f, 0f);
        sideways.SnapToSetpoints();
        CameraView parallel = sideways.BuildView(ConversionSettings.DefaultClipping, Viewport.X / Viewport.Y);

        // Aiming at the very centre of that view: the ray runs along the plane.
        Assert.False(ZapperAim.TryIntersect(
            parallel with { Forward = new Vector3(1f, 0f, 0f), Right = new Vector3(0f, 0f, 1f), Up = Vector3.UnitY },
            new Vector2(Viewport.X * 0.5f, Viewport.Y * 0.5f),
            Viewport,
            Vector3.Zero,
            out _));

        zapper.Update(
            active: true,
            parallel with { Forward = new Vector3(1f, 0f, 0f), Right = new Vector3(0f, 0f, 1f), Up = Vector3.UnitY },
            new Vector2(Viewport.X * 0.5f, Viewport.Y * 0.5f),
            Viewport,
            rightButton: false,
            Vector3.Zero);

        Assert.Equal(valid, zapper.Position);
    }

    /// <summary>
    /// REQ-INP-062, §4.1 and DEC-02-Q1: the trigger is the right mouse button, bit 4 is 1 when
    /// it is released and 0 while it is held, and the left button does nothing to the Zapper.
    /// </summary>
    [Fact]
    public void REQ_INP_062_the_trigger_is_the_right_button_with_the_dec_02_q1_polarity()
    {
        ZapperState zapper = new();
        CameraView view = DefaultView();
        Vector2 pointer = Project(view, new Vector3(128f, 120f, 0f));

        zapper.Update(active: true, view, pointer, Viewport, rightButton: false, Vector3.Zero);
        Assert.False(zapper.TriggerHeld);
        Assert.Equal(0x10, zapper.PortByte);

        zapper.Update(active: true, view, pointer, Viewport, rightButton: true, Vector3.Zero);
        Assert.True(zapper.TriggerHeld);
        Assert.Equal(0x00, zapper.PortByte);

        // The word the core receives carries that byte on the Zapper port and nothing else.
        Assert.Equal(
            0x00,
            (InputRouting.CoreWord(
                InputAction.None,
                InputAction.A,
                ControllerPortType.Controller,
                ControllerPortType.Zapper,
                zapper.PortByte) >> 8) & 0xFF);
    }

    /// <summary>
    /// REQ-INP-061: while the Zapper is not active — in pause, or with no Zapper port — moving
    /// the mouse does not change the position transmitted to the core.
    /// </summary>
    [Fact]
    public void REQ_INP_061_an_inactive_zapper_ignores_the_mouse()
    {
        ZapperState zapper = new();
        CameraView view = DefaultView();

        zapper.Update(
            active: true,
            view,
            Project(view, new Vector3(128f, 120f, 0f)),
            Viewport,
            rightButton: false,
            Vector3.Zero);

        ZapperPosition frozen = zapper.Position;

        zapper.Update(
            active: false,
            view,
            Project(view, new Vector3(32f, 32f, 0f)),
            Viewport,
            rightButton: true,
            Vector3.Zero);

        Assert.Equal(frozen, zapper.Position);
        Assert.False(zapper.TriggerHeld);
    }

    /// <summary>
    /// REQ-CORE-061 with REQ-INP-062: a read of <c>$4017</c> with the trigger released and the
    /// aim off screen gives <c>0x18</c>; with the trigger held it gives <c>0x08</c>.
    /// </summary>
    [Fact]
    public void REQ_INP_065_the_core_reads_the_zapper_byte_the_host_composed()
    {
        NesCore core = new();
        core.LoadRom(SyntheticRom.Build(), "zapper.nes");
        core.SetPortType(1, ControllerPortType.Zapper);
        core.ZapperPositionSource = _ => new ZapperPosition(-20, 120);

        core.RunFrame(InputRouting.CoreWord(
            InputAction.None,
            InputAction.None,
            ControllerPortType.Controller,
            ControllerPortType.Zapper,
            ZapperState.ReleasedByte));

        Assert.Equal(0x18, core.Input.Read(1, openBus: 0x00));

        core.RunFrame(InputRouting.CoreWord(
            InputAction.None,
            InputAction.None,
            ControllerPortType.Controller,
            ControllerPortType.Zapper,
            ZapperState.PressedByte));

        Assert.Equal(0x08, core.Input.Read(1, openBus: 0x00));
    }

    /// <summary>
    /// REQ-INP-060 and DEC-09-Q6 (divergence D): the port types start on Controller, are not
    /// persisted, and go back to Controller/Controller when a <b>different</b> game is loaded,
    /// before the profile's start hook. Relaunching the same game keeps them.
    /// </summary>
    [Fact]
    public void REQ_INP_060_a_different_game_puts_both_ports_back_on_controllers()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("duck", rom, title: "Duck Hunt");
        harness.Catalog.Add("other", rom, title: "Other Game");

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("duck"));
        Assert.Equal(ControllerPortType.Controller, harness.Host.PortType(0));
        Assert.Equal(ControllerPortType.Controller, harness.Host.PortType(1));

        // A Zapper profile would do this from its Start hook (SPEC-05 REQ-LUA-032).
        harness.Host.Session!.Core.SetPortType(1, ControllerPortType.Zapper);
        Assert.Equal(ControllerPortType.Zapper, harness.Host.PortType(1));
        Assert.True(harness.Host.ZapperActive);

        // Relaunching the same game is not a different game: the types are left alone.
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("duck"));
        Assert.Equal(ControllerPortType.Zapper, harness.Host.PortType(1));

        // Another game sends both ports back to Controller.
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("other"));
        Assert.Equal(ControllerPortType.Controller, harness.Host.PortType(0));
        Assert.Equal(ControllerPortType.Controller, harness.Host.PortType(1));
        Assert.False(harness.Host.ZapperActive);
    }

    /// <summary>
    /// REQ-INP-004 end to end: with port 2 in <c>Zapper</c>, J2 holding A reaches the core as the
    /// Zapper byte, and the right button pulls the trigger down to <c>0x00</c>.
    /// </summary>
    [Fact]
    public void REQ_INP_004_a_zapper_port_shows_the_trigger_and_no_button_to_the_core()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("duck", rom, title: "Duck Hunt");
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("duck"));

        harness.Host.Session!.Core.SetPortType(1, ControllerPortType.Zapper);

        // Pointer over the centre of the image, right button held.
        CameraView view = harness.Scene.Camera.BuildView(
            harness.Scene.Clipping,
            Viewport.X / Viewport.Y);
        Vector2 pointer = Project(view, new Vector3(128f, 128f, 0f));

        harness.Tick(
            1.0 / 60.0,
            new InputSnapshot(
                InputAction.None,
                InputAction.A,
                pointer,
                Viewport,
                RightButton: true));

        Assert.Equal(0x00, harness.Host.Session!.Core.Input.PortByte(1));
        Assert.True(harness.Host.Zapper.TriggerHeld);
        Assert.InRange(harness.Host.Zapper.Position.X, 127, 129);
        Assert.InRange(harness.Host.Zapper.Position.Y, 119, 121);

        // Releasing the button gives 0x10, and J2's A still never reaches the port.
        harness.Tick(
            1.0 / 60.0,
            new InputSnapshot(InputAction.None, InputAction.A, pointer, Viewport));

        Assert.Equal(0x10, harness.Host.Session!.Core.Input.PortByte(1));
    }

    /// <summary>The default camera of REQ-SCN-012 with the default clip margins.</summary>
    private static CameraView DefaultView(Vec4F? clipping = null)
    {
        SceneCamera camera = new();
        camera.ResetView();
        camera.SnapToSetpoints();
        return camera.BuildView(
            clipping ?? ConversionSettings.DefaultClipping,
            Viewport.X / Viewport.Y);
    }

    /// <summary>
    /// The screen position of a scene point, which is the inverse of the ray cast and therefore
    /// what lets §5.3 be expressed in scene points as the spec does.
    /// </summary>
    private static Vector2 Project(in CameraView view, Vector3 scenePoint)
    {
        Vector4 clip = Vector4.Transform(
            new Vector4(SceneCamera.ToGl(scenePoint), 1f),
            view.ViewProjection);

        float ndcX = clip.X / clip.W;
        float ndcY = clip.Y / clip.W;

        return new Vector2(
            (ndcX + 1f) * 0.5f * Viewport.X,
            (1f - ndcY) * 0.5f * Viewport.Y);
    }
}
