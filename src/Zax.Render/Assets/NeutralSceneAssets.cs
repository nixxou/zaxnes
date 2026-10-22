using Zax.Engine3D;

namespace Zax.Render.Assets;

/// <summary>
/// Reads the skybox and television assets straight out of <c>assets-neutral/</c>.
/// </summary>
/// <remarks>
/// Meshes and the small textures are cached once read; cubemap faces are <b>not</b> cached,
/// because the three 2048 × 2048 skybox cubemaps would hold about 300 MB of RGBA in memory.
/// The GPU side uploads each face as it is read and lets it go (<c>CubemapCache</c>).
/// </remarks>
public sealed class NeutralSceneAssets : ISceneAssets
{
    private readonly Dictionary<string, ImageData?> _textures = [];
    private readonly Dictionary<string, TileMesh?> _meshes = [];

    /// <summary>Creates the reader for an <c>assets-neutral</c> folder.</summary>
    /// <param name="root">The folder holding <c>cubemaps/</c>, <c>meshes/</c> and <c>textures/</c>.</param>
    public NeutralSceneAssets(string root)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(root);
        Root = root;
    }

    /// <summary>The asset folder.</summary>
    public string Root { get; }

    /// <summary>The file name of a cubemap face, as <c>cubemaps.json</c> spells it.</summary>
    public static string FaceName(CubeFace face) => face switch
    {
        CubeFace.PositiveX => "px",
        CubeFace.NegativeX => "nx",
        CubeFace.PositiveY => "py",
        CubeFace.NegativeY => "ny",
        CubeFace.PositiveZ => "pz",
        _ => "nz",
    };

    /// <summary>
    /// Opens <paramref name="root"/> when it holds the three asset folders, else null.
    /// </summary>
    public static NeutralSceneAssets? TryOpen(string? root)
        => string.IsNullOrWhiteSpace(root) || !Directory.Exists(Path.Combine(root, "cubemaps"))
            ? null
            : new NeutralSceneAssets(root);

    /// <inheritdoc/>
    public ImageData? CubemapFace(string cubemap, CubeFace face)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(cubemap);
        return Read(Path.Combine(Root, "cubemaps", cubemap, FaceName(face) + ".png"));
    }

    /// <inheritdoc/>
    public ImageData? Texture(string name)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(name);
        if (_textures.TryGetValue(name, out ImageData? cached))
        {
            return cached;
        }

        ImageData? image = Read(Path.Combine(Root, "textures", name + ".png"));
        _textures[name] = image;
        return image;
    }

    /// <inheritdoc/>
    public TileMesh? Mesh(string name)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(name);
        if (_meshes.TryGetValue(name, out TileMesh? cached))
        {
            return cached;
        }

        string path = Path.Combine(Root, "meshes", name + ".obj");
        TileMesh? mesh = null;
        if (File.Exists(path))
        {
            try
            {
                mesh = ObjMesh.Load(path);
            }
            catch (IOException)
            {
                mesh = null;
            }
        }

        _meshes[name] = mesh;
        return mesh;
    }

    private static ImageData? Read(string path)
    {
        if (!File.Exists(path))
        {
            return null;
        }

        try
        {
            return PngImage.Load(path);
        }
        catch (Exception exception) when (exception is IOException or InvalidDataException)
        {
            return null;
        }
    }
}
