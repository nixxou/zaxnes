using System.Numerics;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// SPEC-08 §3.7 and §3.8: the port types, the mouse Zapper — trigger, aim, off-screen shot — and
/// the script pad.
/// </summary>
public sealed class InputZapperTests
{
    private static readonly Vec4F DefaultClipping = new(0.1f, 8.1f, 8.1f, 0.1f);

    /// <summary>
    /// REQ-INP-060 and §4.1: both ports start in Controller, the type is not persisted, and a
    /// script can set port 2 to Zapper, which reaches the core at once.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_060_the_ports_start_on_controller_and_a_script_changes_them()
    {
        GameProfile profile = ZapperProfile("Setting:SetInputType(0, 1)");
        using HostDriver host = HostDriver.Create(profile);

        Assert.Equal(ControllerPortType.Controller, host.PortType(0));
        Assert.Equal(ControllerPortType.Controller, host.PortType(1));

        host.Launch(host.EntryIdByTitle("Zapper"), host.WriteRom(LoopRom.Counter()));

        Assert.Equal(ControllerPortType.Controller, host.PortType(0));
        Assert.Equal(ControllerPortType.Zapper, host.PortType(1));
        Assert.True(host.ZapperActive);

        // Nothing of the port types is written to disk (REQ-INP-060, REQ-PERS-085).
        host.Shutdown();
        string settings = File.Exists(host.Paths.SettingsFile)
            ? File.ReadAllText(host.Paths.SettingsFile)
            : string.Empty;
        Assert.DoesNotContain("port_type", settings, StringComparison.Ordinal);
    }

    /// <summary>
    /// REQ-INP-060 and REQ-PERS-012 (DEC-09-Q6): loading a game different from the previous one
    /// puts both ports back to Controller, before the profile's start-up hook.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_060_a_different_game_puts_both_ports_back_to_controller()
    {
        GameProfile profile = ZapperProfile("Setting:SetInputType(0, 1)");
        using HostDriver host = HostDriver.Create(profile);

        string zapperGame = host.EntryIdByTitle("Zapper");
        string rom = host.WriteRom(LoopRom.Counter());
        host.Launch(zapperGame, rom);
        Assert.Equal(ControllerPortType.Zapper, host.PortType(1));

        // Another game: the types are back on two controllers before its Start hook runs.
        host.Launch(host.TwoDimensionalModeId!, rom);
        Assert.Equal(ControllerPortType.Controller, host.PortType(0));
        Assert.Equal(ControllerPortType.Controller, host.PortType(1));
        Assert.False(host.ZapperActive);
    }

    /// <summary>
    /// REQ-INP-061: the Zapper is active only while a game runs with a Zapper port, and in pause
    /// moving the mouse changes nothing of what the core sees.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_061_the_zapper_only_follows_the_mouse_while_the_game_runs()
    {
        GameProfile profile = ZapperProfile("Setting:SetInputType(0, 1)");
        using HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle("Zapper"), host.WriteRom(LoopRom.Counter()));

