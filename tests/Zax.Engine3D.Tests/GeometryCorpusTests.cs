using System.Diagnostics;
using System.Globalization;
using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// The geometry built for every 3D object of the 101 built-in profiles of
/// <c>assets-neutral/profiles</c>, checked against annex A of SPEC-04B.
/// </summary>
/// <remarks>
/// No ROM is needed, as DEC-R2-T7 asks of an acceptance criterion of the clean side: loading a
/// profile builds the geometry of all its patterns (REQ-E3D-600).
/// </remarks>
[Collection(AssetCollection.Name)]
public sealed class GeometryCorpusTests
{
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Annex_A_every_object_of_the_101_profiles_builds_and_the_counts_match()
    {
        Dictionary<VariantGeometry, int> objectsByGeometry = [];
        Dictionary<VariantGeometry, HashSet<string>> profilesByGeometry = [];
        Dictionary<int, int> depths = [];
        Dictionary<int, int> emptyColors = [];
        Dictionary<MeshAlgorithm, int> algorithms = [];
        Dictionary<ProfileLayer, int> layers = [];
        Dictionary<VariantRenderStyle, int> styles = [];

        int profiles = 0;
        int version2 = 0;
        int patterns = 0;
        int objects = 0;
        int solid = 0;
        int fullWarp = 0;
        int antiShadow = 0;
        int flip = 0;
        int indexedTextures = 0;
        int directTextures = 0;
        int resizedTextures = 0;
        int deformed = 0;
        int translucent = 0;
        int rotated = 0;
        int scaled = 0;
        int offset = 0;
        int noCast = 0;
        int noReceiv = 0;
        int noRecv = 0;
        int globalAxis = 0;
        int globalAxisText = 0;

        long triangles = 0;
        long vertices = 0;
        int built = 0;
        List<string> problems = [];
        Stopwatch watch = Stopwatch.StartNew();

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            string name = Path.GetFileNameWithoutExtension(path);
            GameProfile profile = NeutralAssets.LoadPath(path);
            profiles++;
            if (profile.IsV2Format)
            {
                version2++;
            }

            // Annex A counts the stored parameters, so they are read before the build: a
            // cylinder's sizeZ becomes D' once its geometry exists (REQ-E3D-452).
            foreach (ProfilePattern pattern in All(profile))
            {
                patterns++;
                foreach (PatternVariant variant in pattern.Variants)
                {
                    objects++;
                    Bump(objectsByGeometry, variant.Geometry);
                    if (!profilesByGeometry.TryGetValue(variant.Geometry, out HashSet<string>? seen))
                    {
                        seen = [];
                        profilesByGeometry[variant.Geometry] = seen;
                    }

                    seen.Add(name);

                    Bump(depths, variant.Depth);
                    Bump(emptyColors, variant.EmptyColor);
                    Bump(algorithms, variant.MeshAlgorithm);
                    Bump(layers, variant.Layer);
                    Bump(styles, variant.RenderStyle);

                    solid += variant.Solid ? 1 : 0;
                    fullWarp += variant.FullWarp ? 1 : 0;
                    antiShadow += variant.AntiShadow ? 1 : 0;
                    flip += variant.Flip ? 1 : 0;
                    deformed += IsSet(variant.DeformAmplitude) ? 1 : 0;
                    translucent += variant.Alpha < 1f ? 1 : 0;
                    rotated += IsSet(variant.Rotation) ? 1 : 0;
                    scaled += IsScaled(variant.Scale) ? 1 : 0;
                    offset += IsSet(variant.Offset) ? 1 : 0;

                    string tagString = variant.TagString ?? string.Empty;
                    if (tagString.Contains("_gaxis", StringComparison.Ordinal))
                    {
                        globalAxisText++;
                    }

                    foreach (string tag in ProfileTags.Parse(tagString))
                    {
                        noCast += string.Equals(tag, "!cast", StringComparison.Ordinal) ? 1 : 0;
                        noReceiv += string.Equals(tag, "!receiv", StringComparison.Ordinal) ? 1 : 0;
                        noRecv += string.Equals(tag, "!recv", StringComparison.Ordinal) ? 1 : 0;
                        globalAxis += string.Equals(tag, "_gaxis", StringComparison.Ordinal) ? 1 : 0;
                    }

                    if (variant.Texture is not { IsNullMarker: false } texture)
                    {
                        continue;
                    }

                    if (texture.Pixels?.Format == TextureFormat.Direct)
                    {
                        directTextures++;
                    }
                    else
                    {
                        indexedTextures++;
                    }

                    int cellsX = pattern.Grid?.Width ?? pattern.TileSize?.X ?? 0;
                    int cellsY = pattern.Grid?.Height ?? pattern.TileSize?.Y ?? 0;
                    if (texture.Width != 8 * cellsX || texture.Height != 8 * cellsY)
                    {
                        resizedTextures++;
                    }
                }
            }

            ConversionEngine engine = new();
            GeometryBuilder geometry = engine.UseGeometry();
            try
            {
                engine.LoadProfile(profile);
            }
            catch (Exception error)
            {
                problems.Add($"{name}: {error.GetType().Name} {error.Message}");
                continue;
            }

            if (geometry.Progress.Value < 1f)
            {
                problems.Add($"{name}: the build stopped at {geometry.Progress.Value:P0}");
            }

            foreach (EnginePattern pattern in engine.Patterns.Permanent.Concat(engine.Patterns.Temporary))
            {
                foreach (PatternInstance instance in pattern.Instances)
                {
                    ObjectGeometry? object3D = geometry.Of(instance);
                    if (object3D is null)
                    {
                        problems.Add($"{name}: an object has no geometry");
                        continue;
                    }

                    built++;
                    foreach (TileMesh mesh in object3D.Meshes)
                    {
                        triangles += mesh.TriangleCount;
                        vertices += mesh.VertexCount;
                    }
                }
            }

            engine.Unload();
        }

