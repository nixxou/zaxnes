using System.Numerics;
using System.Reflection;
using Zax.App.Input;
using Zax.App.Persistence;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-046 — bending in 2D mode: the setting <c>scene.bend_2d_mode</c> of §3 and the three
/// automatic criteria of REQ-SCN-074, on the export harness of REQ-SCN-068, on the Zapper aim
/// and on the settings model. It closes Q-079 as reading 1.
/// </summary>
public sealed class Amd046Tests
{
    private static readonly Vector2 Viewport = new(640f, 480f);

    /// <summary>The tolerance of the REQ-SCN-067 vectors the first criterion reuses.</summary>
    private const float BentTolerance = 0.05f;

    /// <summary>The tolerance of the flat case, 10⁻³ unit.</summary>
    private const float FlatTolerance = 1e-3f;

    /// <summary>A grid of at least 33 × 33 distinct vertices (32 × 32 cells).</summary>
    private const int LeastVertices = 33 * 33;

    // ------------------------------------------------------------ §3 the setting

    /// <summary>
    /// AMD-046 §3: a boolean user preference, true by default, applied immediately, exposed to
    /// libretro as <c>enabled</c> / <c>disabled</c> and <b>not</b> to Lua — neither as a member
    /// of <c>Setting</c> nor as a style parameter of REQ-LUA-114.
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-PERS-001, REQ-PERS-083
    public void REQ_SCN_074_the_setting_is_a_user_preference_with_no_lua_access()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(SettingKeys.SceneBend2dMode);

        Assert.Equal("scene.bend_2d_mode", descriptor.Key);
        Assert.Equal(SettingKind.Bool, descriptor.Kind);
        Assert.Equal(SettingScope.User, descriptor.Scope);
        Assert.Equal(SettingEffect.Immediate, descriptor.Effect);
        Assert.True(descriptor.DefaultValue.AsBool);

        // No Lua access: the original program has none (AMD-046 §2, notes).
        Assert.Null(descriptor.LuaPath);
        Assert.DoesNotContain(
            SettingKeys.SceneBend2dMode,
            SettingsCatalogue.VisualStyleParameters[1].Values);

