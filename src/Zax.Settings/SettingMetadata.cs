namespace Zax.Settings;

/// <summary>
/// Scope of a setting: who owns its current value and what happens to it when a profile loads
/// (SPEC-09 §2).
/// </summary>
public enum SettingScope
{
    /// <summary>
    /// U — user preference: global to the application, persisted in the settings file and
    /// <b>kept</b> when a profile loads (REQ-PERS-005 point 2).
    /// </summary>
    User,

    /// <summary>
    /// P — scene parameter of the profile: replaced by the value embedded in the profile when a
    /// game different from the previous one starts (REQ-PERS-005 point 1).
    /// </summary>
    Profile,

    /// <summary>S — session: never persisted, back to its default at every program start.</summary>
    Session,

    /// <summary>D — derived: read only, computed from other settings or from the platform.</summary>
    Derived,
}

/// <summary>
/// When a change takes effect, for a frontend that has to warn about a restart
/// (REQ-PERS-084, column "Effet" of SPEC-09 §4.1).
/// </summary>
public enum SettingEffect
{
    /// <summary><c>imm.</c> — at the next rendered frame at the latest.</summary>
    Immediate,

    /// <summary><c>jeu</c> — at the next game load.</summary>
    NextGameLoad,

    /// <summary><c>ROM</c> — at the next ROM load.</summary>
    NextRomLoad,

    /// <summary><c>redém.</c> — at the next program start.</summary>
    Restart,

    /// <summary>No effect at all: the value is stored and handed back only.</summary>
    None,
}

/// <summary>The shape of a setting's value.</summary>
public enum SettingKind
{
    /// <summary><c>true</c> or <c>false</c>.</summary>
    Bool,

    /// <summary>A whole number.</summary>
    Int,

    /// <summary>A real number.</summary>
    Float,

    /// <summary>A whole number out of a finite named set (column "Domaine").</summary>
    Enum,

    /// <summary>An RGBA colour of four reals in 0..1.</summary>
    Color,

    /// <summary>Three reals.</summary>
    Vector3,

    /// <summary>Text.</summary>
    String,

    /// <summary>Four whole numbers: x, y, width, height.</summary>
    Rect,

    /// <summary>
    /// A value this model stores and hands back without interpreting it — the editor preset
    /// lists and the region timing block (REQ-PERS-016, REQ-PERS-013 point 7).
    /// </summary>
    Opaque,
}

/// <summary>
/// Theme a setting belongs to: the grouping of the options window (AMD-010 REQ-UI-202) and the
/// libretro option category (REQ-PERS-083).
/// </summary>
public enum SettingTheme
{
    /// <summary>Video output and rendering.</summary>
    Video,

    /// <summary>Mesh generation and shadows.</summary>
    Render,

    /// <summary>Camera.</summary>
    Camera,

    /// <summary>Light and shadows.</summary>
    Light,

    /// <summary>Scene: background and skybox.</summary>
    Scene,

    /// <summary>Picture clipping.</summary>
    Clipping,

    /// <summary>Depth layers.</summary>
    Layers,

    /// <summary>Emulation.</summary>
    Emulation,

    /// <summary>2D to 3D conversion.</summary>
    Conversion,

    /// <summary>Audio.</summary>
    Audio,

    /// <summary>Inputs.</summary>
    Input,

    /// <summary>Interface.</summary>
    Interface,

    /// <summary>VR, out of scope (REQ-PERS-011).</summary>
    Vr,

    /// <summary>Profile editor, out of scope (REQ-PERS-016).</summary>
    Editor,

    /// <summary>Movie recording, out of scope (REQ-PERS-016).</summary>
    Movie,

    /// <summary>Internal and derived values.</summary>
    Internal,
}

/// <summary>What to do with a value that falls outside the domain (REQ-PERS-003).</summary>
public enum OutOfDomainRule
{
    /// <summary>
    /// No rule given by the column "Domaine": the default value is used, as REQ-PERS-003
    /// requires in that case.
    /// </summary>
    UseDefault,

    /// <summary>Bring the value back to the nearest bound.</summary>
    Clamp,

    /// <summary>Take the absolute value (<c>conv.sprite_tolerance</c>, rule 4).</summary>
    AbsoluteValue,

    /// <summary>
    /// Remainder of the division by the number of available cores (<c>emu.core</c>, rule 2).
    /// </summary>
    ModuloCoreCount,

    /// <summary>Any value is accepted: the setting has no domain.</summary>
    Unbounded,
}

/// <summary>One admitted value of an enumerated domain, with the label the file and the
/// libretro option use (REQ-PERS-081).</summary>
/// <param name="Value">Numeric value, which Lua accepts as well as the named constant
/// (REQ-PERS-004).</param>
/// <param name="Label">Stable label, written to the settings file and shown by libretro.</param>
public readonly record struct SettingOption(double Value, string Label);