        watch.Stop();
        Assert.True(problems.Count == 0, string.Join("\n", problems.Take(20)));

        // Header of annex A.
        Assert.Equal(101, profiles);
        Assert.Equal(100, version2);
        Assert.Equal(67832, patterns);
        Assert.Equal(74389, objects);
        Assert.Equal(74389, built);

        // Geometry types: objects and profiles that use them.
        AssertGeometry(objectsByGeometry, profilesByGeometry, VariantGeometry.Default, 63633, 100);
        AssertGeometry(objectsByGeometry, profilesByGeometry, VariantGeometry.VerticalCylinder, 7121, 99);
        AssertGeometry(objectsByGeometry, profilesByGeometry, VariantGeometry.HorizontalCylinder, 2373, 87);
        AssertGeometry(objectsByGeometry, profilesByGeometry, VariantGeometry.Cube, 872, 64);
        AssertGeometry(objectsByGeometry, profilesByGeometry, VariantGeometry.HalfHorizontalCylinder, 246, 36);
        AssertGeometry(objectsByGeometry, profilesByGeometry, VariantGeometry.HalfVerticalCylinder, 144, 30);

        // Options.
        Assert.Equal(13475, solid);
        Assert.Equal(10518, fullWarp);
        Assert.Equal(2568, antiShadow);
        Assert.Equal(311, flip);

        // Mesh algorithm and transparent colour.
        Assert.Equal(73356, algorithms.GetValueOrDefault(MeshAlgorithm.FromSettings));
        Assert.Equal(400, algorithms.GetValueOrDefault(MeshAlgorithm.Greedy));
        Assert.Equal(633, algorithms.GetValueOrDefault(MeshAlgorithm.Marching));
        Assert.Equal(0, algorithms.GetValueOrDefault(MeshAlgorithm.GreedyMulticolor));

        Assert.Equal(62717, emptyColors.GetValueOrDefault(0));
        Assert.Equal(271, emptyColors.GetValueOrDefault(1));
        Assert.Equal(109, emptyColors.GetValueOrDefault(2));
        Assert.Equal(245, emptyColors.GetValueOrDefault(3));
        Assert.Equal(11047, emptyColors.GetValueOrDefault(4));

        // Textures, deformation, transparency, render style.
        Assert.Equal(13539, indexedTextures);
        Assert.Equal(142, directTextures);
        Assert.Equal(4, resizedTextures);
        Assert.Equal(289, deformed);
        Assert.Equal(628, translucent);
        Assert.Equal(1639, styles.GetValueOrDefault(VariantRenderStyle.Hidden));
        Assert.Equal(72744, styles.GetValueOrDefault(VariantRenderStyle.Normal));
        Assert.Equal(6, styles.GetValueOrDefault(VariantRenderStyle.ShadowOnly));

        // Layers and transforms.
        Assert.Equal(30762, layers.GetValueOrDefault(ProfileLayer.Unidentified));
        Assert.Equal(156, layers.GetValueOrDefault(ProfileLayer.Ui));
        Assert.Equal(32456, layers.GetValueOrDefault(ProfileLayer.L1));
        Assert.Equal(9534, layers.GetValueOrDefault(ProfileLayer.L2));
        Assert.Equal(1318, layers.GetValueOrDefault(ProfileLayer.L3));
        Assert.Equal(163, layers.GetValueOrDefault(ProfileLayer.L4));
        Assert.Equal(3248, rotated);
        Assert.Equal(784, scaled);
        Assert.Equal(15080, offset);

