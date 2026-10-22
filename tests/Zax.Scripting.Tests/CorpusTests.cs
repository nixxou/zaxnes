using System.Globalization;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Scripting.Tests;

/// <summary>
/// The checks that run on the 579 scripts of the 101 built-in profiles of
/// <c>assets-neutral/profiles</c>: the script model of REQ-LUA-001 to 004, the compilation of
/// REQ-LUA-100, the API coverage of REQ-LUA-051 and the acceptance run of REQ-ACC-011.
/// </summary>
[Collection(AssetCollection.Name)]
public sealed class CorpusTests
{
    /// <summary>REQ-LUA-001: the catalogue holds 579 scripts in 89 of the 101 profiles.</summary>
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Corpus_shape_matches_REQ_LUA_001()
    {
        int profiles = 0;
        int withScripts = 0;
        int scripts = 0;
        int enabled = 0;

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadPath(path);
            profiles++;
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            withScripts++;
            scripts += profile.Scripts.Count;
            foreach (ProfileScript script in profile.Scripts)
            {
                if (script.Enabled)
                {
                    enabled++;
                }
            }
        }

        Assert.Equal(101, profiles);
        Assert.Equal(89, withScripts);
        Assert.Equal(579, scripts);
        Assert.Equal(556, enabled);
        Assert.Equal(23, scripts - enabled);
    }

    /// <summary>
    /// REQ-LUA-100: the 579 scripts compile under the chosen Lua 5.4 runtime, with no change,
    /// including the two profiles that use <c>goto</c> and labels.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Every_script_compiles_REQ_LUA_100()
    {
        List<string> failures = [];
        int compiled = 0;

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadPath(path);
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            using ScriptSession session = ScriptSession.Create(profile);
            compiled += profile.Scripts.Count;
            foreach (string error in session.Log.Errors)
            {
                failures.Add($"{Path.GetFileName(path)}: {error}");
            }
        }

        Assert.Equal(579, compiled);
        Assert.Empty(failures);
    }

    /// <summary>
    /// REQ-LUA-004: for each of the 89 profiles that carry scripts, sorting the names with the
    /// collation of REQ-LUA-004 gives back the order of the file.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Script_order_reproduces_the_file_order_REQ_LUA_004()
    {
        List<string> mismatches = [];

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadPath(path);
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            string[] inFile = [.. profile.Scripts.Select(static script => script.Name ?? string.Empty)];
            string[] sorted = [.. inFile];
            Array.Sort(sorted, ScriptNameComparer.Instance);
            if (!inFile.SequenceEqual(sorted, StringComparer.Ordinal))
            {
                mismatches.Add($"{Path.GetFileName(path)}: [{string.Join(", ", inFile)}]");
            }
        }

        Assert.Empty(mismatches);
    }

    /// <summary>
    /// REQ-LUA-051: a static resolution of every access chain of the 579 scripts, starting at
    /// the globals of §4.1, finds no missing member and no wrong <c>.</c>/<c>:</c> separator.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Every_member_access_of_the_corpus_resolves_REQ_LUA_051()
    {
        ScriptApiSurface api = ScriptApiSurface.Describe();
        LuaAccessResolver resolver = new(api);

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadPath(path);
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            resolver.Check(
                Path.GetFileNameWithoutExtension(path),
                profile.Scripts.Select(static script =>
                    (script.Name ?? string.Empty, script.Source ?? string.Empty)));
        }

        Assert.True(
            resolver.Unresolved.Count == 0,
            "members the corpus reads that the API does not declare:\n"
                + string.Join("\n", resolver.Unresolved.Select(static f => f.ToString()).Distinct()));
        Assert.True(
            resolver.WrongSeparator.Count == 0,
            "calls made with the wrong '.'/':' separator:\n"
                + string.Join("\n", resolver.WrongSeparator.Select(static f => f.ToString()).Distinct()));

        // The corpus has to exercise a good part of the surface; a resolver that silently gave
        // up on everything would otherwise pass.
        Assert.True(
            resolver.ResolvedCouples.Count >= 150,
            $"only {resolver.ResolvedCouples.Count} (type, member) couples resolved");
    }

    /// <summary>
    /// REQ-ACC-011: for each of the 89 profiles that carry scripts, <c>Start</c>, then 600
    /// frames of <c>Update</c>, <c>UpdateS</c> and <c>LateUpdate</c> on a synthetic frame, then
    /// <c>End</c>, produce no Lua error caused by a missing API member.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Acceptance_run_of_the_89_profiles_REQ_ACC_011()
    {
        const int frames = 600;
        List<string> missingMember = [];
        List<string> dataCaused = [];
        int clean = 0;
        int total = 0;

        foreach (string path in NeutralAssets.ProfileFiles())
        {
            GameProfile profile = NeutralAssets.LoadPath(path);
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            total++;
            string name = Path.GetFileNameWithoutExtension(path);
            using ScriptSession session = ScriptSession.Create(profile);
            Batch batch = SyntheticFrame(profile);

            session.Runtime.InvokeStart();
            for (int frame = 1; frame <= frames; frame++)
            {
                batch.ConvertAndPublish(session.Engine, frame);
            }

            session.Runtime.InvokeEnd();

            if (session.Log.Errors.Count == 0)
            {
                clean++;
                continue;
            }

            foreach (string error in session.Log.Errors)
            {
                // Only the binding layer produces these two, so they are the signature of an
                // API member that is absent or that refused its arguments.
                if (error.Contains("no such field", StringComparison.Ordinal)
                    || error.Contains("invalid arguments to", StringComparison.Ordinal)
                    || error.Contains("it is read only", StringComparison.Ordinal))
                {
                    missingMember.Add($"{name}: {error}");
                }
                else
                {
                    dataCaused.Add($"{name}: {error}");
                }
            }
        }

        Assert.Equal(89, total);
        Assert.True(
            missingMember.Count == 0,
            $"{missingMember.Count} errors point at the API surface:\n"
                + string.Join("\n", missingMember));

        // An error the synthetic frame itself causes is tolerated when it is documented
        // (REQ-ACC-011 acceptance criterion). Any other one fails the test.
        List<string> undocumented = [.. dataCaused.Where(static error =>
            !DocumentedFrameDataErrors.Any(known => error.Contains(known, StringComparison.Ordinal)))];
        Assert.True(
            undocumented.Count == 0,
            $"{clean}/{total} profiles ran without any error; undocumented errors:\n"
                + string.Join("\n", undocumented));
        Assert.Equal(total - DocumentedFrameDataErrors.Length, clean);
    }

    /// <summary>
    /// The errors the synthetic frame of this test provokes, which REQ-ACC-011 tolerates once
    /// documented. They come from the frame's contents, not from the API surface.
    /// </summary>
    /// <remarks>
    /// "punchout", script <c>zmac_punch</c>: it reads the global <c>opY</c>, which the script
    /// of the opponent sets in its own <c>UpdateS</c>. The synthetic frame has no shape with
    /// the opponent's tag, so that script never runs and <c>opY</c> stays nil
    /// (<c>math.min(nil, 76)</c>). A real game frame always carries both shapes.
    /// </remarks>
    private static readonly string[] DocumentedFrameDataErrors =
    [
        "punchout: Lua: script 'zmac_punch' hook UpdateS raised: attempt to compare number with nil",
    ];

    /// <summary>
    /// REQ-LUA-100 and the two <c>goto</c> profiles of §5.3: Burger Time and Jackal load and
    /// run their hooks.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Profiles_that_use_goto_load_and_run()
    {
        foreach (string prefix in (string[])["Burger", "Jackal"])
        {
            GameProfile? profile = NeutralAssets.LoadNamed(prefix);
            Assert.NotNull(profile);
            Assert.Contains(
                profile.Scripts,
                script => (script.Source ?? string.Empty).Contains("goto", StringComparison.Ordinal));

            using ScriptSession session = ScriptSession.Create(profile);
            Assert.Empty(session.Log.Errors);

            session.Runtime.InvokeStart();
            Batch batch = SyntheticFrame(profile);
            for (int frame = 1; frame <= 8; frame++)
            {
                batch.ConvertAndPublish(session.Engine, frame);
            }

            session.Runtime.InvokeEnd();
            Assert.DoesNotContain(
                session.Log.Errors,
                static error => error.Contains("no such field", StringComparison.Ordinal)
                    || error.Contains("invalid arguments to", StringComparison.Ordinal));
        }
    }

    /// <summary>
    /// REQ-LUA-004: the Excitebike order, where the underscore sorts before the digit, is the
    /// one the collation produces and the one the hooks follow.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [AssetFact]
    public void Excitebike_hook_order_REQ_LUA_004()
    {
        GameProfile? profile = NeutralAssets.LoadNamed("Excitebike");
        Assert.NotNull(profile);

        using ScriptSession session = ScriptSession.Create(profile);
        string[] order = [.. session.Runtime.Scripts.Select(static script => script.Name)];
        Assert.Equal(["bike", "camera", "ground", "ramp_rotation", "ramp7", "scene"], order);
    }

    /// <summary>
    /// A synthetic frame built from the profile's own patterns, so that the shapes the
    /// segmentation recognises carry the tags the scripts are linked to (REQ-ACC-011).
    /// </summary>
    internal static Batch SyntheticFrame(GameProfile profile)
    {
        ArgumentNullException.ThrowIfNull(profile);

        Batch batch = new();
        for (int i = 0; i < 32; i++)
        {
            // A palette with no two equal entries, so that a colour read is meaningful.
            batch.Palette(i, (byte)((i * 3) % 64));
        }

        int column = 0;
        int row = 0;
        int rowHeight = 0;
        int sprites = 0;

        foreach (ProfilePattern pattern in profile.PermanentPatterns)
        {
            PatternGrid? grid = pattern.Grid;
            if (grid is null || grid.Width <= 0 || grid.Height <= 0)
            {
                continue;
            }

            if (grid.Width > 30 || grid.Height > 28)
            {
                continue;
            }

            if (column + grid.Width > 30)
            {
                column = 0;
                row += Math.Max(rowHeight, 1);
                rowHeight = 0;
            }

            if (row + grid.Height > 28)
            {
                break;
            }

            for (int j = 0; j < grid.Height; j++)
            {
                for (int i = 0; i < grid.Width; i++)
                {
                    ProfileTile? tile = grid[i, j];
                    if (tile is null)
                    {
                        continue;
                    }

                    TileImageKey key = new(
                        tile.LowPlane,
                        tile.HighPlane,
                        tile.FlipX,
                        tile.FlipY);
                    if (key.IsEmpty)
                    {
                        continue;
                    }

                    batch.BgCell(key, column + i, row + j, (i + j) & 3);

                    // A few sprite tiles as well, so that sprite shapes exist too.
                    if (sprites < 48 && i == 0 && j == 0)
                    {
                        batch.Sprite(key, 8 + ((sprites * 5) % 232), 16 + ((sprites * 7) % 200), 1);
                        sprites++;
                    }
                }
            }

            column += grid.Width;
            rowHeight = Math.Max(rowHeight, grid.Height);
        }

        return batch;
    }

    /// <summary>
    /// The API description the runtime produces is complete enough to be useful: every global
    /// of §4.1 names a declared type, and the surface has the order of magnitude §5.1 reports.
    /// </summary>
    [Fact]
    public void Api_description_is_consistent_REQ_LUA_080()
    {
        ScriptApiSurface api = ScriptApiSurface.Describe();

        foreach ((string global, string type) in api.Globals)
        {
            Assert.True(
                type.Length == 0 || api.Types.ContainsKey(type),
                $"global {global} names the undeclared type {type}");
        }

        foreach (ScriptApiType type in api.Types.Values)
        {
            foreach (ScriptApiMember member in type.Members.Values)
            {
                Assert.True(
                    member.ResultType.Length == 0 || api.Types.ContainsKey(member.ResultType),
                    $"{type.Name}.{member.Name} yields the undeclared type {member.ResultType}");
            }
        }

        Assert.True(
            api.MemberCount >= 370,
            string.Create(CultureInfo.InvariantCulture, $"only {api.MemberCount} members declared"));
        Assert.Equal(3, api.Constants.Count);
    }
}
