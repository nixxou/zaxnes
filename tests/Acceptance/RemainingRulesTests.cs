using Zax.Engine3D;
using Zax.Profiles;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The part-A requirements not covered by the other files: the 2D mode, the deformation,
/// the direct texture, the clipping contract, the catalogue and the user profile folder.
/// </summary>
public sealed class RemainingRulesTests
{
    // ------------------------------------------------------------------ SPEC-04 §3.17

    [Fact] // REQ-E3D-381 (the normative pass-2 rule)
    public void In_two_dimensional_mode_pass_two_keeps_one_tile_per_image()
    {
        ConversionEngine engine = new() { TwoDimensionalMode = true };

        SyntheticBatch batch = new() { MaskX = 0, MaskY = 0 };
        for (int column = 0; column < 8; column++)
        {
            batch.Background(column % 2 == 0 ? 'A' : 'B', 8 * (10 + column), 8 * 5);
        }

        ConversionFrame frame = batch.Convert(engine);

        // The shape grown in pass 2 carries each image exactly once: two tiles, not the
        // whole wall. (The tiles it leaves behind become pass-3 business once the pattern
        // has been learned; see Q-041.)
        FrameShape first = frame.Shapes[0];
        Assert.Equal(2, first.TileCount);
        Assert.Equal(2, first.Images.Count);
        Assert.All(frame.Shapes, s => Assert.Equal(s.Images.Count, s.Images.Distinct().Count()));

        // In 3D mode pass 2 makes a one-tile shape instead: the growth of REQ-E3D-381 is
        // specific to 2D mode.
        ConversionEngine normal = new();
        SyntheticBatch same = new() { MaskX = 0, MaskY = 0 };
        for (int column = 0; column < 8; column++)
        {
            same.Background(column % 2 == 0 ? 'A' : 'B', 8 * (10 + column), 8 * 5);
        }

        ConversionFrame plain = same.Convert(normal);
        Assert.Equal(1, plain.Shapes[0].TileCount);
    }

    [Fact] // REQ-E3D-381 (pass S1 does the same for sprites)
    public void In_two_dimensional_mode_pass_s1_keeps_one_sprite_per_image()
    {
        ConversionEngine engine = new() { TwoDimensionalMode = true };

        SyntheticBatch batch = new() { MaskX = 0, MaskY = 0 };
        for (int i = 0; i < 6; i++)
        {
            batch.Sprite(i % 2 == 0 ? 'A' : 'B', 100 + (8 * i), 100);
        }

        ConversionFrame frame = batch.Convert(engine);
        FrameShape first = frame.Shapes[0];
        Assert.Equal(2, first.TileCount);
        Assert.Equal(2, first.Images.Count);
    }

    // ------------------------------------------------------------------ SPEC-04B

    [Fact] // REQ-E3D-568, TV-D1
    public void Tvd1_animated_deformation()
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();

        // A 16 x 16 pixel shape whose top-left corner is (0, 0): a 2 x 2 pattern of full
        // tiles, placed at the top-left of the screen.
        List<TileImage?> cells = [];
        for (int i = 0; i < 4; i++)
        {
            cells.Add(engine.Images.Intern(0xFFFFFFFFFFFFFFFFUL, 0x0101010101010101UL * (ulong)i, false, false));
        }

        EnginePattern pattern = new(2, 2, cells, permanent: true);
        pattern.AddInstance(new PatternInstance
        {
            Depth = 4,
            Layer = ZLayer.L1,
            Scale = new Vec3(1, 1, 1),
            Pivot = new Vec3(0.5f, 0.5f, 0.5f),
            DeformPivot = new Vec3(0.5f, 0.5f, 0.5f),
            DeformAmplitude = new Vec3(0, 0.1f, 0),
            DeformSpeed = SpeedWithTwoPi(),
            DeformOffset = DefaultPhases(),
            Alpha = 1,
            Style = RenderStyle.Normal,
            CustomPalette = -1,
            TexTilting = new Vec2(1, 1),
            FillRate = 1,
        });

        engine.Patterns.Register(pattern);
        geometry.Build(pattern);
        geometry.Commit();

        SyntheticBatch batch = new() { MaskX = 0, MaskY = 0 };
        for (int j = 0; j < 2; j++)
        {
            for (int i = 0; i < 2; i++)
            {
                (ulong low, ulong high) = (0xFFFFFFFFFFFFFFFFUL, 0x0101010101010101UL * (ulong)((j * 2) + i));
                batch.BackgroundPlanes(low, high, 8 * i, 8 * j);
            }
        }