        // The line added to the table of SPEC-09 §4.6.
        LibretroCoreOption option =
            Assert.NotNull(LibretroCoreOptions.Find("zaxnes_scene_bend_2d_mode"));
        Assert.Equal(SettingKeys.SceneBend2dMode, option.SettingKey);
        Assert.Equal(["enabled", "disabled"], option.Values);
        Assert.Equal("enabled", option.DefaultValue);
        Assert.Equal("Scene", option.Category);
    }

    /// <summary>
    /// AMD-046 §3: the checkbox sits in the advanced « Geometry » group of AMD-043 §6, right
    /// after « Arc Center », with the label and the help of the amendment — English by default,
    /// French once the interface is in French (AMD-035).
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-UI-202
    public void REQ_SCN_074_the_checkbox_follows_the_arc_centre_in_the_geometry_group()
    {
        IReadOnlyList<string> geometry = GeometryKeys();
        Assert.Equal(SettingKeys.SceneBend2dMode, geometry[^1]);
        Assert.Equal(SettingKeys.SceneArcCenter, geometry[^2]);
        Assert.Equal(GeometryGroupKey, OptionsGroupOf(SettingKeys.SceneBend2dMode));

        (string Tab, string Key, string Label, double Minimum, double Maximum) row = Assert.Single(
            UiDriver.OptionRows(),
            r => string.Equals(r.Key, SettingKeys.SceneBend2dMode, StringComparison.Ordinal));

        Assert.Contains(row.Label, (string[])["Bend 2D Mode", "Courber le mode 2D"]);

        string help = HelpOf(SettingKeys.SceneBend2dMode);
        Assert.True(
            help.Contains("Turn it off to keep the picture flat.", StringComparison.Ordinal)
            || help.Contains("Désactivez-le pour garder l'image plate.", StringComparison.Ordinal),
            $"the help of scene.bend_2d_mode is not the one of §3: '{help}'.");
    }

    /// <summary>
    /// REQ-SCN-074, third automatic criterion: the value survives a restart, and it is
    /// « active » on a fresh install.
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-PERS-001, REQ-PERS-002
    public void REQ_SCN_074_the_value_survives_a_restart_and_starts_enabled()
    {
        // A fresh install: no settings file at all.
        MemorySettingsStorage storage = new();
        SettingsModel fresh = new();
        Assert.True(fresh.GetBool(SettingKeys.SceneBend2dMode));
        new SettingsFile(storage).Load(fresh, DateTimeOffset.UnixEpoch);
        Assert.True(fresh.GetBool(SettingKeys.SceneBend2dMode));

        fresh.SetBool(SettingKeys.SceneBend2dMode, false);
        Assert.True(new SettingsFile(storage).Save(fresh));

        string json = System.Text.Encoding.UTF8.GetString(storage.Read(SettingsFile.DefaultName)!);
        Assert.Contains(SettingKeys.SceneBend2dMode, json, StringComparison.Ordinal);

        SettingsModel restarted = new();
        new SettingsFile(storage).Load(restarted, DateTimeOffset.UnixEpoch);
        Assert.False(restarted.GetBool(SettingKeys.SceneBend2dMode));

        // And the host property reads the same storage.
        AppSettings settings = new(restarted);
        Assert.False(settings.Bend2dMode);
        settings.Bend2dMode = true;
        Assert.True(restarted.GetBool(SettingKeys.SceneBend2dMode));
    }

    // --------------------------------------------------- §2 the first criterion

    /// <summary>
    /// REQ-SCN-074, first criterion, setting active: with the left-right bend at 90°, every
    /// exported vertex of the 2D-mode rectangle, of flat position <c>P</c>, is less than 0,05
    /// unit from <c>B(P)</c>, and the rectangle counts at least 1 089 distinct vertices.
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-SCN-067
    public void REQ_SCN_074_the_rectangle_is_bent_when_the_setting_is_active()
    {
        SceneState scene = TwoDimensionalScene();
        scene.Settings.BendLeftRight = 90;
        Assert.True(scene.Settings.Bend2dMode);

        ExportedObject rectangle = Assert.Single(SceneExport.Objects(scene, View(scene)));
        SpaceBend bend = scene.Settings.Bend;

        Assert.True(rectangle.Bent);
        Assert.True(
            Distinct(rectangle.Positions) >= LeastVertices,
            $"the rectangle exported {Distinct(rectangle.Positions)} distinct vertices.");

        for (int v = 0; v < rectangle.Positions.Count; v++)
        {
            Vector3 flat = rectangle.Unbent[v];
            Vector3 expected = ToVector(bend.Apply(ToVec3(flat)));
            Assert.True(
                Vector3.Distance(rectangle.Positions[v], expected) < BentTolerance,
                $"vertex {v}: {rectangle.Positions[v]} is not B({flat}) = {expected}.");
        }

        // It really did move: a rectangle left flat would pass the loop above only with a null
        // curvature.
        Assert.True(
            MaxTravel(rectangle) > 1f,
            "the rectangle did not bend at all.");

        // The rectangle of §2: z = 0, x ∈ [0, 256], y ∈ [8, 248] before bending.
        Assert.All(rectangle.Unbent, p =>
        {
            Assert.Equal(0f, p.Z, FlatTolerance);
            Assert.InRange(p.X, 0f, 256f);
            Assert.InRange(p.Y, 8f, 248f);
        });
    }

    /// <summary>
    /// REQ-SCN-074, first criterion, setting inactive: with the left-right bend at 90° every
    /// exported vertex is in the plane <c>z = 0</c>, at its flat position to 10⁻³ unit — and the
    /// subdivision is still there, since the first bullet imposes it unconditionally.
    /// </summary>
    [Fact] // REQ-SCN-074
    public void REQ_SCN_074_the_rectangle_stays_flat_when_the_setting_is_inactive()
    {
        SceneState scene = TwoDimensionalScene();
        scene.Settings.BendLeftRight = 90;
        scene.Settings.Bend2dMode = false;

        ExportedObject rectangle = Assert.Single(SceneExport.Objects(scene, View(scene)));

        Assert.False(rectangle.Bent);
        Assert.True(
            Distinct(rectangle.Positions) >= LeastVertices,
            $"the rectangle exported {Distinct(rectangle.Positions)} distinct vertices.");

        for (int v = 0; v < rectangle.Positions.Count; v++)
        {
            Assert.Equal(0f, rectangle.Positions[v].Z, FlatTolerance);
            Assert.True(
                Vector3.Distance(rectangle.Positions[v], rectangle.Unbent[v]) < FlatTolerance,
                $"vertex {v} left its flat position {rectangle.Unbent[v]}.");
        }
    }

    /// <summary>
    /// REQ-SCN-074: « whatever the settings of REQ-SCN-066 say ». Each of the four of them, on
    /// its own and all together, leaves the rectangle at its flat position while the setting is
    /// off.
    /// </summary>
    [Theory] // REQ-SCN-074, REQ-SCN-066
    [InlineData(180, 0, 0, false)]
    [InlineData(0, -90, 0, false)]
    [InlineData(0, 0, 120, false)]
    [InlineData(0, 0, 60, true)]
    [InlineData(90, 90, 90, true)]
    public void REQ_SCN_074_no_setting_of_REQ_SCN_066_moves_a_flat_rectangle(
        int bendLeftRight,
        int bendTopBottom,
        int arcHorizontal,
        bool arcAtEdge)
    {
        SceneState scene = TwoDimensionalScene();
        scene.Settings.Bend2dMode = false;
        scene.Settings.BendLeftRight = bendLeftRight;
        scene.Settings.BendTopBottom = bendTopBottom;
        scene.Settings.ArcHorizontal = arcHorizontal;
        scene.Settings.ArcAtEdge = arcAtEdge;

        ExportedObject rectangle = Assert.Single(SceneExport.Objects(scene, View(scene)));
        Assert.False(scene.Settings.Bend.IsIdentity);
        Assert.Equal(0f, MaxTravel(rectangle), FlatTolerance);
    }

    /// <summary>
    /// REQ-SCN-074, order clause of REQ-SCN-068 and first criterion: the cut by the visible
    /// rectangle (REQ-SCN-036, applied to the image by REQ-E3D-643) is judged on the positions
    /// <b>before</b> bending, so the set of visible NES rows and columns is the same with the
    /// setting on and off, for the default margins.
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-SCN-036
    public void REQ_SCN_074_the_visible_nes_rows_do_not_depend_on_the_setting()
    {
        SceneState bentScene = TwoDimensionalScene();
        bentScene.Settings.BendLeftRight = 90;
        ExportedObject bent = Assert.Single(SceneExport.Objects(bentScene, View(bentScene)));

        SceneState flatScene = TwoDimensionalScene();
        flatScene.Settings.BendLeftRight = 90;
        flatScene.Settings.Bend2dMode = false;
        ExportedObject flat = Assert.Single(SceneExport.Objects(flatScene, View(flatScene)));

        // The cut applies in both cases, and it is judged on the unbent positions, which the
        // setting does not touch.
        Assert.True(bent.Clipped);
        Assert.True(flat.Clipped);
        Assert.Equal(bent.Unbent, flat.Unbent);

        // Default margins (0,1 ; 8,1 ; 8,1 ; 0,1): the visible rows and columns of the two
        // exports, read off the positions the cut is judged on.
        VisibleRect rect = bentScene.VisibleRectangle;
        Assert.Equal(flatScene.VisibleRectangle, rect);

        SortedSet<int> bentRows = VisibleRows(bent, rect);
        SortedSet<int> flatRows = VisibleRows(flat, rect);
        Assert.NotEmpty(bentRows);
        Assert.Equal(bentRows, flatRows);
        Assert.Equal(VisibleColumns(bent, rect), VisibleColumns(flat, rect));

        // Had the cut been judged after the curvature, the bent rectangle would have lost rows:
        // part of its bent image is outside R, which is what makes the criterion discriminating.
        Assert.Contains(bent.Positions, p => !rect.Contains(p.X, p.Y));
    }

    /// <summary>
    /// REQ-SCN-074, first criterion, last member: in 3D mode toggling the setting changes no
    /// exported vertex at all.
    /// </summary>
    [Fact] // REQ-SCN-074
    public void REQ_SCN_074_the_setting_has_no_effect_in_three_dimensional_mode()
    {
        SceneState scene = ThreeDimensionalScene();
        scene.Settings.BendLeftRight = 90;
        CameraView view = View(scene);

        IReadOnlyList<ExportedObject> on = SceneExport.Objects(scene, view);
        scene.Settings.Bend2dMode = false;
        IReadOnlyList<ExportedObject> off = SceneExport.Objects(scene, view);

        Assert.NotEmpty(on);
        Assert.Equal(on.Count, off.Count);
        for (int i = 0; i < on.Count; i++)
        {
            Assert.Equal(on[i].Bent, off[i].Bent);
            Assert.Equal(on[i].Clipped, off[i].Clipped);
            Assert.Equal(on[i].Positions, off[i].Positions);
            Assert.Equal(on[i].Normals, off[i].Normals);
        }

        // And a backdrop stays subject to REQ-SCN-068 either way.
        scene.Backdrop.Enabled = true;
        scene.Settings.Bend2dMode = true;
        ExportedObject withBend = SceneExport.Objects(scene, view)[^1];
        scene.Settings.Bend2dMode = false;
        ExportedObject without = SceneExport.Objects(scene, view)[^1];

        Assert.True(withBend.Bent);
        Assert.True(without.Bent);
        Assert.Equal(withBend.Positions, without.Positions);
    }

    /// <summary>
    /// REQ-SCN-074, §2: a backdrop (REQ-SCN-048, driven by Lua) stays subject to REQ-SCN-068,
    /// whether the setting is on or off — it is never touched by <c>scene.bend_2d_mode</c>. In
    /// 2D mode the scene itself holds the rectangle alone (REQ-SCN-055, Q-022), so the clause
    /// bears on the frames where a backdrop is drawn at all, which the 3D-mode test above
    /// exercises; here the point is that enabling one changes nothing of the 2D mode.
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-SCN-048
    public void REQ_SCN_074_a_backdrop_is_never_touched_by_the_setting()
    {
        SceneState scene = TwoDimensionalScene();
        scene.Settings.BendLeftRight = 90;
        scene.Backdrop.Enabled = true;
        CameraView view = View(scene);

        foreach (bool bend2dMode in (bool[])[true, false])
        {
            scene.Settings.Bend2dMode = bend2dMode;

            // REQ-SCN-055: 2D mode shows the picture and nothing else, so the backdrop is not
            // part of the frame — and the setting does not change that either.
            ExportedObject rectangle = Assert.Single(SceneExport.Objects(scene, view));
            Assert.Equal(bend2dMode, rectangle.Bent);
        }
    }

    // -------------------------------------------------- §2 the second criterion

    /// <summary>
    /// REQ-SCN-074, second criterion: 2D mode, port 2 forced to <c>Zapper</c>, a null
    /// <c>ZapperOffset</c>, left-right bend at 90°. Setting inactive — REQ-INP-124 does not
    /// apply, the aim is the one of REQ-INP-063 — a pointer on the scene point
    /// <c>(128, 120, 0)</c> gives the transmitted position <c>(128, 128)</c> ±1. Setting active,
    /// the pointer on the bent image <c>B(128, 120, 0)</c>: the same position, inside the 0,5
    /// unit of REQ-INP-124.
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-INP-124, REQ-INP-063
    public void REQ_SCN_074_a_flat_two_dimensional_mode_aims_as_REQ_INP_063()
    {
        SpaceBend bend = new(90f, 0f, 0f, arcAtEdge: false);
        CameraView view = DefaultView();
        Vec3 centre = new(128f, 120f, 0f);

        // Setting inactive: the aim is computed with the identity, so the pointer is placed on
        // the flat point itself.
        SpaceBend flat = ZapperAim.AimBend(twoDimensionalMode: true, bend2dMode: false, bend);
        Assert.True(flat.IsIdentity);

        ZapperState zapper = new();
        zapper.Update(
            active: true,
            view,
            Project(view, ToVector(centre)),
            Viewport,
            rightButton: false,
            Vector3.Zero,
            1f,
            flat);

        Assert.True(zapper.HasPosition);
        Assert.Equal(128, zapper.Position.X, tolerance: 1);
        Assert.Equal(128, zapper.Position.Y, tolerance: 1);

        // Setting active: REQ-INP-124 applies unchanged, so the pointer is placed on the bent
        // image and the transmitted position is the same.
        SpaceBend active = ZapperAim.AimBend(twoDimensionalMode: true, bend2dMode: true, bend);
        Assert.False(active.IsIdentity);

        ZapperState bentZapper = new();
        bentZapper.Update(
            active: true,
            view,
            Project(view, ToVector(bend.Apply(centre))),
            Viewport,
            rightButton: false,
            Vector3.Zero,
            1f,
            active);

        Assert.True(bentZapper.HasPosition);
        Assert.Equal(128, bentZapper.Position.X, tolerance: 1);
        Assert.Equal(128, bentZapper.Position.Y, tolerance: 1);

        // The exception is the 2D mode's alone: in 3D mode the curvature is handed over as it
        // was, whatever the setting says.
        Assert.Equal(bend, ZapperAim.AimBend(false, false, bend));
        Assert.Equal(bend, ZapperAim.AimBend(false, true, bend));
    }

    /// <summary>
    /// REQ-SCN-074, second criterion, the discriminating half: a flat 2D mode aims where the
    /// picture is, which is <b>not</b> what REQ-INP-124 would answer for the same pointer.
    /// </summary>
    [Fact] // REQ-SCN-074, REQ-INP-124
    public void REQ_SCN_074_the_two_aims_differ_away_from_the_centre_of_curvature()
    {
        SpaceBend bend = new(90f, 0f, 0f, arcAtEdge: false);
        CameraView view = DefaultView();

        // A point off the axis of the bend: its image moves, so the two readings of the same
        // pointer land on different NES pixels.
        Vec3 point = new(224f, 200f, 0f);
        Vector2 pointer = Project(view, ToVector(point));

        Assert.True(ZapperAim.TryIntersect(
            view, pointer, Viewport, Vector3.Zero, SpaceBend.None, 1f, out Vector2 asFlat));
        Assert.True(ZapperAim.TryIntersect(
            view, pointer, Viewport, Vector3.Zero, bend, 1f, out Vector2 asBent));

        Assert.True(
            Vector2.Distance(asFlat, new Vector2(point.X, point.Y)) < 0.5f,
            $"the flat aim gave {asFlat} instead of ({point.X}, {point.Y}).");
        Assert.True(
            Vector2.Distance(asFlat, asBent) > 1f,
            $"the two aims are the same point: {asFlat} and {asBent}.");
    }

    // ------------------------------------------------------------------ fixtures

    /// <summary>
    /// A 2D-mode frame: the "2D Mode" profile holds no pattern at all, so the scene shows the
    /// core's pixel frame instead of the converted tiles (REQ-SCN-055, Q-022).
    /// </summary>
    private static SceneState TwoDimensionalScene()
    {
        SceneFixture fixture = new() { TwoDimensionalMode = true };
        SceneState scene = fixture.Build();
        scene.TwoDimensionalMode = true;
        return scene;
    }

    /// <summary>A plain 3D frame: one converted tile in L1 and one HUD tile.</summary>
    private static SceneState ThreeDimensionalScene()
    {
        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.Ui)
            .Tile(SceneFixture.Solid(1, 0), 64, 120)
            .Tile(SceneFixture.Solid(1, 1), 120, 120);

        return fixture.Build();
    }

    /// <summary>The NES rows the cut of REQ-SCN-036 keeps, read off the unbent positions.</summary>
    private static SortedSet<int> VisibleRows(ExportedObject rectangle, VisibleRect rect)
    {
        SortedSet<int> rows = [];
        foreach (Vector3 p in rectangle.Unbent)
        {
            if (rect.Contains(p.X, p.Y))
            {
                rows.Add((int)MathF.Floor(SceneConstants.SceneHeight - p.Y));
            }
        }

        return rows;
    }

    /// <summary>The NES columns the cut keeps, read the same way.</summary>
    private static SortedSet<int> VisibleColumns(ExportedObject rectangle, VisibleRect rect)
    {
        SortedSet<int> columns = [];
        foreach (Vector3 p in rectangle.Unbent)
        {
            if (rect.Contains(p.X, p.Y))
            {
                columns.Add((int)MathF.Floor(p.X));
            }
        }

        return columns;
    }

    /// <summary>How far the farthest vertex travelled between its flat and final positions.</summary>
    private static float MaxTravel(ExportedObject exported)
    {
        float largest = 0f;
        for (int i = 0; i < exported.Positions.Count; i++)
        {
            largest = MathF.Max(
                largest,
                Vector3.Distance(exported.Positions[i], exported.Unbent[i]));
        }

        return largest;
    }

    /// <summary>How many distinct positions the export holds (REQ-SCN-074: at least 1 089).</summary>
    private static int Distinct(IReadOnlyList<Vector3> positions)
        => positions.Distinct().Count();

    private static CameraView View(SceneState scene)
    {
        scene.Camera.SnapToSetpoints();
        return scene.Camera.BuildView(scene.Clipping, Viewport.X / Viewport.Y);
    }

    private static CameraView DefaultView()
    {
        SceneCamera camera = new();
        camera.ResetView();
        camera.SnapToSetpoints();
        return camera.BuildView(ConversionSettings.DefaultClipping, Viewport.X / Viewport.Y);
    }

    /// <summary>The pointer a scene point projects to (the frame is left-handed).</summary>
    private static Vector2 Project(in CameraView view, Vector3 point)
    {
        Vector4 clip = Vector4.Transform(new Vector3(point.X, point.Y, -point.Z), view.ViewProjection);
        float x = clip.X / clip.W;
        float y = clip.Y / clip.W;
        return new Vector2((x + 1f) * 0.5f * Viewport.X, (1f - y) * 0.5f * Viewport.Y);
    }

    private static Vec3 ToVec3(Vector3 v) => new(v.X, v.Y, v.Z);

    private static Vector3 ToVector(Vec3 v) => new(v.X, v.Y, v.Z);

    // The options window lives in the shipped assembly, so it is read the way UiDriver does.

    private static string GeometryGroupKey { get; } = (string)AppType("Zax.App.Ui.UiKeys")
        .GetField("GroupGeometry", BindingFlags.Public | BindingFlags.Static)!
        .GetValue(null)!;

    private static IReadOnlyList<string> GeometryKeys()
        => [.. (IEnumerable<string>)AppType("Zax.App.Ui.OptionsModel")
            .GetProperty("GeometryKeys", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!];

    private static string? OptionsGroupOf(string key)
        => (string?)AppType("Zax.App.Ui.OptionsModel")
            .GetMethod("GroupOf", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [SettingsCatalogue.Get(key)]);

    private static string HelpOf(string key)
        => (string?)AppType("Zax.App.Ui.UiText")
            .GetMethod("HelpOf", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [SettingsCatalogue.Get(key)]) ?? string.Empty;

    private static Type AppType(string name)
        => typeof(Zax.App.AppOptions).Assembly.GetType(name)
        ?? throw new InvalidOperationException($"Zax.App has no type '{name}'");
}
