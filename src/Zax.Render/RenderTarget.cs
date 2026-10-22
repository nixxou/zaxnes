namespace Zax.Render;

/// <summary>
/// A framebuffer owned by the host, into which <see cref="ISceneRenderer"/> draws.
/// </summary>
/// <param name="Framebuffer">
/// Name of an OpenGL framebuffer object that is complete for both colour and depth.
/// Zero is accepted but means "the host really did hand us its default framebuffer";
/// the renderer never assumes it.
/// </param>
/// <param name="Width">Width of the target in pixels. Must be positive.</param>
/// <param name="Height">Height of the target in pixels. Must be positive.</param>
/// <remarks>
/// PLAN.md, libretro portability section: the renderer draws into a target provided from
/// the outside (FBO, size and context), so that a libretro frontend can supply its own.
/// </remarks>
public readonly record struct RenderTarget(uint Framebuffer, int Width, int Height)
{
    /// <summary>True when the declared size can be rendered into.</summary>
    public bool IsValid => Width > 0 && Height > 0;

    /// <summary>Throws when the target size is not usable.</summary>
    /// <exception cref="ArgumentOutOfRangeException">The width or the height is not positive.</exception>
    public void Validate()
    {
        if (Width <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(Width), Width, "Render target width must be positive.");
        }

        if (Height <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(Height), Height, "Render target height must be positive.");
        }
    }
}
