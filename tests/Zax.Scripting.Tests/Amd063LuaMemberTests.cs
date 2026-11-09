using Zax.Engine3D;
using Zax.Profiles;
using Zax.Profiles.Hash;
using Zax.Profiles.Tests;

namespace Zax.Scripting.Tests;

/// <summary>
/// AMD-063 — the Lua members of 3dSen 2.1 that do not touch the camera: <c>Info.Version</c>
/// (REQ-LUA-118), <c>Pattern.HasPlaceholders</c> (REQ-LUA-120) and <c>Instance:IsSolidTile</c>
/// (REQ-LUA-121).
/// </summary>
public sealed class Amd063LuaMemberTests
{
    /// <summary>The raw bytes of the two tiles the « 3dh » profiles use; neither is blank.</summary>
    private static byte[] RawOne => RawTile.FromPlanes(0x0F0F_0F0F_0F0F_0F0FUL, 0x00FF_00FF_00FF_00FFUL);

    private static byte[] RawTwo => RawTile.FromPlanes(0x1234_5678_9ABC_DEF0UL, 0x0FED_CBA9_8765_4321UL);

    // ---------------------------------------------------------------- REQ-LUA-118

    /// <summary>REQ-LUA-118 — the criterion: <c>CS.Nes3D.Info.Version</c> reads "2.1".</summary>
    [Fact] // AMD-063, REQ-LUA-118
    public void The_version_of_the_info_table_is_two_point_one()
    {
        using ScriptSession session = Plain();
        Assert.Equal("2.1", session.Runtime.EvaluateText("return CS.Nes3D.Info.Version"));
    }

    /// <summary>
    /// REQ-LUA-118 — the criterion of AMD-040 stays true: the leading decimal integer of the
    /// version is at least 2, which is how a 2.0 script decides it may read the 2.0 members.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-118, AMD-040 REQ-LUA-112
    public void The_leading_integer_of_the_version_is_still_at_least_two()
    {
        using ScriptSession session = Plain();
        Assert.Equal(
            2d,
            session.Runtime.EvaluateNumber(
                "return tonumber(CS.Nes3D.Info.Version:match('^%d+'))"));
        Assert.Equal(
            1d,
            session.Runtime.EvaluateNumber(
                "if tonumber(CS.Nes3D.Info.Version:match('^%d+')) >= 2 then return 1 else return 0 end"));

        // The rest of the table is untouched (AMD-040 REQ-LUA-112).
        Assert.Equal("Zaxnes", session.Runtime.EvaluateText("return CS.Nes3D.Info.Title"));
    }

    // ---------------------------------------------------------------- REQ-LUA-121

    /// <summary>
    /// REQ-LUA-121 — the criterion: <c>shape:IsSolidTile(nil)</c> and
    /// <c>shape:IsSolidTile(3)</c> are both false, neither raises, and the script keeps its
    /// hooks (REQ-LUA-062).
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-121, REQ-LUA-062
    public void Is_solid_tile_is_always_false_and_never_raises()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("a", """
                seen = 0
                function UpdateS()
                  seen = seen + 1
                  kind = type(shape.IsSolidTile)
                  withNil = shape:IsSolidTile(nil)
                  withThree = shape:IsSolidTile(3)
                  withNothing = shape:IsSolidTile()
                  withTable = shape:IsSolidTile({})
                  withMany = shape:IsSolidTile(1, 'x', false)
                end
                """)
            .Start();

        session.Frame(new Batch().BgCell(Img.A, 10, 5));
        session.Frame(new Batch().BgCell(Img.A, 10, 5), counter: 1);

        Assert.Empty(session.Log.Errors);
        Assert.Equal("function", session.Runtime.EvaluateText("return kind"));

        // Two frames of UpdateS: the hooks are still alive after the calls.
        Assert.Equal(2d, session.Runtime.EvaluateNumber("return seen"));

