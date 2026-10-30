using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// AMD-043 REQ-SCN-068 on the GPU: the vertex shaders really place the geometry at
/// <c>B(P)</c>, the HUD stays where it was, and the shadow pass follows the bent surface.
/// </summary>
/// <remarks>
/// <see cref="Zax.Engine3D.SpaceBend"/> is checked on its own against the thirteen vectors of
/// REQ-SCN-067; what these tests add is that the GLSL counterpart agrees with it, which no
/// CPU-side harness can show.
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class BendTests
{
    [GlFact] // REQ-SCN-067, REQ-SCN-068
    public void REQ_SCN_068_the_shader_places_the_tiles_at_their_bent_position()
    {
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = Marker();
        Capture flat = gl.Render(renderer, scene);
        Vector2 before = Centroid(flat);
        CameraView view = renderer.LastView;

        scene.Settings.BendLeftRight = 90;
        Capture bent = gl.Render(renderer, scene);
        Vector2 after = Centroid(bent);

        // The marker really moved …
        Assert.True(
            Vector2.Distance(before, after) > 8f,
            $"the curvature did not move the marker: {before} → {after}.");

        // … and it moved to where B puts it. The marker spans a few tiles, so its centroid is
        // compared with the image of the centroid rather than with one vertex.
        Vector3 centre = new(MarkerCentreX, MarkerCentreY, MarkerDepth);
        Vec3 image = scene.Settings.Bend.Apply(new Vec3(centre.X, centre.Y, centre.Z));
        Vector2 expected = Project(view, new Vector3(image.X, image.Y, image.Z), gl.Width, gl.Height);

        Assert.True(
            Vector2.Distance(expected, after) < 6f,
            $"the shader put the marker at {after}, B says {expected}.");

        // A sanity check on the other side: without bending, the same projection of the plain
        // centre matches the first capture.
        Vector2 straight = Project(view, centre, gl.Width, gl.Height);
        Assert.True(
            Vector2.Distance(straight, before) < 6f,
            $"the flat marker is at {before}, the projection says {straight}.");
    }

    [GlFact] // REQ-SCN-068, REQ-SCN-045
    public void REQ_SCN_068_the_hud_is_left_flat()
    {
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = HudOnly();
        Capture flat = gl.Render(renderer, scene);

        scene.Settings.BendLeftRight = 180;
        scene.Settings.BendTopBottom = -90;
        scene.Settings.ArcHorizontal = 120;
        Capture bent = gl.Render(renderer, scene);

        // Nothing but the HUD is in the scene, so the whole image has to be identical.
        Assert.Equal(flat.Pixels, bent.Pixels);
    }

    [GlFact] // REQ-SCN-068
    public void REQ_SCN_068_the_shadow_pass_follows_the_bent_surface()
    {
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        SceneState scene = Marker();
        scene.Settings.Shadows = true;
        _ = gl.Render(renderer, scene);
        Assert.True(renderer.Statistics.ShadowPass, "the flat scene casts a shadow.");

        scene.Settings.BendLeftRight = 90;
        _ = gl.Render(renderer, scene);

        // The casters are still in the map: a volume fitted on the unbent anchors would have
        // left the bent geometry outside it and dropped the pass.
        Assert.True(renderer.Statistics.ShadowPass);
        Assert.True(renderer.Statistics.ShadowCasters > 0);
    }

    // ----------------------------------------------------------------- fixtures

    /// <summary>Scene x of the centre of the marker block.</summary>
    private const float MarkerCentreX = 216f;

    /// <summary>Scene y of the centre of the marker block.</summary>
    private const float MarkerCentreY = 128f;

    /// <summary>Scene z of the front face of the marker block.</summary>
    private const float MarkerDepth = 0f;

    /// <summary>
    /// A grey floor and a red block of 2 × 2 cells standing to the right of the frame centre,
    /// whose centre is <c>(216, 128, 0)</c>.
    /// </summary>
    private static SceneState Marker()
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(18, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(2), depth: 8, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), column: 0, row: 1, columns: 32, rows: 29);

        // Cells (26, 14) and (27, 14) and the row below: scene x 208 to 224, y 120 to 136.
        for (int row = 14; row < 16; row++)
        {
            for (int column = 26; column < 28; column++)
            {
                fixture.Sprite(SceneFixture.Solid(2), column * 8, row * 8, palette: 0);
            }
        }

        return fixture.Build();
    }

    /// <summary>A frame whose only geometry is on the HUD plane (REQ-SCN-045).</summary>
    private static SceneState HudOnly()
        => new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.Ui)
            .Rect(SceneFixture.Solid(1), column: 10, row: 10, columns: 6, rows: 6)
            .Build();

    /// <summary>The centroid of the reddish pixels, in pixel coordinates.</summary>
    private static Vector2 Centroid(Capture capture)
    {
        double sumX = 0d;
        double sumY = 0d;
        int found = 0;
        for (int y = 0; y < capture.Height; y++)
        {
            for (int x = 0; x < capture.Width; x++)
            {
                (int r, int g, int b) = capture.At(x, y);
                if (r > 80 && r > g * 2 && r > b * 2)
                {
                    sumX += x;
                    sumY += y;
                    found++;
                }
            }
        }

        Assert.True(found > 20, $"only {found} pixels of the marker were drawn.");
        return new Vector2((float)(sumX / found), (float)(sumY / found));
    }

    /// <summary>
    /// Where a scene point lands on the screen. The scene frame is left-handed, so the clip
    /// position is built from <c>(x, y, −z)</c> as the vertex shader does (REQ-SCN-001).
    /// </summary>
    private static Vector2 Project(in CameraView view, Vector3 point, int width, int height)
    {
        Vector4 clip = Vector4.Transform(
            new Vector3(point.X, point.Y, -point.Z),
            view.ViewProjection);
        float x = clip.X / clip.W;
        float y = clip.Y / clip.W;
        return new Vector2((x + 1f) * 0.5f * width, (1f - y) * 0.5f * height);
    }
}