        host.Tick(1.0 / 60.0, Snapshot(new Vector2(320f, 240f)));
        ZapperPosition running = host.Zapper.Position;

        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);

        host.Tick(1.0 / 60.0, Snapshot(new Vector2(10f, 10f)));
        Assert.Equal(running, host.Zapper.Position);

        host.Run(ZaxCommands.Resume);
        host.Tick(1.0 / 60.0, Snapshot(new Vector2(10f, 10f)));
        Assert.NotEqual(running, host.Zapper.Position);
    }

    /// <summary>
    /// REQ-INP-062 and §4.1: the trigger is the right mouse button, the left one does not act on
    /// it, and bit 4 of the port byte is 1 when released and 0 when held.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_062_the_trigger_is_the_right_mouse_button()
    {
        GameProfile profile = ZapperProfile("Setting:SetInputType(0, 1)");
        using HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle("Zapper"), host.WriteRom(LoopRom.Counter()));

        Assert.Equal(0x10, ZapperState.ReleasedByte);
        Assert.Equal(0x00, ZapperState.PressedByte);

        host.Tick(1.0 / 60.0, Snapshot(new Vector2(320f, 240f), left: true));
        Assert.False(host.Zapper.TriggerHeld);
        Assert.Equal(ZapperState.ReleasedByte, host.Zapper.PortByte);
        Assert.Equal(
            ZapperState.ReleasedByte,
            (InputRouting.CoreWord(
                InputAction.None,
                InputAction.None,
                ControllerPortType.Controller,
                ControllerPortType.Zapper,
                host.Zapper.PortByte) >> 8) & 0xFF);

        host.Tick(1.0 / 60.0, Snapshot(new Vector2(320f, 240f), right: true));
        Assert.True(host.Zapper.TriggerHeld);
        Assert.Equal(ZapperState.PressedByte, host.Zapper.PortByte);
    }

    /// <summary>
    /// REQ-INP-063 and §5.3: the relation <c>px = trunc((x − Ox) / e)</c>,
    /// <c>py = trunc(248 − (y − Oy) / e)</c> on the six vectors of the table.
    /// </summary>
    [Theory]
    [InlineData(128f, 120f, 0f, 0f, 128, 128)]
    [InlineData(128f, 128f, 0f, 0f, 128, 120)]
    [InlineData(0f, 248f, 0f, 0f, 0, 0)]
    [InlineData(0f, 239.9f, 0f, 0f, 0, 8)]
    [InlineData(-20f, 120f, 0f, 0f, -20, 128)]
    [InlineData(128f, 120f, 10f, 0f, 118, 128)]
    public void REQ_INP_063_the_aim_relation_gives_the_vectors_of_paragraph_5_3(
        float x,
        float y,
        float offsetX,
        float offsetY,
        int px,
        int py)
    {
        ZapperPosition position = ZapperAim.ToNesPixel(
            new Vector2(x, y),
            new Vector3(offsetX, offsetY, 0f));

        Assert.Equal(px, position.X);
        Assert.Equal(py, position.Y);
        Assert.Equal(248f, ZapperAim.SceneHeight);
    }

    /// <summary>
    /// REQ-INP-063, notes: a positive <c>ZapperOffset.y</c> is an image moved up, so the same
    /// point of the screen aims at a NES pixel lower down.
    /// </summary>
    [Fact]
    public void REQ_INP_063_a_positive_vertical_offset_aims_lower_down()
    {
        ZapperPosition none = ZapperAim.ToNesPixel(new Vector2(128f, 120f), Vector3.Zero);
        ZapperPosition raised = ZapperAim.ToNesPixel(new Vector2(128f, 120f), new Vector3(0f, 16f, 0f));

        Assert.Equal(none.Y + 16, raised.Y);
    }

    /// <summary>
    /// REQ-INP-063, last sentence of the criterion: through the real camera, the position is the
    /// one the relation gives for the scene point under the pointer, in the default view and with
    /// the camera turned 30° (±1).
    /// </summary>
    [Theory]
    [InlineData(0f)]
    [InlineData(30f)]
    public void REQ_INP_063_the_ray_cast_lands_on_the_scene_point_under_the_pointer(float yaw)
    {
        SceneCamera camera = new();
        camera.ResetView();
        camera.Current.Angles = new Vector3(0f, yaw, 0f);
        camera.SnapToSetpoints();

        CameraView view = camera.BuildView(DefaultClipping, 640f / 480f);
        Vector2 size = new(640f, 480f);

        foreach (Vector2 scenePoint in (Vector2[])
            [new(128f, 120f), new(128f, 128f), new(40f, 200f), new(200f, 60f)])
        {
            Vector2 pointer = Project(view, new Vector3(scenePoint, 0f), size);
            Assert.True(
                ZapperAim.TryAim(view, pointer, size, Vector3.Zero, 1f, out ZapperPosition aimed),
                $"the ray missed the plane for {scenePoint}");

            ZapperPosition expected = ZapperAim.ToNesPixel(scenePoint, Vector3.Zero);
            Assert.True(
                Math.Abs(expected.X - aimed.X) <= 1 && Math.Abs(expected.Y - aimed.Y) <= 1,
                $"yaw {yaw}: expected ({expected.X}, {expected.Y}), got ({aimed.X}, {aimed.Y})");
        }
    }

    /// <summary>
    /// REQ-INP-064: a shot outside the image is transmitted unclamped, and a ray that misses the
    /// plane keeps the last valid position.
    /// </summary>
    [Fact]
    public void REQ_INP_064_an_off_screen_shot_is_not_clamped_and_a_miss_keeps_the_position()
    {
        ZapperPosition left = ZapperAim.ToNesPixel(new Vector2(-20f, 120f), Vector3.Zero);
        Assert.Equal(-20, left.X);

        ZapperPosition below = ZapperAim.ToNesPixel(new Vector2(300f, -10f), Vector3.Zero);
        Assert.Equal(300, below.X);
        Assert.Equal(258, below.Y);

        SceneCamera camera = new();
        camera.ResetView();
        camera.SnapToSetpoints();
        CameraView view = camera.BuildView(DefaultClipping, 1f);
        Vector2 size = new(256f, 256f);

        ZapperState zapper = new();
        zapper.Update(true, view, new Vector2(128f, 128f), size, false, Vector3.Zero);
        ZapperPosition valid = zapper.Position;
        Assert.True(zapper.HasPosition);

        // A camera looking along the plane: the ray is parallel to it and never meets it.
        camera.Current.Angles = new Vector3(0f, 90f, 0f);
        camera.SnapToSetpoints();
        CameraView sideways = camera.BuildView(DefaultClipping, 1f);
        Assert.False(ZapperAim.TryIntersect(sideways, new Vector2(128f, 128f), size, Vector3.Zero, out _));

        zapper.Update(true, sideways, new Vector2(128f, 128f), size, false, Vector3.Zero);
        Assert.Equal(valid, zapper.Position);
    }

    /// <summary>
    /// REQ-INP-065 and §5.3: the Zapper hands the core the trigger and the position; the light
    /// detection is the core's business. The six rows of the table are checked on the port byte.
    /// </summary>
    [Fact]
    public void REQ_INP_065_the_zapper_supplies_the_trigger_and_the_position()
    {
        ZapperState zapper = new();
        SceneCamera camera = new();
        camera.ResetView();
        camera.SnapToSetpoints();
        CameraView view = camera.BuildView(DefaultClipping, 1f);
        Vector2 size = new(256f, 256f);

        zapper.Update(true, view, new Vector2(128f, 128f), size, rightButton: false, Vector3.Zero);
        Assert.Equal(0x10, zapper.PortByte);

        zapper.Update(true, view, new Vector2(128f, 128f), size, rightButton: true, Vector3.Zero);
        Assert.Equal(0x00, zapper.PortByte);

        // The class exposes no light bit: bit 3 is the core's (SPEC-02 REQ-CORE-061).
        Assert.Equal(0, zapper.PortByte & 0x08);
        Assert.Equal(0, ZapperState.ReleasedByte & 0x08);

        zapper.Reset();
        Assert.False(zapper.TriggerHeld);
        Assert.False(zapper.HasPosition);
    }

    /// <summary>
    /// REQ-INP-066: no reticle and no gun model is drawn — the scene has no such element — and
    /// the aim keeps working while the cursor is masked.
    /// </summary>
    [Fact]
    public void REQ_INP_066_no_reticle_is_drawn_and_the_aim_works_when_the_cursor_is_hidden()
    {
        // Nothing in the scene stands for a reticle or a gun.
        foreach (var property in typeof(SceneState).GetProperties())
        {
            Assert.DoesNotContain("reticle", property.Name, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("crosshair", property.Name, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("gun", property.Name, StringComparison.OrdinalIgnoreCase);
        }

        // The cursor masking and the aim are independent: the Zapper never reads the cursor.
        CursorAutoHide cursor = new();
        for (int check = 0; check < CursorAutoHide.ChecksBeforeHiding; check++)
        {
            cursor.Update(gameRunning: true, pointerMoved: false, CursorAutoHide.CheckSeconds);
        }

        Assert.False(cursor.CursorVisible);

        SceneCamera camera = new();
        camera.ResetView();
        camera.SnapToSetpoints();
        ZapperState zapper = new();
        zapper.Update(
            true,
            camera.BuildView(DefaultClipping, 1f),
            new Vector2(128f, 128f),
            new Vector2(256f, 256f),
            rightButton: true,
            Vector3.Zero);

        Assert.True(zapper.HasPosition);
        Assert.True(zapper.TriggerHeld);
    }

    /// <summary>
    /// REQ-INP-070: the script pad is off by default — a profile that defines
    /// <c>InputProcess</c> without enabling it has no influence at all.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_070_the_script_pad_is_disabled_by_default()
    {
        GameProfile profile = PadProfile("function InputProcess(i) return 8 end");
        using HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle("Pad-probe"), host.WriteRom(LoopRom.Counter()));

        SessionDriver session = host.Session!;
        Assert.False(session.Scripts.ScriptPadEnabled);

        session.RunFrame();
        Assert.Equal(0, MergedMask(session, 0));
    }

    /// <summary>
    /// REQ-INP-071 and REQ-INP-072: once enabled the pad is called once per emulated frame and
    /// its mask is merged by OR into the player's; it can never mask a physical input.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_071_the_script_pad_is_additive_and_runs_once_per_frame()
    {
        GameProfile profile = PadProfile(
            """
            calls = 0
            function Start()
              ScriptPad:Enable(true)
              ScriptPad.PadIndex = 0
            end
            function InputProcess(index)
              calls = calls + 1
              padIndex = index
              if calls % 2 == 1 then return 8 else return 0 end
            end
            """);

        using HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle("Pad-probe"), host.WriteRom(LoopRom.Counter()));

        SessionDriver session = host.Session!;
        Assert.True(session.Scripts.ScriptPadEnabled);

        List<int> masks = [];
        for (int frame = 0; frame < 6; frame++)
        {
            session.RunFrame((int)InputAction.A);
            masks.Add(MergedMask(session, 0));
        }

        // Start came from the pad one frame out of two, A from the keyboard on every frame: the
        // pad added a bit and never took one away.
        Assert.Equal(3, masks.Count(mask => mask == 0x09));
        Assert.Equal(3, masks.Count(mask => mask == 0x01));
        Assert.Equal(6, session.Scripts.EvaluateNumber("return calls"));
        Assert.Equal(0, session.Scripts.EvaluateNumber("return padIndex"));
    }

    /// <summary>
    /// REQ-INP-071, last sentence: the returned mask is an action mask, so <c>0x400</c> turns the
    /// camera to the left.
    /// </summary>
    [Fact]
    public void REQ_INP_071_the_returned_mask_is_an_action_mask()
    {
        (InputAction one, InputAction two) = InputRouting.MergeScriptPad(
            InputAction.None,
            InputAction.None,
            (InputAction)0x400,
            padIndex: 0);

        Assert.Equal(InputAction.RotLeft, one);
        Assert.Equal(InputAction.None, two);
        Assert.Equal(
            CameraAction.RotateLeft,
            InputActions.ToCameraActions(InputRouting.CameraSources(one, two)));

        (one, two) = InputRouting.MergeScriptPad(
            InputAction.A,
            InputAction.B,
            InputAction.Start,
            padIndex: 1);
        Assert.Equal(InputAction.A, one);
        Assert.Equal(InputAction.B | InputAction.Start, two);
    }

    /// <summary>
    /// REQ-INP-073: an error in <c>InputProcess</c> makes the pad contribute 0, is reported once
    /// and stops the function being called again; the game stays playable.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_073_an_error_in_the_script_pad_is_reported_once()
    {
        GameProfile profile = PadProfile(
            """
            function Start() ScriptPad:Enable(true) end
            function InputProcess(index) error("boom") end
            """);

        using HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle("Pad-probe"), host.WriteRom(LoopRom.Counter()));

        SessionDriver session = host.Session!;
        for (int frame = 0; frame < 5; frame++)
        {
            session.RunFrame((int)InputAction.A);
            Assert.Equal(0x01, MergedMask(session, 0));
        }

        Assert.Equal(
            1,
            host.ScriptLog.Errors.Count(line => line.Contains("boom", StringComparison.Ordinal)));
    }

    /// <summary>
    /// REQ-INP-074: the VR hand and head data exist on desktop and read as the null vector.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_074_the_virtual_reality_vectors_are_null_on_desktop()
    {
        GameProfile profile = PadProfile(
            """
            function Start()
              local l = ScriptPad.LeftHand.pos
              local r = ScriptPad.RightHand.pos
              local h = ScriptPad.Head.pos
              local v = ScriptPad:HandPos(0)
              print(string.format("%g %g %g %g %g %g %g %g %g %g %g %g",
                l.x, l.y, l.z, r.x, r.y, r.z, h.x, h.y, h.z, v.x, v.y, v.z))
            end
            """);

        using HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle("Pad-probe"), host.WriteRom(LoopRom.Counter()));

        Assert.Empty(host.ScriptLog.Errors);
        Assert.Contains("0 0 0 0 0 0 0 0 0 0 0 0", host.ScriptLog.Prints);
    }

    /// <summary>The merged mask of a player, as the scripts of SPEC-05 §4.2 read it.</summary>
    private static int MergedMask(SessionDriver session, int player)
        => (int)session.Scripts.EvaluateNumber($"return Gamepad[{player}].State");

    private static InputSnapshot Snapshot(Vector2 pointer, bool left = false, bool right = false)
        => new(
            InputAction.None,
            InputAction.None,
            pointer,
            new Vector2(640f, 480f),
            LeftButton: left,
            RightButton: right);

    /// <summary>Projects a scene point to a pointer position in the render target.</summary>
    private static Vector2 Project(in CameraView view, Vector3 point, Vector2 size)
    {
        Vector4 clip = Vector4.Transform(point, view.ViewProjection);
        float x = clip.X / clip.W;
        float y = clip.Y / clip.W;
        return new Vector2((x + 1f) * 0.5f * size.X, (1f - y) * 0.5f * size.Y);
    }

    private static GameProfile ZapperProfile(string startBody)
        => Profile("Zapper-probe", $"function Start() {startBody} end");

    private static GameProfile PadProfile(string script) => Profile("Pad-probe", script);

    private static GameProfile Profile(string title, string script)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = title;
        profile.Scripts.Add(new ProfileScript("verify", true, script, "*"));
        return profile;
    }
}
