// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>
/// The registry of patterns: the permanent set and the temporary set active during the session
/// (SPEC-04 §2, REQ-E3D-060 to 065).
/// </summary>
/// <remarks>
/// Lookup by key is a dictionary lookup and the candidates of an image are a list held by the
/// image itself, so both are constant time amortised as REQ-E3D-360 requires. Both sets keep
/// registration order; the candidate order of an image is kept by the image itself, where
/// AMD-021 R2-008 gives a newcomer the place of the last pattern removed from that image.
/// </remarks>
public sealed class PatternRegistry
{
    private readonly List<EnginePattern> _permanent = [];
    private readonly List<EnginePattern> _temporary = [];
    private readonly Dictionary<ImageSetKey, EnginePattern> _permanentByKey = [];
    private readonly Dictionary<ImageSetKey, EnginePattern> _temporaryByKey = [];
    private int _nextRegistrationIndex;

    /// <summary>The permanent patterns, in registration order.</summary>
    public IReadOnlyList<EnginePattern> Permanent => _permanent;

    /// <summary>The temporary patterns, in registration order.</summary>
    public IReadOnlyList<EnginePattern> Temporary => _temporary;

    /// <summary>
    /// How many patterns came from the profile. Patterns registered later were learned during
    /// play (REQ-E3D-161) and are the ones a savestate carries (DEC-R2-T5).
    /// </summary>
    public int ProfilePatternCount { get; private set; }

    /// <summary>
    /// Registers the patterns of a profile: every permanent one in file order, then every
    /// temporary one in file order (REQ-E3D-060).
    /// </summary>
    public void LoadProfile(GameProfile profile, ImageRegistry images)
    {
        ArgumentNullException.ThrowIfNull(profile);
        ArgumentNullException.ThrowIfNull(images);

        foreach (ProfilePattern pattern in profile.PermanentPatterns)
        {
            Register(EnginePattern.FromProfile(pattern, images, permanent: true));
        }

        foreach (ProfilePattern pattern in profile.TemporaryPatterns)
        {
            Register(EnginePattern.FromProfile(pattern, images, permanent: false));
        }

        ProfilePatternCount = _nextRegistrationIndex;
    }

    /// <summary>
    /// Registers a pattern <b>without</b> the replacement rules of REQ-E3D-065, which is what
    /// loading a profile does (REQ-E3D-060).
    /// </summary>
    /// <remarks>
    /// Q-011. REQ-E3D-060 requires every pattern of the file to be registered — its acceptance
    /// criterion counts 101 profiles out of 101 — while REQ-E3D-065 would drop any temporary
    /// pattern whose images contain those of a later one. The two cannot both hold:
    /// "Elevator Action" holds three such temporary patterns out of its 300. The file is
    /// therefore taken as it is and the replacement rules are kept for the patterns learned
    /// while playing, which is also what the deserialisation path of the earlier, public
    /// 3DNes_Engine does.
    /// </remarks>
    public void Register(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);

        pattern.RegistrationIndex = _nextRegistrationIndex++;
        if (pattern.Permanent)
        {
            _permanent.Add(pattern);
            _permanentByKey[pattern.Key] = pattern;
        }
        else
        {
            _temporary.Add(pattern);
            _temporaryByKey[pattern.Key] = pattern;
        }

