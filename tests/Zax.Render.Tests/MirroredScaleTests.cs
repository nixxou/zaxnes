using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// FIX-3 (Q-069): an instance whose scale has a negative component is <b>mirrored</b>. The
/// profile of "Zelda II" scales its ground slab by −1 on z, and the renderer used to draw that
/// solid inside-out — the slab's visible surface jumped from the front of its depth interval to
/// the back of it, behind the grass that stands on it.
/// </summary>
/// <remarks>
/// A box whose pivot is centred maps exactly onto itself when it is mirrored, so the capture of
/// the mirrored object has to be the capture of the ordinary one, shading and shadow included
/// (REQ-SCN-028, REQ-SCN-038, SPEC-04B REQ-E3D-406).
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class MirroredScaleTests
{
    private const int Black = 0;

    [GlFact] // REQ-SCN-038, REQ-SCN-028, SPEC-04B REQ-E3D-406 (Q-069)
    public void A_mirrored_object_renders_the_same_solid_as_an_unmirrored_one()
    {
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        foreach (int depth in new[] { 8, 56 })
        {
            foreach (Vec3 scale in new[]
                     {
                         new Vec3(1f, 1f, -1f),
                         new Vec3(-1f, 1f, 1f),
                         new Vec3(-1f, -1f, -1f),
                     })
            {
                SceneFixture fixture = Fixture(depth);
                SceneState scene = fixture.Build();
                scene.Camera.Current.Angles = new Vector3(14f, 10f, 0f);
                scene.Camera.SnapToSetpoints();

                Capture plain = gl.Render(renderer, scene);
                Assert.True(plain.CountOther(Black) > 1000, "the object is not drawn");

                Instance(scene).Scale = scale;
                scene.DrawList = fixture.Engine!.BuildDrawList();
                Capture mirrored = gl.Render(renderer, scene);

                int differing = Differing(plain, mirrored);
                Assert.True(
                    differing == 0,
                    $"depth {depth} scaled by ({scale.X}, {scale.Y}, {scale.Z}): {differing} pixel(s) "
                        + "differ from the unmirrored solid — the object renders inside-out");
            }
        }
    }

    [GlFact] // REQ-E3D-205, REQ-SCN-038 (Q-069)
    public void A_mirrored_slab_stays_in_front_of_what_its_depth_interval_puts_behind_it()
    {
        // The arrangement of "Zelda II" in its side view: a deep ground slab mirrored on z, and a
        // shallow row of grass whose z offset puts it 32 units further away. The front face of
        // the slab is the nearest surface of the scene, so the pixels where the two overlap must
        // be the slab's colour and never the grass's.
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        // The grass is a sprite shape so that both objects can hold the same cells: a background
        // cell carries one tile only.
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Palette(17, 0x16)
            .PatternBlock(SceneFixture.Solid(3), 4, 4, depth: 56, ProfileLayer.L1)
            .PatternBlock(SceneFixture.Solid(1, id: 1), 4, 4, depth: 8, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 16, columns: 4, rows: 4);

        for (int row = 0; row < 4; row++)
        {
            for (int column = 0; column < 4; column++)
            {
                fixture.Sprite(SceneFixture.Solid(1, id: 1), (14 + column) * 8, (16 + row) * 8);
            }
        }

        SceneState scene = fixture.Build();
        List<FrameInstance> objects = Instances(scene);
        Assert.Equal(2, objects.Count);

        FrameInstance slab = objects[0];
        FrameInstance grass = objects[1];
        slab.Offset = new Vec3(0f, 0f, -16f);
        slab.Scale = new Vec3(1f, 1f, -1f);
        grass.Offset = new Vec3(0f, 0f, 16f);

        ConversionSettings settings = fixture.Engine!.Settings;
        settings.Layer(ZLayer.L1).Alignment = ZAlignment.Front;
        slab.CalculatePos(settings);
        grass.CalculatePos(settings);

        // REQ-E3D-205 with the FRONT coefficient: the slab's front plane is 32 units nearer than
        // the grass's, whatever the mirroring does to the mesh inside that interval.
        Assert.Equal(-16f, slab.Center.Z - (slab.Depth / 2f), 3);
        Assert.Equal(16f, grass.Center.Z - (grass.Depth / 2f), 3);

        // Shadows off: the only question here is which surface wins the depth test, and the
        // grass would otherwise cast on the slab whether or not it is itself visible.
        scene.Settings.Shadows = false;
        fixture.Engine.Settings.Shadow = false;

        grass.Style = RenderStyle.Hidden;
        scene.DrawList = fixture.Engine.BuildDrawList();
        Capture alone = gl.Render(renderer, scene);
        Assert.True(alone.CountOther(Black) > 1000, "the slab is not drawn");

        grass.Style = RenderStyle.Normal;
        scene.DrawList = fixture.Engine.BuildDrawList();
        Capture both = gl.Render(renderer, scene);

        int differing = Differing(alone, both);
        Assert.True(
            differing == 0,
            $"{differing} pixel(s) of the object 32 units behind the slab show through it — the "
                + "mirrored slab does not present its front face");
    }

    private static SceneFixture Fixture(int depth)
        => new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .PatternBlock(SceneFixture.Solid(3), 4, 4, depth, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 4, rows: 4);

    private static FrameInstance Instance(SceneState scene) => Instances(scene)[0];

    private static List<FrameInstance> Instances(SceneState scene)
    {
        List<FrameInstance> instances = [];
        foreach (DrawTile tile in scene.DrawList!.Tiles)
        {
            if (!instances.Contains(tile.Instance))
            {
                instances.Add(tile.Instance);
            }
        }

        return instances;
    }

    /// <summary>Pixels more than two levels apart, the tolerance the other capture tests use.</summary>
    private static int Differing(Capture left, Capture right)
    {
        int differing = 0;
        for (int y = 0; y < left.Height; y++)
        {
            for (int x = 0; x < left.Width; x++)
            {
                (int lr, int lg, int lb) = left.At(x, y);
                (int rr, int rg, int rb) = right.At(x, y);
                if (Math.Abs(lr - rr) > 2 || Math.Abs(lg - rg) > 2 || Math.Abs(lb - rb) > 2)
                {
                    differing++;
                }
            }
        }

        return differing;
    }
}
