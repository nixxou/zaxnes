using System.Numerics;
using Zax.Engine3D;

namespace Zax.Render;

/// <summary>
/// The celestial bodies of the procedural skyboxes of AMD-024 REQ-PUB-003: a sun disc, a moon
/// disc and a cloud layer, all produced by the skybox shader and driven from here.
/// </summary>
/// <remarks>
/// <para>
/// Q-064: REQ-PUB-003 asks the procedural skyboxes to carry « soleil et lune, nuages » among
/// their functional parameters, while SPEC-06 REQ-SCN-050 states that the sun, the moon and the
/// clouds of the original skybox pack <b>are disabled</b> — and gives an acceptance criterion
/// (horizon, zenith, nadir) that a visible sun would break. The two are reconciled by making the
/// three bodies real, parametric and <b>off by default</b>: every gain starts at 0, so the
/// default image is the gradient and the stars of REQ-SCN-050, and a host or a test that sets a
/// gain sees the image change, which is what the criterion of REQ-PUB-003 asks for.
/// </para>
/// <para>
/// The bodies are fixed in the scene frame, like the rest of the skybox (REQ-SCN-049): nothing
/// here follows the clock. Only the star twinkle of REQ-SCN-050 moves, and it is not a body.
/// </para>
/// </remarks>
// Q-064
public sealed class SkyBodySettings
{
    private float _sunGain;
    private float _moonGain;
    private float _cloudGain;
    private float _sunRadiusDegrees = 2.4f;
    private float _moonRadiusDegrees = 3.2f;
    private float _cloudCoverage = 0.45f;
    private Vector3 _sunDirection = Vector3.Normalize(new Vector3(-0.45f, 0.62f, 1f));
    private Vector3 _moonDirection = Vector3.Normalize(new Vector3(0.7f, 0.42f, -1f));

    /// <summary>Weight of the sun disc in the sky, 0 (the default) to 1.</summary>
    public float SunGain
    {
        get => _sunGain;
        set => _sunGain = Math.Clamp(value, 0f, 1f);
    }

    /// <summary>Weight of the moon disc, 0 (the default) to 1.</summary>
    public float MoonGain
    {
        get => _moonGain;
        set => _moonGain = Math.Clamp(value, 0f, 1f);
    }

    /// <summary>Weight of the cloud layer, 0 (the default) to 1.</summary>
    public float CloudGain
    {
        get => _cloudGain;
        set => _cloudGain = Math.Clamp(value, 0f, 1f);
    }

    /// <summary>Direction of the sun in scene coordinates; normalised at use.</summary>
    public Vector3 SunDirection
    {
        get => _sunDirection;
        set => _sunDirection = Safe(value, new Vector3(0f, 1f, 0f));
    }

    /// <summary>Direction of the moon in scene coordinates; normalised at use.</summary>
    public Vector3 MoonDirection
    {
        get => _moonDirection;
        set => _moonDirection = Safe(value, new Vector3(0f, 1f, 0f));
    }

    /// <summary>Colour of the sun disc, sRGB.</summary>
    public Rgb24 SunColor { get; set; } = new(255, 243, 214);

    /// <summary>Colour of the moon disc, sRGB.</summary>
    public Rgb24 MoonColor { get; set; } = new(226, 230, 238);

    /// <summary>Colour of the clouds, sRGB.</summary>
    public Rgb24 CloudColor { get; set; } = new(242, 244, 248);

    /// <summary>Angular radius of the sun disc in degrees, 0.2 to 30.</summary>
    public float SunRadiusDegrees
    {
        get => _sunRadiusDegrees;
        set => _sunRadiusDegrees = Math.Clamp(value, 0.2f, 30f);
    }

    /// <summary>Angular radius of the moon disc in degrees, 0.2 to 30.</summary>
    public float MoonRadiusDegrees
    {
        get => _moonRadiusDegrees;
        set => _moonRadiusDegrees = Math.Clamp(value, 0.2f, 30f);
    }

    /// <summary>
    /// How far the edge of a disc is feathered, as a fraction of its radius. 0 is a hard edge.
    /// </summary>
    public float DiscSoftness { get; set; } = 0.45f;

    /// <summary>
    /// How much of the sky the clouds cover, 0 to 1. Low values leave a few wisps, high values
    /// an overcast sky.
    /// </summary>
    public float CloudCoverage
    {
        get => _cloudCoverage;
        set => _cloudCoverage = Math.Clamp(value, 0f, 1f);
    }

    /// <summary>Angular scale of the cloud pattern; larger values give smaller cells.</summary>
    public float CloudScale { get; set; } = 2.6f;

    /// <summary>Whether anything at all is drawn, that is whether some gain is not zero.</summary>
    public bool AnyVisible => SunGain > 0f || MoonGain > 0f || CloudGain > 0f;

    /// <summary>Puts every parameter back to its default, which draws no body at all.</summary>
    public void Reset()
    {
        SunGain = 0f;
        MoonGain = 0f;
        CloudGain = 0f;
        _sunDirection = Vector3.Normalize(new Vector3(-0.45f, 0.62f, 1f));
        _moonDirection = Vector3.Normalize(new Vector3(0.7f, 0.42f, -1f));
        SunColor = new Rgb24(255, 243, 214);
        MoonColor = new Rgb24(226, 230, 238);
        CloudColor = new Rgb24(242, 244, 248);
        SunRadiusDegrees = 2.4f;
        MoonRadiusDegrees = 3.2f;
        DiscSoftness = 0.45f;
        CloudCoverage = 0.45f;
        CloudScale = 2.6f;
    }

    private static Vector3 Safe(Vector3 value, Vector3 fallback)
    {
        float length = value.Length();
        return length > 1e-6f ? value / length : fallback;
    }
}