        ConversionFrame frame = batch.Convert(engine, 0);
        engine.Publish();
        FrameDrawList list = engine.BuildDrawList(frame);

        DrawTile tile = list.Tiles[0];
        Assert.NotNull(tile.Deform);
        DeformParameters deform = tile.Deform!.Value;

        Assert.Equal(new Vec3(16, 16, 4), deform.Size);

        // delta_y = 0.1 * 16 * sin(2 pi nrm.x + pi/2); nrm.x = 0 -> +1.6, 0.25 -> 0,
        // 0.5 -> -1.6. nrm.x is (TileRelativeToPivot + v).x / 16.
        foreach ((float normalisedX, float expected) in new[] { (0f, 1.6f), (0.25f, 0f), (0.5f, -1.6f) })
        {
            float vx = (normalisedX * 16f) - deform.TileRelativeToPivot.X;
            Vec3 displacement = deform.Displacement(new Vec3(vx, -deform.TileRelativeToPivot.Y, -deform.TileRelativeToPivot.Z), 0f);
            Assert.Equal(expected, displacement.Y, 3);
            Assert.Equal(0f, displacement.X, 4);
            Assert.Equal(0f, displacement.Z, 4);
        }
    }

    [Fact] // REQ-E3D-568 (the default matrices give a constant translation)
    public void Default_deformation_matrices_give_a_constant_translation()
    {
        DeformParameters deform = new(
            new Vec3(1, 2, 3), default, DefaultPhases(), new Vec3(8, 8, 4), new Vec3(0, 0, 0));

        Vec3 displacement = deform.Displacement(new Vec3(3, 4, 1), 1.5f);
        Assert.Equal(8f, displacement.X, 3);
        Assert.Equal(16f, displacement.Y, 3);
        Assert.Equal(12f, displacement.Z, 3);
    }

    private static Mat4 DefaultPhases()
    {
        Mat4 phases = default;
        for (int row = 0; row < 3; row++)
        {
            for (int column = 0; column < 4; column++)
            {
                phases[row, column] = MathF.PI / 2f;
            }
        }

        return phases;
    }

    private static Mat4 SpeedWithTwoPi()
    {
        Mat4 speed = default;
        speed[1, 0] = 2f * MathF.PI;
        return speed;
    }

    [Fact] // REQ-E3D-562
    public void A_direct_texture_keeps_its_own_colours()
    {
        ConversionEngine engine = new();
        TileImage image = engine.Images.Intern(0xFFFFFFFFFFFFFFFFUL, 0, false, false);
        EnginePattern pattern = new(1, 1, [image], permanent: true);

        int[] pixels = new int[64];
        Array.Fill(pixels, 0x00FF0000);
        pixels[0] = 0; // the one texel that must take the universal background colour

        VariantTexture direct = new(8, 8, new TexturePixels(TextureFormat.Direct, pixels));
        pattern.AddInstance(new PatternInstance
        {
            Depth = 1,
            Alpha = 1,
            Style = RenderStyle.Normal,
            CustomPalette = -1,
            Scale = new Vec3(1, 1, 1),
            Pivot = new Vec3(0.5f, 0.5f, 0.5f),
            DeformPivot = new Vec3(0.5f, 0.5f, 0.5f),
            TexTilting = new Vec2(1, 1),
            FillRate = 1,
            Texture = direct,
        });

        engine.Patterns.Register(pattern);
        GeometryBuilder geometry = engine.UseGeometry();
        geometry.BackgroundColor = new Rgb24(0x38, 0xC0, 0xFC);
        geometry.Build(pattern);
        geometry.Commit();

        ObjectTexture texture = geometry.Of(pattern.Instances[0])!.Texture;
        Assert.True(texture.IsDirect);
        Assert.Equal(new Rgb24(0xFF, 0x00, 0x00), texture.Color(1, 0));
        Assert.Equal(new Rgb24(0x38, 0xC0, 0xFC), texture.Color(0, 0));
    }

    [Fact] // REQ-E3D-643
    public void The_clipping_margins_do_not_change_the_meshes_or_the_transforms()
    {
        static (int Tiles, string Transforms) Run(Vec4F clipping)
        {
            ConversionEngine engine = new();
            GeometryBuilder geometry = engine.UseGeometry();
            engine.Settings.Clipping = clipping;

            EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);
            geometry.Build(pattern);
            geometry.Commit();

            ConversionFrame frame = new SyntheticBatch().Background('A', 64, 64).Convert(engine);
            engine.Publish();
            FrameDrawList list = engine.BuildDrawList(frame);

            return (
                list.Tiles.Count,
                string.Join(';', list.Tiles.Select(t => $"{t.Anchor.X},{t.Anchor.Y},{t.Anchor.Z}")));
        }

        (int tilesA, string transformsA) = Run(new Vec4F(0.1f, 8.1f, 8.1f, 0.1f));
        (int tilesB, string transformsB) = Run(new Vec4F(16f, 16f, 16f, 16f));

        Assert.Equal(tilesA, tilesB);
        Assert.Equal(transformsA, transformsB);
    }

    [Fact] // REQ-E3D-454
    public void Shape_density_classes()
    {
        // 2 x 1 cells, one occupied, 30 non-zero pixels: tile density 2, pixel density 1.
        Assert.Equal(2, (int)Math.Ceiling(4.0 * 1 / (2 * 1)));
        Assert.Equal(1, (int)Math.Ceiling(4.0 * 30 / (64 * 2 * 1)));

        ConversionEngine engine = new();
        ulong low = 0;
        for (int i = 0; i < 30; i++)
        {
            low |= 1UL << i;
        }

        TileImage image = engine.Images.Intern(low, 0, false, false);
        Assert.Equal(30, image.PixelCount);
        Assert.Equal(ImageDensity.Average, image.Density);
    }

    // ------------------------------------------------------------------ SPEC-03 §3.9

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-060
    public void The_catalogue_exposes_the_101_games_of_the_full_edition()
    {
        ProfileCatalogue catalogue = ProfileCatalogue.Parse(
            File.ReadAllText(NeutralAssets.CataloguePath), NeutralAssets.Root);

        Assert.Equal(101, catalogue.Entries.Count);
        Assert.Equal(101, catalogue.Ids.Count);

        CatalogueEntry arkanoid = catalogue.Find("1")!;
        Assert.Equal("Arkanoid", arkanoid.Title);
        Assert.True(File.Exists(catalogue.ProfilePath(arkanoid)));

        foreach (CatalogueEntry entry in catalogue.Entries)
        {
            Assert.True(File.Exists(catalogue.ProfilePath(entry)), entry.Title);
            Assert.True(catalogue.LoadProfile(PhysicalProfileFileSystem.Instance, entry).IsSuccess, entry.Title);
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-061
    public void The_2d_mode_entry_is_recognised_by_its_title()
    {
        ProfileCatalogue catalogue = ProfileCatalogue.Parse(
            File.ReadAllText(NeutralAssets.CataloguePath), NeutralAssets.Root);

        CatalogueEntry twoD = catalogue.TwoDimensionalMode!;
        Assert.NotNull(twoD);
        Assert.Equal("47", twoD.Id);
        Assert.True(twoD.Is2DMode);
        Assert.EndsWith("empty.bytes", twoD.File, StringComparison.Ordinal);

        // Its profile carries no 3D pattern at all.
        GameProfile profile = catalogue.LoadProfile(PhysicalProfileFileSystem.Instance, twoD).Profile!;
        Assert.Empty(profile.PermanentPatterns);
        Assert.Empty(profile.TemporaryPatterns);
    }

    [Fact] // REQ-PROF-062
    public void The_user_profile_folder_honours_ids_titles_and_duplicates()
    {
        string root = Path.Combine(Path.GetTempPath(), "zax-profiles-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            byte[] withId = WithHeader(id: "abc123def456", title: null);
            File.WriteAllBytes(Path.Combine(root, "Foo.3dn"), withId);
            File.WriteAllBytes(Path.Combine(root, "Zoo.3dn"), withId); // same id: ignored
            File.WriteAllBytes(Path.Combine(root, "Clash.3dn"), WithHeader(id: "1", title: "Clash"));
            File.WriteAllBytes(Path.Combine(root, "Bad.3dn"), [0x00, 0x01, 0x02, 0x03]);

            UserProfileFolder folder = new(PhysicalProfileFileSystem.Instance, root);
            IReadOnlyList<UserProfile> scanned = folder.Scan(reservedIds: ["1"]);

            UserProfile foo = Assert.Single(scanned);
            Assert.Equal("abc123def456", foo.Id);
            Assert.Equal("Foo", foo.Title); // no Title field: the file name without extension
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }

    [Fact] // REQ-PROF-063
    public void Importing_a_profile_without_an_id_is_refused()
    {
        string root = Path.Combine(Path.GetTempPath(), "zax-import-" + Guid.NewGuid().ToString("N"));
        string source = Path.Combine(Path.GetTempPath(), "zax-src-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        Directory.CreateDirectory(source);
        try
        {
            string noId = Path.Combine(source, "NoId.3dn");
            File.WriteAllBytes(noId, WithHeader(id: null, title: "No identifier"));

            UserProfileFolder folder = new(PhysicalProfileFileSystem.Instance, root);
            ProfileImportResult refused = folder.Import(noId);
            Assert.Equal(ProfileImportOutcome.Rejected, refused.Outcome);

            string good = Path.Combine(source, "Good.3dn");
            File.WriteAllBytes(good, WithHeader(id: "zzz111yyy222", title: "Good"));
            Assert.Equal(ProfileImportOutcome.Added, folder.Import(good).Outcome);

            // Importing the same file twice does not create a second game.
            folder.Import(good);
            Assert.Single(folder.Scan());
        }
        finally
        {
            Directory.Delete(root, recursive: true);
            Directory.Delete(source, recursive: true);
        }
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-064, REQ-PROF-065
    public void The_library_remembers_a_rom_path_per_profile_and_only_deletes_user_games()
    {
        ProfileCatalogue catalogue = ProfileCatalogue.Parse(
            File.ReadAllText(NeutralAssets.CataloguePath), NeutralAssets.Root);

        GameLibrary library = GameLibrary.Build(catalogue);

        GameLibraryEntry arkanoid = library.Entries.Single(e => e.Id == "1");
        Assert.False(arkanoid.HasRom);
        Assert.False(arkanoid.CanDelete);

        // Any ROM can be attached to any profile: no identity check whatsoever.
        arkanoid.RomPath = @"C:\roms\minimal.nes";
        Assert.True(arkanoid.HasRom);
        Assert.Equal(@"C:\roms\minimal.nes", library.Entries.Single(e => e.Id == "1").RomPath);
    }

    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-PROF-041
    public void Loading_a_profile_replaces_the_listed_settings_and_keeps_the_others()
    {
        GameProfile profile = ProfileFormat.Load(NeutralAssets.ReadProfile("empty.bytes")).Profile!;
        IReadOnlyDictionary<string, object?> applied = ProfileSettingApplication.ValuesToApply(profile.Settings!);

        // Replaced by the profile.
        Assert.Equal(new Vec3F(1f, 0.2f, 1f), Assert.IsType<Vec3F>(applied["light.direction"]));
        Assert.Equal(2, Assert.IsType<int>(applied["video.graphic_quality"]));
        Assert.Equal(4.1f, Assert.IsType<float>(applied["clip.left"]), 4);
        Assert.Equal(16.1f, Assert.IsType<float>(applied["clip.bottom"]), 4);
        Assert.Equal(4.1f, Assert.IsType<float>(applied["clip.right"]), 4);
        Assert.Equal(0.1f, Assert.IsType<float>(applied["clip.top"]), 4);

        // scene.use_nes_bg_color is forced to true whatever the file holds.
        Assert.True(Assert.IsType<bool>(applied["scene.use_nes_bg_color"]));

        // Kept from the user: the light intensity and the volume are not in the applied set.
        Assert.DoesNotContain("light.intensity", applied.Keys);
        Assert.DoesNotContain("audio.volume", applied.Keys);
        Assert.Contains("light.intensity", ProfileSettingApplication.PreservedKeys);
        Assert.Contains("audio.volume", ProfileSettingApplication.PreservedKeys);

        // The two sets do not overlap.
        Assert.Empty(ProfileSettingApplication.AppliedKeys
            .Intersect(ProfileSettingApplication.PreservedKeys, StringComparer.Ordinal));
    }

    [Fact] // REQ-PROF-052
    public void Exporting_a_new_profile_normalises_its_header()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = null;
        profile.Title = string.Empty;

        ProfileExport.Normalize(profile, romFileName: "Minimal.nes");

        Assert.Equal("3dn", profile.Signature);
        Assert.Equal("3", profile.VersionText);
        Assert.Equal(12, profile.Id!.Length);
        Assert.All(profile.Id, c => Assert.True(
            char.IsAsciiLetterOrDigit(c) || c is '+' or '/',
            $"'{c}' is not in the base64 alphabet"));
        Assert.Equal("Minimal.nes", profile.Title);

        byte[] written = ProfileFormat.Save(profile);
        GameProfile reread = ProfileFormat.Load(written).Profile!;
        Assert.Equal(profile.Id, reread.Id);
        Assert.Equal(profile.PermanentPatterns.Count, reread.PermanentPatterns.Count);
    }

    /// <summary>A minimal, valid profile carrying only the header fields of REQ-PROF-062.</summary>
    private static byte[] WithHeader(string? id, string? title)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        return ProfileFormat.Save(profile);
    }

}
