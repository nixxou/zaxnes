namespace Zax.Profiles.Hash;

/// <summary>
/// The tile table of a « 3dh » profile: which manifest strings are still waiting for their
/// sixteen bytes, where they are found, and what has to be rebuilt once they are
/// (AMD-062 REQ-PROF-078 to 083).
/// </summary>
/// <remarks>
/// <para>
/// Resolution is <b>monotone</b> and is session knowledge: a tile that has been resolved never
/// goes back to pending, not on a rollback, a run-ahead, a state load or a reset, and the set of
/// resolved tiles belongs neither to the savestates nor to the rollback snapshots
/// (REQ-PROF-080). It follows that determinism is guaranteed at equal initial resolved set,
/// which is why the determinism tests run with an empty cache.
/// </para>
/// <para>
/// The scan is <b>stateless</b>: nothing is remembered from one frame to the next, so the set
/// resolved on a frame is exactly the one a full walk would give. The prefix of REQ-PROF-077 is
/// the fast filter that makes the full walk affordable — a window is hashed only when its prefix
/// is one the manifest holds.
/// </para>
/// </remarks>
public sealed class HashProfileTiles
{
    private readonly HashProfileManifest _manifest;
    private readonly Dictionary<string, List<ProfileTile>> _tilesByKey = new(StringComparer.Ordinal);
    private readonly Dictionary<string, List<int>> _patternsByKey = new(StringComparer.Ordinal);
    private readonly Dictionary<string, byte[]> _resolved = new(StringComparer.Ordinal);
    private readonly List<ProfilePattern> _patterns = [];
    private readonly bool[] _changed;
    private int _pendingLegacyDigests;

    /// <summary>
    /// Builds the table of a decoded profile. Every pattern of the file is listed — the
    /// permanent ones, the temporary ones and the patterns of the editor's memorised slots — so
    /// that a resolution knows everything it has to rebuild.
    /// </summary>
    public HashProfileTiles(GameProfile profile, HashProfileManifest manifest)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ArgumentNullException.ThrowIfNull(manifest);

        _manifest = manifest;
        foreach (ProfilePattern pattern in EnumeratePatterns(profile))
        {
            _patterns.Add(pattern);
        }

        _changed = new bool[_patterns.Count];
        for (int index = 0; index < _patterns.Count; index++)
        {
            Index(_patterns[index], index);
        }