        foreach (TileImage image in pattern.Images)
        {
            image.Attach(pattern);
        }
    }

    /// <summary>
    /// Adds a pattern, applying the replacement rules of REQ-E3D-065: a permanent pattern
    /// replaces the permanent one of the same key and swallows every temporary pattern whose
    /// images it contains; a temporary pattern replaces every temporary pattern that contains
    /// its images.
    /// </summary>
    public void Add(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);

        if (pattern.Permanent)
        {
            if (_permanentByKey.TryGetValue(pattern.Key, out EnginePattern? existing))
            {
                Remove(existing);
            }

            RemoveTemporaries(candidate => candidate.Key.IsSubsetOf(pattern.Key));
        }
        else
        {
            RemoveTemporaries(candidate => pattern.Key.IsSubsetOf(candidate.Key));
        }

        Register(pattern);
    }

    /// <summary>Takes a pattern out of the registry and out of its images' candidate lists.</summary>
    public void Remove(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);

        bool removed = pattern.Permanent
            ? _permanent.Remove(pattern)
            : _temporary.Remove(pattern);
        if (!removed)
        {
            return;
        }

        Dictionary<ImageSetKey, EnginePattern> byKey = pattern.Permanent ? _permanentByKey : _temporaryByKey;
        if (byKey.TryGetValue(pattern.Key, out EnginePattern? current) && ReferenceEquals(current, pattern))
        {
            byKey.Remove(pattern.Key);
        }

        foreach (TileImage image in pattern.Images)
        {
            image.Detach(pattern);
        }
    }

    /// <summary>Removes every permanent pattern that carries <paramref name="tag"/>.</summary>
    /// <remarks>
    /// Used by the script function <c>Pattern.RemoveWithTag</c> (SPEC-05); the candidate order
    /// of the survivors is unchanged, but the place each removal leaves free is remembered for
    /// the next pattern registered on that image (R2-008).
    /// </remarks>
    public int RemovePermanentWithTag(string tag)
    {
        ArgumentNullException.ThrowIfNull(tag);
        List<EnginePattern> doomed = [];
        foreach (EnginePattern pattern in _permanent)
        {
            foreach (PatternInstance instance in pattern.Instances)
            {
                if (instance.ContainsTag(tag))
                {
                    doomed.Add(pattern);
                    break;
                }
            }
        }

        foreach (EnginePattern pattern in doomed)
        {
            Remove(pattern);
        }

        return doomed.Count;
    }

    /// <summary>The permanent pattern of this key, or null.</summary>
    public EnginePattern? FindPermanent(ImageSetKey key)
        => _permanentByKey.TryGetValue(key, out EnginePattern? pattern) ? pattern : null;

    /// <summary>The temporary pattern of this key, or null.</summary>
    public EnginePattern? FindTemporary(ImageSetKey key)
        => _temporaryByKey.TryGetValue(key, out EnginePattern? pattern) ? pattern : null;

    /// <summary>
    /// The first temporary pattern, in registration order, whose images contain
    /// <paramref name="key"/> (REQ-E3D-160 point 2).
    /// </summary>
    public EnginePattern? FindTemporaryContaining(ImageSetKey key)
    {
        foreach (EnginePattern pattern in _temporary)
        {
            if (key.IsSubsetOf(pattern.Key))
            {
                return pattern;
            }
        }

        return null;
    }

    /// <summary>
    /// The first pattern instance that carries <paramref name="tag"/>, walking the permanent
    /// patterns in registration order then their instances by index (SPEC-05 REQ-LUA-085).
    /// </summary>
    public PatternInstance? FindInstanceWithTag(string tag)
    {
        ArgumentNullException.ThrowIfNull(tag);
        foreach (EnginePattern pattern in _permanent)
        {
            foreach (PatternInstance instance in pattern.Instances)
            {
                if (instance.ContainsTag(tag))
                {
                    return instance;
                }
            }
        }

        return null;
    }

    /// <summary>
    /// Every pattern instance that carries <paramref name="tag"/>, in the order of
    /// <see cref="FindInstanceWithTag"/> (SPEC-05 REQ-LUA-085).
    /// </summary>
    public void CollectInstancesWithTag(string tag, List<PatternInstance> destination)
    {
        ArgumentNullException.ThrowIfNull(tag);
        ArgumentNullException.ThrowIfNull(destination);
        destination.Clear();
        foreach (EnginePattern pattern in _permanent)
        {
            foreach (PatternInstance instance in pattern.Instances)
            {
                if (instance.ContainsTag(tag))
                {
                    destination.Add(instance);
                }
            }
        }
    }

    /// <summary>Empties the registry. Done when the game changes (REQ-E3D-008).</summary>
    public void Clear()
    {
        foreach (EnginePattern pattern in _permanent)
        {
            foreach (TileImage image in pattern.Images)
            {
                image.Detach(pattern);
            }
        }

        foreach (EnginePattern pattern in _temporary)
        {
            foreach (TileImage image in pattern.Images)
            {
                image.Detach(pattern);
            }
        }

        _permanent.Clear();
        _temporary.Clear();
        _permanentByKey.Clear();
        _temporaryByKey.Clear();
        _nextRegistrationIndex = 0;
        ProfilePatternCount = 0;
    }

    /// <summary>
    /// Drops everything learned during play: the patterns registered after the profile was
    /// loaded and every temporary pattern. Used before a savestate's learned state is put back
    /// (REQ-E3D-008).
    /// </summary>
    public void ClearLearned()
    {
        for (int i = _permanent.Count - 1; i >= 0; i--)
        {
            if (_permanent[i].RegistrationIndex >= ProfilePatternCount)
            {
                Remove(_permanent[i]);
            }
        }

        for (int i = _temporary.Count - 1; i >= 0; i--)
        {
            Remove(_temporary[i]);
        }

        _nextRegistrationIndex = ProfilePatternCount;
    }

    private void RemoveTemporaries(Func<EnginePattern, bool> doomed)
    {
        for (int i = _temporary.Count - 1; i >= 0; i--)
        {
            EnginePattern candidate = _temporary[i];
            if (doomed(candidate))
            {
                Remove(candidate);
            }
        }
    }
}
