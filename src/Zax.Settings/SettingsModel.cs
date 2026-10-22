namespace Zax.Settings;

/// <summary>Which layer of the model a write goes to (REQ-PERS-005, REQ-PERS-010).</summary>
public enum SettingLayer
{
    /// <summary>
    /// The user's own choice, from the options window, the menus or a libretro core option. For
    /// a U setting this is the preference that gets persisted; for a P setting it changes the
    /// live scene value, and for an S setting the session value.
    /// </summary>
    User,

    /// <summary>
    /// The value a profile brought in. Only P settings have one, and it is replaced wholesale
    /// when a different game loads (REQ-PERS-005).
    /// </summary>
    Profile,

    /// <summary>
    /// A script's write. On a U setting it is a <b>session</b> value that masks the preference
    /// until the game ends and is never persisted (DEC-09-Q3, REQ-PERS-010).
    /// </summary>
    Session,

    /// <summary>
    /// A value an <b>explicitly given</b> command-line option asks for, for this run only. It
    /// masks the persisted preference without replacing it and is never written back; the
    /// user's own change — from the options window or a menu — takes over and is persisted as
    /// usual (Q-066).
    /// </summary>
    CommandLine,
}

/// <summary>One effective change of a setting (REQ-PERS-009).</summary>
/// <param name="Key">Stable key of the setting that changed.</param>
/// <param name="Value">Its new effective value.</param>
/// <param name="Effect">When the change takes effect (REQ-PERS-084).</param>
public readonly record struct SettingChange(string Key, SettingValue Value, SettingEffect Effect);

/// <summary>
/// The settings of one emulator instance: the catalogue of SPEC-09 §4.1 with a value for each
/// key, layered defaults → user preferences → profile values → session writes, validated on
/// every write and notified when the effective value really changes.
/// </summary>
/// <remarks>
/// <para>
/// REQ-PERS-082 governs the shape of this class: no dependency on the UI, the window or the
/// file system, and no global state — each instance owns its values, so two models in the same
/// process are independent. Persistence lives in <see cref="SettingsFile"/>, which turns the
/// user layer into JSON and back through an <see cref="ISettingsStorage"/> the host supplies.
/// </para>
/// <para>
/// Resolution order, by scope:
/// </para>
/// <list type="bullet">
/// <item><description>U — session value if a script wrote one, else a command-line override, else the user preference, else the default;</description></item>
/// <item><description>P — the live profile value if there is one, else the default;</description></item>
/// <item><description>S — the session value if one was set, else the default;</description></item>
/// <item><description>D — computed on every read (REQ-PERS-013).</description></item>
/// </list>
/// </remarks>
public sealed class SettingsModel
{
    private readonly Dictionary<string, SettingValue> _user = new(StringComparer.Ordinal);
    private readonly Dictionary<string, SettingValue> _profile = new(StringComparer.Ordinal);
    private readonly Dictionary<string, SettingValue> _session = new(StringComparer.Ordinal);
    private readonly Dictionary<string, SettingValue> _command = new(StringComparer.Ordinal);

    private int _coreCount = 1;
    private float _displayRefreshRate = 60f;

    /// <summary>Creates a model in which every setting holds its default (REQ-PERS-002).</summary>
    public SettingsModel()
    {
    }

    /// <summary>
    /// Raised once per effective change, with the key and the moment of effect, so that the
    /// renderer, the loop, the engine and the audio apply it when they should (REQ-PERS-009).
    /// A write of the value already in force raises nothing.
    /// </summary>
    public event Action<SettingChange>? Changed;

    /// <summary>
    /// Number of available cores, at least 1; feeds the validation of <c>emu.core</c>
    /// (REQ-PERS-003 rule 2).
    /// </summary>
    public int CoreCount
    {
        get => _coreCount;
        set => _coreCount = Math.Max(1, value);
    }

    /// <summary>
    /// Refresh rate of the display, which <c>video.target_framerate</c> uses when vsync is on
    /// (REQ-PERS-013 point 1).
    /// </summary>
    public float DisplayRefreshRate
    {
        get => _displayRefreshRate;
        set
        {
            if (_displayRefreshRate.Equals(value))
            {
                return;
            }

            _displayRefreshRate = value;
            RaiseDerived(SettingKeys.VideoTargetFramerate);
            RaiseDerived(SettingKeys.VideoTargetTimePerFrame);
        }
    }