        foreach (string key in manifest.Keys)
        {
            if (!HexText.IsIdentifierText(key))
            {
                _pendingLegacyDigests++;
            }
        }
    }

    /// <summary>The manifest this table resolves against.</summary>
    public HashProfileManifest Manifest => _manifest;

    /// <summary>Number of manifest strings, resolved or not.</summary>
    public int TotalCount => _manifest.Count;

    /// <summary>Number of manifest strings whose sixteen bytes are known.</summary>
    public int ResolvedCount => _resolved.Count;

    /// <summary>Number of manifest strings still waiting.</summary>
    public int PendingCount => TotalCount - ResolvedCount;

    /// <summary>True when nothing is waiting any more, which lets every scan stop.</summary>
    public bool IsFullyResolved => PendingCount == 0;

    /// <summary>The manifest strings still waiting, in manifest order.</summary>
    public IReadOnlyList<string> PendingKeys
    {
        get
        {
            List<string> pending = [];
            foreach (string key in _manifest.Keys)
            {
                if (!_resolved.ContainsKey(key))
                {
                    pending.Add(key);
                }
            }

            return pending;
        }
    }

    /// <summary>The sixteen bytes found for each resolved manifest string.</summary>
    public IReadOnlyDictionary<string, byte[]> Resolved => _resolved;

    /// <summary>The patterns of the profile, in the order the table lists them.</summary>
    public IReadOnlyList<ProfilePattern> Patterns => _patterns;

    /// <summary>True when the profile holds at least one tile that carries a manifest string.</summary>
    public bool HasHashTiles => _tilesByKey.Count > 0;

    /// <summary>
    /// Gives a manifest string its sixteen bytes, whatever their provenance, after checking that
    /// they really are the bytes of that string — which is what makes a cache entry verifiable
    /// (REQ-PROF-082). Returns false when the string is unknown, already resolved, or does not
    /// match.
    /// </summary>
    public bool Apply(string key, ReadOnlySpan<byte> raw)
    {
        if (raw.Length != RawTile.ByteCount || !_manifest.Contains(key) || _resolved.ContainsKey(key))
        {
            return false;
        }

        bool matches = HexText.IsIdentifierText(key)
            ? string.Equals(TileId.Compute(raw).ToText(), key, StringComparison.Ordinal)
            : string.Equals(RawTile.LegacyDigest(raw), key, StringComparison.Ordinal);
        return matches && Resolve(key, raw);
    }

    /// <summary>
    /// What a host does when a « 3dh » profile is loaded for a ROM, before the first converted
    /// frame: apply what the cache kept (REQ-PROF-082), then look through the ROM
    /// (REQ-PROF-078). Keeping what the game later resolves is a separate, explicit
    /// <see cref="ResolvedTileCache.Save"/>.
    /// </summary>
    public void PrepareForRom(string? profileId, ReadOnlySpan<byte> romImage, ResolvedTileCache? cache)
    {
        cache?.Apply(profileId, this);
        ResolveFromRom(romImage);
    }

    /// <summary>
    /// Resolves what the ROM holds (REQ-PROF-078): the PRG zone then the CHR zone, at every byte
    /// position, the four mirrorings of each window. A file that is not iNES resolves nothing.
    /// </summary>
    /// <returns>How many manifest strings this call resolved.</returns>
    public int ResolveFromRom(ReadOnlySpan<byte> romFile)
    {
        if (IsFullyResolved)
        {
            return 0;
        }

        RomTileZones zones = RomTileZones.Of(romFile);
        int count = ScanEveryPosition(romFile.Slice(zones.PrgStart, zones.PrgLength));
        count += ScanEveryPosition(romFile.Slice(zones.ChrStart, zones.ChrLength));
        return count;
    }

    /// <summary>
    /// Resolves what the CHR-RAM holds (REQ-PROF-080): every 16-byte aligned block, the four
    /// mirrorings of each. Nothing is remembered from one call to the next.
    /// </summary>
    public int ResolveFromChrRam(ReadOnlySpan<byte> chrRam)
    {
        int count = 0;
        for (int offset = 0; offset + RawTile.ByteCount <= chrRam.Length && !IsFullyResolved;
             offset += RawTile.ByteCount)
        {
            count += ScanWindow(chrRam.Slice(offset, RawTile.ByteCount));
        }

        return count;
    }

    /// <summary>
    /// Resolves one tile of the displayed frame, given by its planes in clear and not mirrored
    /// (REQ-PROF-080).
    /// </summary>
    public int ResolveFromFrameTile(ulong lowPlane, ulong highPlane)
    {
        if (IsFullyResolved)
        {
            return 0;
        }

        Span<byte> raw = stackalloc byte[RawTile.ByteCount];
        RawTile.FromPlanes(lowPlane, highPlane, raw);
        return ScanWindow(raw);
    }

    /// <summary>
    /// The patterns a resolution changed since the last call, in file order, and forgets them.
    /// Their geometry and their textures have to be rebuilt before the frame is drawn
    /// (REQ-PROF-080, REQ-PROF-081).
    /// </summary>
    public IReadOnlyList<ProfilePattern> TakeChangedPatterns()
    {
        List<ProfilePattern> changed = [];
        for (int index = 0; index < _changed.Length; index++)
        {
            if (_changed[index])
            {
                changed.Add(_patterns[index]);
                _changed[index] = false;
            }
        }

        return changed;
    }

    /// <summary>Rebuilds the textures of every pattern (REQ-PROF-083).</summary>
    public void RebuildAllTextures()
    {
        foreach (ProfilePattern pattern in _patterns)
        {
            RebuildTextures(pattern);
        }
    }

    /// <summary>
    /// Rebuilds the texture of each variant of <paramref name="pattern"/> from its
    /// <c>textureKind</c> (REQ-PROF-083). A variant with no <c>textureKind</c> — a 3dn variant,
    /// or one of a memorised slot — is left alone.
    /// </summary>
    public static void RebuildTextures(ProfilePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);

        VariantTexture? defaultTexture = null;
        foreach (PatternVariant variant in pattern.Variants)
        {
            if (variant.TextureKind is not { } kind)
            {
                continue;
            }

            switch (kind)
            {
                case 0:
                    variant.Texture = null;
                    break;
                case 1:
                    variant.Texture = VariantTexture.NullMarker();
                    break;
                default:
                    defaultTexture ??= PatternDefaultTexture.Build(pattern);
                    variant.Texture = ReconstructXor(pattern, variant.XorTexture!, defaultTexture);
                    break;
            }
        }
    }

    private static VariantTexture ReconstructXor(
        ProfilePattern pattern, VariantTexture stored, VariantTexture defaultTexture)
    {
        // REQ-PROF-083: while a tile of the pattern waits and the shape of the stored texture
        // differs from the default one, undoing the exclusive or would mix unrelated arrays, so
        // the variant falls back on the default texture.
        if (pattern.Grid is { HasPendingTile: true }
            && !PatternDefaultTexture.MatchesDefaultShape(stored, defaultTexture))
        {
            return VariantTexture.NullMarker();
        }

        return PatternDefaultTexture.UndoXor(stored, defaultTexture);
    }

    private static IEnumerable<ProfilePattern> EnumeratePatterns(GameProfile profile)
    {
        foreach (ProfilePattern pattern in profile.PermanentPatterns)
        {
            yield return pattern;
        }

        foreach (ProfilePattern pattern in profile.TemporaryPatterns)
        {
            yield return pattern;
        }

        if (profile.Settings is not { } settings)
        {
            yield break;
        }

        foreach (RecordedPatternSlot slot in settings.SavedPatterns)
        {
            if (slot.Pattern is { } pattern)
            {
                yield return pattern;
            }
        }

        foreach (RecordedPatternSlot slot in settings.SavedAnimPatterns)
        {
            if (slot.Pattern is { } pattern)
            {
                yield return pattern;
            }
        }
    }

    private void Index(ProfilePattern pattern, int patternIndex)
    {
        if (pattern.Grid is not { } grid)
        {
            return;
        }

        foreach (ProfileTile? cell in grid.Cells)
        {
            if (cell?.Hash is not { } hash)
            {
                continue;
            }

            if (!_tilesByKey.TryGetValue(hash.Text, out List<ProfileTile>? tiles))
            {
                tiles = [];
                _tilesByKey[hash.Text] = tiles;
            }

            bool known = false;
            foreach (ProfileTile tile in tiles)
            {
                if (ReferenceEquals(tile, cell))
                {
                    known = true;
                    break;
                }
            }

            if (!known)
            {
                tiles.Add(cell);
            }

            if (!_patternsByKey.TryGetValue(hash.Text, out List<int>? indices))
            {
                indices = [];
                _patternsByKey[hash.Text] = indices;
            }

            if (!indices.Contains(patternIndex))
            {
                indices.Add(patternIndex);
            }
        }
    }

    private int ScanEveryPosition(ReadOnlySpan<byte> zone)
    {
        int count = 0;
        for (int offset = 0; offset + RawTile.ByteCount <= zone.Length && !IsFullyResolved; offset++)
        {
            count += ScanWindow(zone.Slice(offset, RawTile.ByteCount));
        }

        return count;
    }

    private int ScanWindow(ReadOnlySpan<byte> window)
    {
        int count = 0;
        Span<byte> flipped = stackalloc byte[RawTile.ByteCount];
        for (int flip = 0; flip < 4; flip++)
        {
            RawTile.Flip(window, (TileFlip)flip, flipped);
            count += TryResolveRaw(flipped);
        }

        return count;
    }

    private int TryResolveRaw(ReadOnlySpan<byte> raw)
    {
        int count = 0;
        (ulong low, ulong high) = RawTile.ToPlanes(raw);
        uint prefix = TileId.ComputePrefix(low, high);
        if (_manifest.HasPrefix(prefix))
        {
            string text = TileId.Compute(raw, prefix).ToText();
            if (_manifest.Contains(text) && Resolve(text, raw))
            {
                count++;
            }
        }

        // A legacy digest has no prefix to filter on, so it costs one SHA-256 per window; no
        // built-in profile carries one (REQ-PROF-072, notes).
        if (_pendingLegacyDigests > 0)
        {
            string digest = RawTile.LegacyDigest(raw);
            if (_manifest.Contains(digest) && Resolve(digest, raw))
            {
                count++;
            }
        }

        return count;
    }

    private bool Resolve(string key, ReadOnlySpan<byte> raw)
    {
        if (_resolved.ContainsKey(key))
        {
            return false;
        }

        byte[] bytes = raw.ToArray();
        _resolved[key] = bytes;
        if (!HexText.IsIdentifierText(key))
        {
            _pendingLegacyDigests--;
        }

        (ulong low, ulong high) = RawTile.ToPlanes(bytes);
        if (_tilesByKey.TryGetValue(key, out List<ProfileTile>? tiles))
        {
            foreach (ProfileTile tile in tiles)
            {
                tile.Resolve(low, high);
            }
        }

        if (_patternsByKey.TryGetValue(key, out List<int>? indices))
        {
            foreach (int index in indices)
            {
                _changed[index] = true;
            }
        }

        return true;
    }
}