        // Tags. Annex A counts 42 objects whose tag string holds the text `_gaxis`, but one of
        // them — in "Road Fighter (Japan)" — stores `mc._gaxis`, a single tag that is not
        // `_gaxis`. REQ-E3D-407 asks for a tag of the comma-separated list, so 41 objects
        // really take the swapped rotation order (Q-018).
        Assert.Equal(980, noCast);
        Assert.Equal(160, noReceiv);
        Assert.Equal(406, noRecv);
        Assert.Equal(42, globalAxisText);
        Assert.Equal(41, globalAxis);

        // Depth distribution of annex A.
        Assert.Equal(51, depths.GetValueOrDefault(0));
        Assert.Equal(2007, depths.GetValueOrDefault(1));
        Assert.Equal(26398, depths.GetValueOrDefault(2));
        Assert.Equal(13084, depths.GetValueOrDefault(3));
        Assert.Equal(10589, depths.GetValueOrDefault(4));
        Assert.Equal(1974, depths.GetValueOrDefault(6));
        Assert.Equal(5895, depths.GetValueOrDefault(8));
        Assert.Equal(1499, depths.GetValueOrDefault(12));
        Assert.Equal(3224, depths.GetValueOrDefault(16));
        Assert.Equal(799, depths.GetValueOrDefault(24));
        Assert.Equal(1687, depths.GetValueOrDefault(32));
        Assert.Equal(2172, depths.GetValueOrDefault(48));
        Assert.Equal(96, Max(depths));

        Assert.True(triangles > 0);
        Assert.True(vertices > 0);

        // Reported, not asserted: the reference machine of SPEC-12 is not this one.
        Console.Out.WriteLine(string.Create(
            CultureInfo.InvariantCulture,
            $"annex A: {objects} objects, {triangles} triangles, {vertices} vertices in {watch.Elapsed.TotalSeconds:F1} s"));
    }

    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void REQ_E3D_564_the_biggest_built_in_textures_stay_within_the_capacity()
    {
        int widest = 0;
        int tallest = 0;
        long texels = 0;

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadPath(path);
            long profileTexels = 0;
            foreach (ProfilePattern pattern in All(profile))
            {
                foreach (PatternVariant variant in pattern.Variants)
                {
                    if (variant.Texture is not { IsNullMarker: false } texture)
                    {
                        continue;
                    }

                    widest = Math.Max(widest, texture.Width);
                    tallest = Math.Max(tallest, texture.Height);
                    profileTexels += (long)texture.Width * texture.Height;
                }
            }

            texels = Math.Max(texels, profileTexels);
        }

        // No object goes past 512 x 512 texels, and no profile past the simultaneous budget.
        Assert.True(widest <= 512, $"a texture is {widest} texels wide");
        Assert.True(tallest <= 512, $"a texture is {tallest} texels tall");
        Assert.True(texels <= 64L * 512 * 512, $"a profile needs {texels} texels at once");
    }

    private static IEnumerable<ProfilePattern> All(GameProfile profile)
        => profile.PermanentPatterns.Concat(profile.TemporaryPatterns);

    private static void Bump<T>(Dictionary<T, int> counts, T key)
        where T : notnull
        => counts[key] = counts.GetValueOrDefault(key) + 1;

    private static int Max(Dictionary<int, int> counts)
    {
        int highest = 0;
        foreach (int key in counts.Keys)
        {
            highest = Math.Max(highest, key);
        }

        return highest;
    }

    private static bool IsSet(Vec3F? value)
        => value is { } v && (v.X != 0f || v.Y != 0f || v.Z != 0f);

    private static bool IsScaled(Vec3F? value)
        => value is { } v && (v.X != 1f || v.Y != 1f || v.Z != 1f);

    private static void AssertGeometry(
        Dictionary<VariantGeometry, int> objects,
        Dictionary<VariantGeometry, HashSet<string>> profiles,
        VariantGeometry geometry,
        int expectedObjects,
        int expectedProfiles)
    {
        Assert.Equal(expectedObjects, objects.GetValueOrDefault(geometry));
        Assert.Equal(expectedProfiles, profiles.GetValueOrDefault(geometry)?.Count ?? 0);
    }
}