        foreach (string name in (string[])
            ["withNil", "withThree", "withNothing", "withTable", "withMany"])
        {
            Assert.Equal("boolean", session.Runtime.EvaluateText($"return type({name})"));
            Assert.Equal(0d, session.Runtime.EvaluateNumber($"if {name} then return 1 else return 0 end"));
        }
    }

    /// <summary>
    /// REQ-LUA-121 — the method is on <c>Instance</c>, so a pattern instance reached through
    /// <c>Pattern.GetPatternWithTag</c> answers it too, outside any frame hook.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-121, REQ-LUA-085
    public void A_pattern_instance_answers_is_solid_tile_as_well()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("a", "-- nothing")
            .Start();

        Assert.Equal(
            0d,
            session.Runtime.EvaluateNumber(
                "if Pattern.GetPatternWithTag('t'):IsSolidTile(0) then return 1 else return 0 end"));
        Assert.Empty(session.Log.Errors);
    }

    // ---------------------------------------------------------------- REQ-LUA-120

    /// <summary>
    /// REQ-LUA-120 — the criterion: on a « 3dh » profile with one unresolved tile,
    /// <c>shape.refPattern.HasPlaceholders</c>, <c>shape.pattern.HasPlaceholders</c> and
    /// <c>Pattern.GetPatternWithTag(t).pattern.HasPlaceholders</c> are all true; once the tile
    /// is resolved they are all false.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-120, AMD-062 REQ-PROF-081, REQ-LUA-085
    public void A_pending_tile_of_a_hash_profile_shows_through_has_placeholders()
    {
        (GameProfile profile, HashProfileTiles tiles) = HashProfile();

        // The first tile is found, the second is not: the pattern keeps a pending cell.
        Assert.Equal(1, tiles.ResolveFromFrameTile(Planes(RawOne).Low, Planes(RawOne).High));
        Assert.Equal(1, tiles.PendingCount);

        using ScriptSession session = Session(profile, tiles);

        // The resolved cell alone recognises the pattern (AMD-062 REQ-PROF-081).
        session.Frame(new Batch().Sprite(Key(RawOne), 80, 80));

        Assert.Empty(session.Log.Errors);
        Assert.Equal(1d, session.Runtime.EvaluateNumber("return seen"));
        Assert.Equal("boolean", session.Runtime.EvaluateText("return type(onRef)"));
        Assert.True(Flag(session, "onRef"));
        Assert.True(Flag(session, "onFrame"));
        Assert.True(Flag(session, "onTag"));

        // The second tile appears on the frame: REQ-PROF-080 resolves it before that frame is
        // recognised, and the pattern no longer holds a placeholder.
        session.Frame(new Batch().Sprite(Key(RawOne), 80, 80).Sprite(Key(RawTwo), 96, 80), counter: 1);

        Assert.Equal(0, tiles.PendingCount);
        Assert.Empty(session.Log.Errors);
        Assert.False(Flag(session, "onRef"));
        Assert.False(Flag(session, "onFrame"));
        Assert.False(Flag(session, "onTag"));
    }

    /// <summary>
    /// REQ-LUA-120 — the criterion, last clause: with a 3dn profile the property is always
    /// false, through all three paths.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-120
    public void A_plain_profile_never_has_placeholders()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("a", Watcher)
            .Start();

        session.Frame(new Batch().BgCell(Img.A, 10, 5));

        Assert.Empty(session.Log.Errors);
        Assert.Equal(1d, session.Runtime.EvaluateNumber("return seen"));
        Assert.False(Flag(session, "onRef"));
        Assert.False(Flag(session, "onFrame"));
        Assert.False(Flag(session, "onTag"));
    }

    /// <summary>
    /// REQ-LUA-120 — the property is read-only: a write raises, as any unknown write on a
    /// script-visible type does (REQ-LUA-042), and it is not silently accepted.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-120, REQ-LUA-042
    public void Has_placeholders_cannot_be_written()
    {
        using ScriptSession session = new ScriptProfileBuilder()
            .Pattern([Img.A], tags: "t")
            .Script("a", "-- nothing")
            .Start();

        // The registry answers GetPatternWithTag once the patterns are registered, which is
        // after the chunks have run (REQ-LUA-015).
        session.Runtime.Evaluate("""
            ok, err = pcall(function ()
              Pattern.GetPatternWithTag('t').pattern.HasPlaceholders = true
            end)
            """);

        Assert.Equal(0d, session.Runtime.EvaluateNumber("if ok then return 1 else return 0 end"));
        Assert.EndsWith(
            "cannot set HasPlaceholders, it is read only",
            session.Runtime.EvaluateText("return err"),
            StringComparison.Ordinal);
    }

    /// <summary>
    /// The script the two <c>HasPlaceholders</c> criteria run: it reads the property through the
    /// three paths of the criterion on every <c>UpdateS</c>.
    /// </summary>
    private const string Watcher = """
        seen = 0
        function UpdateS()
          seen = seen + 1
          onRef = shape.refPattern.HasPlaceholders
          onFrame = shape.pattern.HasPlaceholders
          onTag = Pattern.GetPatternWithTag('t').pattern.HasPlaceholders
        end
        """;

    /// <summary>The truth value of a global the watcher set.</summary>
    private static bool Flag(ScriptSession session, string name)
        => session.Runtime.EvaluateNumber($"if {name} then return 1 else return 0 end") == 1d;

    /// <summary>A session on a one-pattern 3dn profile with an empty script.</summary>
    private static ScriptSession Plain()
        => new ScriptProfileBuilder().Script("a", string.Empty).Start();

    /// <summary>A started session on <paramref name="profile"/> and its tile table.</summary>
    private static ScriptSession Session(GameProfile profile, HashProfileTiles tiles)
    {
        ScriptSession session = ScriptSession.Create(profile, tiles);
        session.Runtime.InvokeStart();
        return session;
    }

    /// <summary>
    /// A « 3dh » profile with a 2 × 1 pattern whose two cells name <see cref="RawOne"/> and
    /// <see cref="RawTwo"/>, its variant tagged <c>t</c> and the watcher as its only script.
    /// </summary>
    private static (GameProfile Profile, HashProfileTiles Tiles) HashProfile()
    {
        TileId first = TileId.Compute(RawOne);
        TileId second = TileId.Compute(RawTwo);
        byte[] bytes = SyntheticHashProfile.Build(
            manifest: [first, second],
            tileKeys: [first.ToText(), second.ToText()]);

        ProfileLoadResult result = ProfileFormat.Load(bytes);
        Assert.True(result.IsSuccess, result.Message);
        ProfileDocument document = result.Document!;
        GameProfile profile = document.Profile;

        // The tag is what Pattern.GetPatternWithTag looks for (REQ-LUA-085); the synthetic
        // builder writes no tag, and a tag has nothing to do with the pending state.
        profile.PermanentPatterns[0].Variants[0].TagString = "t";
        profile.Scripts.Add(new ProfileScript("a", true, Watcher, "*"));
        return (profile, document.Tiles!);
    }

    private static (ulong Low, ulong High) Planes(byte[] raw) => RawTile.ToPlanes(raw);

    /// <summary>The tile image a raw tile shows on screen.</summary>
    private static TileImageKey Key(byte[] raw)
    {
        (ulong low, ulong high) = Planes(raw);
        return new TileImageKey(low, high, false, false);
    }
}
