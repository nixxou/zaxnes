using Zax.Engine3D;

namespace Zax.Render.Assets;

/// <summary>
/// The six faces of a cubemap, in the order <c>assets-neutral/cubemaps/cubemaps.json</c> names
/// them: a left-handed frame with Y up, rows stored top-down.
/// </summary>
public enum CubeFace
{
    /// <summary><c>px</c>, the +X face.</summary>
    PositiveX = 0,

    /// <summary><c>nx</c>, the −X face.</summary>
    NegativeX = 1,

    /// <summary><c>py</c>, the +Y face.</summary>
    PositiveY = 2,

    /// <summary><c>ny</c>, the −Y face.</summary>
    NegativeY = 3,

    /// <summary><c>pz</c>, the +Z face.</summary>
    PositiveZ = 4,

    /// <summary><c>nz</c>, the −Z face.</summary>
    NegativeZ = 5,
}

/// <summary>
/// Where the renderer gets the extracted assets of SPEC-06: the skybox cubemaps of REQ-SCN-050
/// and REQ-SCN-051, and the meshes and textures of the decorative television of REQ-SCN-054.
/// </summary>
/// <remarks>
/// The renderer never touches the file system itself: the host supplies this (REQ-SCN-057,
/// REQ-ARCH-003). A test can therefore inject the uniform cubemaps the acceptance criterion of
/// REQ-SCN-051 asks for. Every member may return <see langword="null"/>; the renderer then
/// falls back to the plain background and to no television.
/// </remarks>
public interface ISceneAssets
{
    /// <summary>
    /// One face of the cubemap <paramref name="cubemap"/> (<c>Stars_LP_01</c>, <c>Starfield</c>,
    /// <c>NebulaDensity</c>, <c>NebulaDiffusion</c>, <c>Cubemap</c>), or null when it is missing.
    /// All six faces of one cubemap have the same size.
    /// </summary>
    ImageData? CubemapFace(string cubemap, CubeFace face);

    /// <summary>The texture <paramref name="name"/> of <c>assets-neutral/textures</c>, or null.</summary>
    ImageData? Texture(string name);

    /// <summary>The mesh <paramref name="name"/> of <c>assets-neutral/meshes</c>, or null.</summary>
    TileMesh? Mesh(string name);
}
