namespace Zax.App;

/// <summary>
/// How big the offscreen buffer the scene is drawn into has to be, from the render method and
/// the resolution scale of SPEC-09 §4.1.1 (REQ-SCN-063, AMD-032 §3).
/// </summary>
/// <remarks>
/// <para>
/// The application always draws the scene into its own framebuffer and blits it onto the
/// window (PLAN.md, libretro portability). The two settings only decide how big that
/// framebuffer is:
/// </para>
/// <list type="bullet">
///   <item>
///     <description>
///       <c>video.render_method</c> = 0, "Direct": the buffer has the size of the view, so the
///       scene is drawn at the native resolution and the blit is one-to-one. This is what the
///       original does on Windows, where it offers no other choice, and
///       <c>video.res_scale</c> then has no effect at all.
///     </description>
///   </item>
///   <item>
///     <description>
///       <c>video.render_method</c> = 1, "Render Tex": the buffer is the view scaled by
///       <c>video.res_scale_factor</c> — 1, 2/3, 1/2, 1/3 or 1/4 — and the blit stretches it
///       back, filtered bilinearly (REQ-SCN-063).
///     </description>
///   </item>
/// </list>
/// </remarks>
internal static class RenderScaling
{
    /// <summary>The render method that draws at the view's own resolution.</summary>
    public const int DirectMethod = 0;

    /// <summary>The render method the resolution scale applies to.</summary>
    public const int RenderTextureMethod = 1;

    /// <summary>
    /// Whether the resolution scale has any effect, which is what the options window greys the
    /// row out on.
    /// </summary>
    /// <param name="renderMethod">The value of <c>video.render_method</c>.</param>
    public static bool ScaleApplies(int renderMethod) => renderMethod == RenderTextureMethod;

    /// <summary>
    /// The size of the offscreen buffer for a view of <paramref name="viewWidth"/> ×
    /// <paramref name="viewHeight"/> pixels.
    /// </summary>
    /// <param name="viewWidth">Width of the game view in window pixels.</param>
    /// <param name="viewHeight">Height of the game view in window pixels.</param>
    /// <param name="renderMethod">The value of <c>video.render_method</c>.</param>
    /// <param name="factor">The value of <c>video.res_scale_factor</c>.</param>
    /// <returns>At least one pixel in each direction, whatever the factor.</returns>
    public static (int Width, int Height) SceneSize(
        int viewWidth,
        int viewHeight,
        int renderMethod,
        float factor)
    {
        int width = Math.Max(1, viewWidth);
        int height = Math.Max(1, viewHeight);
        if (!ScaleApplies(renderMethod) || !float.IsFinite(factor) || factor >= 1f || factor <= 0f)
        {
            return (width, height);
        }

        // Rounding rather than truncating keeps 2/3 of 1080 at 720 instead of 719, so the
        // factors stay the exact fractions SPEC-06 names wherever the view divides evenly.
        return (
            Math.Max(1, (int)MathF.Round(width * factor)),
            Math.Max(1, (int)MathF.Round(height * factor)));
    }
}