    /// <summary>
    /// The region timing block <c>emu.tv_data</c> hands back (REQ-PERS-013 point 7). The core
    /// owns its shape, so the model only carries it.
    /// </summary>
    public object? TvData { get; set; }

    /// <summary>The effective value of <paramref name="key"/>.</summary>
    /// <exception cref="KeyNotFoundException">No setting carries that key.</exception>
    public SettingValue Get(string key)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        return Read(descriptor);
    }

    /// <summary>The effective value of <paramref name="descriptor"/>.</summary>
    public SettingValue Get(SettingDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        return Read(descriptor);
    }

    /// <summary>The effective value as a boolean.</summary>
    public bool GetBool(string key) => Get(key).AsBool;

    /// <summary>The effective value as a whole number.</summary>
    public int GetInt(string key) => Get(key).AsInt;

    /// <summary>The effective value as a real.</summary>
    public float GetFloat(string key) => Get(key).AsFloat;

    /// <summary>The effective value as a colour.</summary>
    public SettingColor GetColor(string key) => Get(key).AsColor;

    /// <summary>The effective value as a vector.</summary>
    public SettingVector3 GetVector3(string key) => Get(key).AsVector3;

    /// <summary>The effective value as text.</summary>
    public string GetString(string key) => Get(key).AsString;

    /// <summary>The effective value as a rectangle.</summary>
    public SettingRect GetRect(string key) => Get(key).AsRect;

    /// <summary>
    /// The value <b>stored</b> for <paramref name="key"/>, before the read-time rules of
    /// REQ-PERS-011 apply. For <c>vr.scene_ratio</c> this is the value a script wrote, which the
    /// model keeps although reading gives 1.
    /// </summary>
    public SettingValue GetStored(string key)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        return ReadStored(descriptor);
    }

    /// <summary>Whether a script's session value is masking the preference for this key.</summary>
    public bool HasSessionValue(string key) => _session.ContainsKey(key);

    /// <summary>
    /// Whether a command-line override is masking the stored value for this key (Q-066). It is
    /// what tells the persistence layer that the value in force is not the user's own choice and
    /// must not be written back.
    /// </summary>
    public bool HasCommandLineValue(string key) => _command.ContainsKey(key);

    /// <summary>
    /// The user preferences to persist: every U setting whose stored value differs from its
    /// default is written, so a file stays as small as what the user actually changed
    /// (REQ-PERS-020).
    /// </summary>
    public IEnumerable<KeyValuePair<string, SettingValue>> UserPreferences()
    {
        foreach (SettingDescriptor descriptor in SettingsCatalogue.All)
        {
            if (!descriptor.IsPersisted)
            {
                continue;
            }

            if (_user.TryGetValue(descriptor.Key, out SettingValue stored) && stored != descriptor.DefaultValue)
            {
                yield return new KeyValuePair<string, SettingValue>(descriptor.Key, stored);
            }
        }
    }

    /// <summary>
    /// Writes <paramref name="value"/> to <paramref name="key"/> and reports whether the
    /// effective value changed. The value is validated first (REQ-PERS-003) and the change is
    /// notified once (REQ-PERS-009).
    /// </summary>
    /// <param name="key">Stable key.</param>
    /// <param name="value">The value to write; it is reshaped to the setting's kind.</param>
    /// <param name="layer">Which layer the write belongs to.</param>
    public bool Set(string key, SettingValue value, SettingLayer layer = SettingLayer.User)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        return Write(descriptor, value, layer);
    }

    /// <summary>Writes a boolean.</summary>
    public bool SetBool(string key, bool value, SettingLayer layer = SettingLayer.User)
        => Set(key, SettingValue.OfBool(value), layer);

    /// <summary>Writes a whole number.</summary>
    public bool SetInt(string key, int value, SettingLayer layer = SettingLayer.User)
        => Set(key, SettingValue.OfInt(value), layer);

    /// <summary>Writes a real.</summary>
    public bool SetFloat(string key, float value, SettingLayer layer = SettingLayer.User)
        => Set(key, SettingValue.OfFloat(value), layer);

    /// <summary>Writes a colour.</summary>
    public bool SetColor(string key, SettingColor value, SettingLayer layer = SettingLayer.User)
        => Set(key, SettingValue.OfColor(value), layer);

    /// <summary>Writes a vector.</summary>
    public bool SetVector3(string key, SettingVector3 value, SettingLayer layer = SettingLayer.User)
        => Set(key, SettingValue.OfVector3(value), layer);

    /// <summary>Writes text.</summary>
    public bool SetString(string key, string? value, SettingLayer layer = SettingLayer.User)
        => Set(key, SettingValue.OfString(value), layer);

    /// <summary>Writes a rectangle.</summary>
    public bool SetRect(string key, SettingRect value, SettingLayer layer = SettingLayer.User)
        => Set(key, SettingValue.OfRect(value), layer);

    /// <summary>
    /// Takes <paramref name="key"/> back to its default, in the layer that owns it. Used by the
    /// "Default values" button of the options window (AMD-010 REQ-UI-202).
    /// </summary>
    public bool Reset(string key)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        if (descriptor.Scope == SettingScope.Derived)
        {
            return false;
        }

        SettingValue before = Read(descriptor);
        _user.Remove(descriptor.Key);
        _session.Remove(descriptor.Key);
        _command.Remove(descriptor.Key);
        if (descriptor.Scope == SettingScope.Profile)
        {
            _profile.Remove(descriptor.Key);
        }

        return NotifyIfChanged(descriptor, before);
    }

    /// <summary>Takes every setting of <paramref name="theme"/> back to its default.</summary>
    public void ResetTheme(SettingTheme theme)
    {
        foreach (SettingDescriptor descriptor in SettingsCatalogue.OfTheme(theme))
        {
            Reset(descriptor.Key);
        }
    }

    /// <summary>
    /// Applies the settings a profile embeds, for a game <b>different</b> from the previous one
    /// (REQ-PERS-005). Relaunching the same game must not call this (REQ-PERS-006).
    /// </summary>
    /// <param name="values">
    /// The values to apply, keyed by stable key; SPEC-03 REQ-PROF-041 fixes the list, and only
    /// P settings are taken into account.
    /// </param>
    /// <remarks>
    /// Beyond the file's own values, the three rules the specification adds are applied here:
    /// <c>scene.use_nes_bg_color</c> is forced true, the six pairs of layer shadow flags go back
    /// to "layer 2 casts, layers 2 to 5 receive" whatever the profile stores (REQ-PERS-005
    /// points 3 and 4), and both input ports go back to Gamepad (REQ-PERS-012, DEC-09-Q6).
    /// </remarks>
    public void ApplyProfile(IEnumerable<KeyValuePair<string, SettingValue>> values)
    {
        ArgumentNullException.ThrowIfNull(values);

        // Every P setting starts again from its default, so that a field absent from the
        // profile takes the default of REQ-PROF-040 and not the value the previous game left.
        foreach (SettingDescriptor descriptor in SettingsCatalogue.OfScope(SettingScope.Profile))
        {
            SettingValue before = Read(descriptor);
            _profile.Remove(descriptor.Key);
            NotifyIfChanged(descriptor, before);
        }

        foreach ((string key, SettingValue value) in values)
        {
            if (SettingsCatalogue.Find(key) is { Scope: SettingScope.Profile } descriptor)
            {
                Write(descriptor, value, SettingLayer.Profile);
            }
        }

        Set(SettingKeys.SceneUseNesBgColor, SettingValue.OfBool(true), SettingLayer.Profile);
        for (int layer = 0; layer < SettingKeys.LayerCount; layer++)
        {
            Set(
                SettingKeys.LayerCastShadow(layer),
                SettingValue.OfBool(SettingsCatalogue.DefaultCastShadow(layer)),
                SettingLayer.Profile);
            Set(
                SettingKeys.LayerRecvShadow(layer),
                SettingValue.OfBool(SettingsCatalogue.DefaultRecvShadow(layer)),
                SettingLayer.Profile);
        }

        ResetInputPorts();
    }

    /// <summary>
    /// Both input ports go back to Gamepad, which happens at every program start and at every
    /// load of a different game, before the start-up hook (REQ-PERS-012, DEC-09-Q6).
    /// </summary>
    public void ResetInputPorts()
    {
        for (int port = 0; port < SettingKeys.PortCount; port++)
        {
            Reset(SettingKeys.InputPortType(port));
        }
    }

    /// <summary>
    /// The automatic resets at the end of a game (REQ-PERS-012): the session layer is dropped,
    /// so the scripts' writes on user preferences are undone (REQ-PERS-010) and
    /// <c>camera.smooth_time</c> goes back to 0.4, and <c>emu.game_speed</c> goes back to 0.
    /// </summary>
    public void EndOfGame()
    {
        ClearSessionLayer();
        Reset(SettingKeys.EmuGameSpeed);
    }

    /// <summary>
    /// Drops every value a script wrote, restoring the user's preferences and the session
    /// defaults (REQ-PERS-010).
    /// </summary>
    public void ClearSessionLayer()
    {
        if (_session.Count == 0)
        {
            return;
        }

        string[] keys = [.. _session.Keys];
        List<(SettingDescriptor Descriptor, SettingValue Before)> before = new(keys.Length);
        foreach (string key in keys)
        {
            SettingDescriptor descriptor = SettingsCatalogue.Get(key);
            before.Add((descriptor, Read(descriptor)));
        }

        _session.Clear();
        foreach ((SettingDescriptor descriptor, SettingValue previous) in before)
        {
            NotifyIfChanged(descriptor, previous);
        }
    }

    private SettingValue Read(SettingDescriptor descriptor)
    {
        // REQ-PERS-011: on desktop the VR runtime always reads 0 and the scene ratio always
        // reads 1, whatever is stored.
        if (descriptor.Key == SettingKeys.VrRuntime)
        {
            return SettingValue.OfInt(0);
        }

        if (descriptor.Key == SettingKeys.VrSceneRatio)
        {
            return SettingValue.OfFloat(1f);
        }

        return descriptor.Scope == SettingScope.Derived
            ? Derive(descriptor)
            : ReadStored(descriptor);
    }

    private SettingValue ReadStored(SettingDescriptor descriptor)
    {
        if (descriptor.Scope == SettingScope.Derived)
        {
            return Derive(descriptor);
        }

        if (_session.TryGetValue(descriptor.Key, out SettingValue session))
        {
            return session;
        }

        // An explicitly given command-line option wins over what the file holds, and only over
        // that: a script's write for the current game still masks it (Q-066).
        if (_command.TryGetValue(descriptor.Key, out SettingValue command))
        {
            return command;
        }

        if (descriptor.Scope == SettingScope.Profile)
        {
            return _profile.TryGetValue(descriptor.Key, out SettingValue profile)
                ? profile
                : descriptor.DefaultValue;
        }

        return _user.TryGetValue(descriptor.Key, out SettingValue user) ? user : descriptor.DefaultValue;
    }

    private bool Write(SettingDescriptor descriptor, SettingValue value, SettingLayer layer)
    {
        if (descriptor.Scope == SettingScope.Derived)
        {
            // Writing SceneScale writes SceneRatio (REQ-PERS-013 point 3); the other derived
            // members refuse the write.
            return descriptor.Key == SettingKeys.SceneSceneScale
                && Set(SettingKeys.VrSceneRatio, value, layer);
        }

        if (descriptor.Key == SettingKeys.VrRuntime)
        {
            // Ignored without error, so a profile testing VrRuntime > 0 takes the desktop
            // branch (REQ-PERS-011).
            return false;
        }

        if (descriptor.Key == SettingKeys.MovieMode && value.AsInt != 0)
        {
            // Frozen at NONE (REQ-PERS-016).
            return false;
        }

        SettingValue clean = descriptor.Domain.Coerce(
            value.CoerceTo(descriptor.Kind),
            descriptor.DefaultValue,
            _coreCount);

        SettingValue before = Read(descriptor);
        SettingValue storedBefore = ReadStored(descriptor);

        // The session layer exists for one reason: a script's write on a <b>user preference</b>
        // must not become the preference (REQ-PERS-010, DEC-09-Q3). A script's write on a
        // profile setting is a change of the live scene value, which REQ-PERS-006 keeps when the
        // same game is relaunched, so it goes to the profile layer like any other.
        Dictionary<string, SettingValue> target = descriptor.Scope switch
        {
            SettingScope.Session => _session,
            SettingScope.Profile => layer == SettingLayer.CommandLine ? _command : _profile,
            _ => layer switch
            {
                SettingLayer.Session => _session,
                SettingLayer.CommandLine => _command,
                _ => _user,
            },
        };

        target[descriptor.Key] = clean;

        // The user's own choice ends the command-line override of that one setting: from now on
        // the value in force is a preference again, and it is persisted (Q-066).
        if (layer == SettingLayer.User && !ReferenceEquals(target, _command))
        {
            _command.Remove(descriptor.Key);
        }

        // The VR scene ratio reads 1 either way, so the change that matters — and that asks the
        // scene for the view reset of REQ-SCN-002 — is the one of the stored value.
        if (descriptor.Key == SettingKeys.VrSceneRatio)
        {
            if (storedBefore == clean)
            {
                return false;
            }

            Raise(descriptor, Read(descriptor));
            return true;
        }

        return NotifyIfChanged(descriptor, before);
    }

    private bool NotifyIfChanged(SettingDescriptor descriptor, SettingValue before)
    {
        SettingValue after = Read(descriptor);
        if (before == after)
        {
            return false;
        }

        Raise(descriptor, after);
        RaiseDependents(descriptor);
        return true;
    }

    private void Raise(SettingDescriptor descriptor, SettingValue value)
        => Changed?.Invoke(new SettingChange(descriptor.Key, value, descriptor.Effect));

    private void RaiseDerived(string key)
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(key);
        Raise(descriptor, Derive(descriptor));
    }

    /// <summary>
    /// Tells the subscribers about the derived values a change makes stale (REQ-PERS-013), so a
    /// subsystem can watch <c>video.target_framerate</c> instead of every setting that feeds it.
    /// </summary>
    private void RaiseDependents(SettingDescriptor descriptor)
    {
        switch (descriptor.Key)
        {
            case SettingKeys.VideoVSync:
            case SettingKeys.VideoFrameRate:
                RaiseDerived(SettingKeys.VideoTargetFramerate);
                RaiseDerived(SettingKeys.VideoTargetTimePerFrame);
                break;

            case SettingKeys.VideoResScale:
                RaiseDerived(SettingKeys.VideoResScaleFactor);
                break;

            case SettingKeys.SceneSkybox:
                RaiseDerived(SettingKeys.SceneSkyboxEnable);
                break;

            case SettingKeys.SceneRootScale:
            case SettingKeys.VrSceneRatio:
                RaiseDerived(SettingKeys.SceneSceneScale);
                break;

            case SettingKeys.EmuTvMode:
                RaiseDerived(SettingKeys.EmuTvData);
                break;

            default:
                if (descriptor.Key.EndsWith("].aligned", StringComparison.Ordinal))
                {
                    RaiseDerived(descriptor.Key.Replace("].aligned", "].coef", StringComparison.Ordinal));
                }
                else if (descriptor.Key.StartsWith("input.port_type[", StringComparison.Ordinal))
                {
                    RaiseDerived(SettingKeys.InputZapperEnable);
                }

                break;
        }
    }

    /// <summary>The derived values of REQ-PERS-013.</summary>
    private SettingValue Derive(SettingDescriptor descriptor)
    {
        switch (descriptor.Key)
        {
            case SettingKeys.VideoTargetFramerate:
                return SettingValue.OfFloat(TargetFramerate());

            case SettingKeys.VideoTargetTimePerFrame:
                float rate = TargetFramerate();
                return SettingValue.OfFloat(rate <= 0f ? 0f : 1f / rate);

            case SettingKeys.SceneSceneScale:

                // RootScale × SceneRatio; the ratio reads 1 on desktop, so this is RootScale.
                return SettingValue.OfFloat(GetFloat(SettingKeys.SceneRootScale) * 1f);

            case SettingKeys.SceneSkyboxEnable:
                return SettingValue.OfBool(GetInt(SettingKeys.SceneSkybox) > 0);

            case SettingKeys.InputZapperEnable:
                bool zapper = false;
                for (int port = 0; port < SettingKeys.PortCount && !zapper; port++)
                {
                    zapper = GetInt(SettingKeys.InputPortType(port)) == 1;
                }

                return SettingValue.OfBool(zapper);

            case SettingKeys.VideoResScaleFactor:
                return SettingValue.OfFloat(GetInt(SettingKeys.VideoResScale) switch
                {
                    1 => 2f / 3f,
                    2 => 0.5f,
                    3 => 1f / 3f,
                    4 => 0.25f,
                    _ => 1f,
                });

            case SettingKeys.EmuTvData:
                return SettingValue.OfOpaque(TvData);

            default:
                if (descriptor.Key.EndsWith("].coef", StringComparison.Ordinal))
                {
                    string aligned = descriptor.Key.Replace("].coef", "].aligned", StringComparison.Ordinal);
                    return SettingValue.OfFloat(SettingsCatalogue.AlignmentCoefficient(GetInt(aligned)));
                }

                return descriptor.DefaultValue;
        }
    }

    private float TargetFramerate() => GetInt(SettingKeys.VideoVSync) switch
    {
        1 => _displayRefreshRate,
        2 => _displayRefreshRate / 2f,
        _ => GetInt(SettingKeys.VideoFrameRate),
    };
}
