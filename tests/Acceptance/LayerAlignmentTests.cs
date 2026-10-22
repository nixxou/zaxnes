using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Acceptance.Tests;

/// <summary>
/// FIX-3: the depth order a script decides. "Zelda II" stores its L1 layer aligned on the back
/// and its <c>scene</c> script switches the side view to the front; the ground slab and the row
/// of grass that stands on it then rely on their own z offsets to stack in the right order
/// (REQ-E3D-205, REQ-PERS-004, REQ-PERS-007).
/// </summary>
/// <remarks>
/// No GPU: the whole ordering is decided by the depth interval of REQ-E3D-205 and the anchors it
/// feeds, which the conversion computes after the Lua hooks of each frame.
/// </remarks>
public sealed class LayerAlignmentTests
{
    /// <summary>The slab of "Zelda II": 2 × 1 tiles, 56 deep, pushed 16 units towards the eye.</summary>
    private const int SlabDepth = 56;

    /// <summary>The grass: 1 × 4 tiles, 8 deep, pushed 16 units away, pivoted on its front plane.</summary>
    private const int GrassDepth = 8;

    [Fact] // REQ-E3D-205, REQ-PERS-004, REQ-PERS-007
    public void A_scripted_layer_alignment_reorders_the_depths_of_two_z_offsets()
    {
        using LuaRun run = LuaRun.Create(GameProfile.CreateEmpty());

        EnginePattern slab = SyntheticPatterns.Build(run.Engine, ["AB"]);
        PatternInstance slabVariant = slab.Instances[0];
        slabVariant.Depth = SlabDepth;
        slabVariant.Offset = new Vec3(0f, 0f, -16f);

        EnginePattern grass = SyntheticPatterns.Build(run.Engine, ["C", "D", "E", "F"]);
        PatternInstance grassVariant = grass.Instances[0];
        grassVariant.Depth = GrassDepth;
        grassVariant.Offset = new Vec3(0f, 0f, 16f);
        grassVariant.Pivot = new Vec3(0.5f, 0f, 0f);

        // The value the profile stores: L1 aligned on the back (SPEC-09 §4.1.6, REQ-PERS-004
        // takes the number as well as the constant).
        run.Do("Setting.Layer[2].Aligned = 2");
        Assert.Equal(-1f, run.Num("Setting.Layer[2].Coef"), 3);

        (float slabFront, float grassFront) = Fronts(run, counter: 1);
        Assert.Equal(-72f, slabFront, 3);
        Assert.Equal(8f, grassFront, 3);

        // What the "scene" script does when the side view opens. REQ-PERS-007: the write is
        // visible to the very next read, so the frame converted after it is already placed with
        // the new coefficient.
        run.Do("Setting.Layer[2].Aligned = Align.FRONT");
        Assert.Equal(1f, run.Num("Setting.Layer[2].Coef"), 3);

        (slabFront, grassFront) = Fronts(run, counter: 2);

        // REQ-E3D-205 with the front coefficient: the interval starts at the layer offset, so
        // each object's own z offset alone decides where its front plane lands.
        Assert.Equal(-16f, slabFront, 3);
        Assert.Equal(16f, grassFront, 3);
        Assert.True(
            slabFront < grassFront,
            $"the slab's front plane ({slabFront}) is not in front of the grass's ({grassFront})");
        Assert.Equal(32f, grassFront - slabFront, 3);
    }

    /// <summary>
    /// Converts one frame holding the two objects and returns the front plane of each: the
    /// centre of REQ-E3D-205 less half the depth, which is where REQ-E3D-405 places the nearest
    /// face of the mesh.
    /// </summary>
    private static (float Slab, float Grass) Fronts(LuaRun run, int counter)
    {
        ConversionFrame frame = run.Frame(
            new SyntheticBatch()
                .Background('A', 16, 32)
                .Background('B', 24, 32)
                .Background('C', 64, 32)
                .Background('D', 64, 40)
                .Background('E', 64, 48)
                .Background('F', 64, 56),
            counter);

        Assert.Equal(2, frame.Instances.Count);
        FrameInstance slab = frame.Instances[0];
        FrameInstance grass = frame.Instances[1];
        Assert.Equal(SlabDepth, slab.Depth);
        Assert.Equal(GrassDepth, grass.Depth);

        return (slab.Center.Z - (slab.Depth / 2f), grass.Center.Z - (grass.Depth / 2f));
    }

    /// <summary>
    /// Q-069, the measurement the ticket rests on: a negative scale component is not an oddity
    /// of one profile. The ground slab of "Zelda II" (<c>p503</c>, scale z = −1) is one of them.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // Q-069
    public void Several_built_in_instances_carry_a_negative_scale()
    {
        List<string> carriers = [];
        int instances = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            GameProfile profile = ProfileCorpus.Load(path);
            int here = 0;
            foreach (ProfilePattern pattern in profile.PermanentPatterns)
            {
                foreach (PatternVariant variant in pattern.Variants)
                {
                    if (variant.Scale is { } scale && (scale.X < 0f || scale.Y < 0f || scale.Z < 0f))
                    {
                        here++;
                    }
                }
            }

            if (here > 0)
            {
                carriers.Add(Path.GetFileNameWithoutExtension(path));
                instances += here;
            }
        }

        // Q-069: 40 instances spread over 13 of the 101 built-in profiles.
        Assert.Equal(40, instances);
        Assert.Equal(13, carriers.Count);
        Assert.Contains(carriers, name => name.StartsWith("Zelda II", StringComparison.Ordinal));
    }
}
