using ImGuiNET;

namespace Zax.App.Ui;

/// <summary>
/// The fonts of SPEC-10 REQ-UI-121, loaded from the machine at run time
/// (AMD-024 REQ-PUB-006): a system face for the body text and, when the family has one, its
/// bold file for the headings.
/// </summary>
/// <remarks>
/// <para>
/// Loading a real font is not cosmetic: the interface is in French (AMD-010) and Dear ImGui's
/// built-in font has no accented glyph, so « Émulation » or « Réinitialiser » would lose its
/// accents. REQ-PUB-006 therefore asks for a face already installed on the machine — Segoe UI,
/// Arial as the fallback — opened at run time and <b>not redistributed</b>:
/// <see cref="SystemFonts"/> finds it. Nothing is bundled any more; the two TTF files of
/// <c>assets-neutral/fonts</c> are gone with the rest of the original assets (REQ-PUB-002).
/// </para>
/// <para>
/// With no candidate installed, or with <c>--builtin-font</c>, the built-in font stays in place
/// and the accents and the triangle of REQ-UI-207 come out as ASCII, which REQ-PUB-006 allows
/// for the fallback.
/// </para>
/// </remarks>
internal static class UiFonts
{
    /// <summary>Body size in pixels.</summary>
    public const float BodySize = 16f;

    /// <summary>Heading size in pixels.</summary>
    public const float HeadingSize = 22f;

    /// <summary>
    /// The triangle of « Fichier ▸ Ouvrir une ROM » (REQ-UI-207), U+25B8 BLACK RIGHT-POINTING
    /// SMALL TRIANGLE.
    /// </summary>
    public const ushort SmallRightTriangle = 0x25B8;

    /// <summary>
    /// U+25BA BLACK RIGHT-POINTING POINTER, the same filled triangle one size up and the most
    /// widely carried of the three.
    /// </summary>
    public const ushort RightPointer = 0x25BA;

    /// <summary>U+25B6 BLACK RIGHT-POINTING TRIANGLE, the second substitute.</summary>
    public const ushort RightTriangle = 0x25B6;

    /// <summary>
    /// The ASCII greater-than sign, the last resort: every font has it, so the sentence still
    /// reads « Fichier > Ouvrir une ROM » rather than showing a missing-glyph box.
    /// </summary>
    public const ushort AsciiPointer = 0x3E;

    /// <summary>The heading font, or null when only the built-in font is loaded.</summary>
    public static ImFontPtr? Heading { get; private set; }

    /// <summary>The body font, or null when only the built-in font is loaded.</summary>
    public static ImFontPtr? Body { get; private set; }

    /// <summary>The file the body font came from, for the log and the about block.</summary>
    public static string? BodyFile { get; private set; }

    /// <summary>
    /// Adds the fonts to the current ImGui context. Called from the controller's configuration
    /// callback, before the font atlas is built.
    /// </summary>
    /// <param name="hint">
    /// The value of <c>--font</c>, or null to take the first installed candidate.
    /// </param>
    /// <param name="builtInOnly">
    /// True to load nothing and keep Dear ImGui's built-in font, the forced fallback of
    /// REQ-PUB-006.
    /// </param>
    /// <returns>True when a system font was loaded.</returns>
    public static bool Load(string? hint = null, bool builtInOnly = false)
    {
        Heading = null;
        Body = null;
        BodyFile = null;

        if (builtInOnly)
        {
            return false;
        }

        (string? regular, string? bold) = SystemFonts.Locate(hint);
        if (regular is null)
        {
            return false;
        }

        ImGuiIOPtr io = ImGui.GetIO();
        nint ranges = GlyphRanges;

        Body = io.Fonts.AddFontFromFileTTF(regular, BodySize, null, ranges);
        BodyFile = regular;

        // The headings use the bold file of the same family when the system has one, and the
        // regular face at the larger size otherwise.
        Heading = io.Fonts.AddFontFromFileTTF(bold ?? regular, HeadingSize, null, ranges);
        return true;
    }

    /// <summary>
    /// Points U+25B8 at the nearest triangle the atlas actually carries (REQ-UI-207). Must be
    /// called <b>after</b> the atlas is built — Dear ImGui's <c>AddRemapChar</c> works on the
    /// index tables the build fills — which is why the caller is the host, right after the ImGui
    /// controller has created the font texture.
    /// </summary>
    /// <remarks>
    /// A system face does not necessarily carry U+25B8: Segoe UI and Arial both have U+25BA, and
    /// some faces only have U+25B6. The substitutes are tried in that order and the ASCII
    /// greater-than sign closes the list, so the sentence is readable whatever the machine has
    /// (REQ-PUB-006). The string itself stays the amendment's, U+25B8 included.
    /// </remarks>
    public static void RemapMissingGlyphs()
    {
        if (Body is null)
        {
            // The built-in font of REQ-PUB-006's degraded mode: Latin-1 is there, so the French
            // accents survive, but neither the triangle of REQ-UI-207 nor the ellipsis of
            // « Ouvrir une ROM… » is. Both are pointed at the ASCII sign the amendment allows
            // as a substitute, which reads better than Dear ImGui's '?' box.
            RemapBuiltIn();
            return;
        }

        Remap(Body);
        Remap(Heading);

        static void Remap(ImFontPtr? font)
        {
            if (font is not { } target || !target.IsLoaded())
            {
                return;
            }

            if (Carries(target, SmallRightTriangle))
            {
                return;
            }

            foreach (ushort substitute in (ushort[])[RightPointer, RightTriangle, AsciiPointer])
            {
                if (!Carries(target, substitute))
                {
                    continue;
                }

                target.AddRemapChar(SmallRightTriangle, substitute, overwrite_dst: true);
                return;
            }
        }
    }

    /// <summary>The ASCII substitutes the built-in font falls back on (REQ-PUB-006).</summary>
    private static readonly (ushort From, ushort To)[] AsciiSubstitutes =
    [
        (SmallRightTriangle, AsciiPointer),
        (0x2026, 0x2E), // Horizontal ellipsis to a full stop.
        (0x2014, 0x2D), // Em dash to a hyphen.
        (0x2013, 0x2D), // En dash to a hyphen.
    ];

    private static void RemapBuiltIn()
    {
        ImFontAtlasPtr atlas = ImGui.GetIO().Fonts;
        if (atlas.Fonts.Size == 0)
        {
            return;
        }

        ImFontPtr font = atlas.Fonts[0];
        if (!font.IsLoaded())
        {
            return;
        }

        foreach ((ushort from, ushort to) in AsciiSubstitutes)
        {
            if (!Carries(font, from) && Carries(font, to))
            {
                font.AddRemapChar(from, to, overwrite_dst: true);
            }
        }
    }

    /// <summary>Whether <paramref name="font"/> has a real glyph for <paramref name="code"/>.</summary>
    private static unsafe bool Carries(ImFontPtr font, ushort code)
        => ImGuiNative.ImFont_FindGlyphNoFallback(font.NativePtr, code) is not null;

    /// <summary>
    /// The glyphs the interface needs: Latin-1 — which covers every accent of the French labels
    /// of AMD-010 — plus the few typographic signs the amendment itself writes, the ellipsis of
    /// « Ouvrir une ROM… » and the triangle of « Fichier ▸ Ouvrir une ROM » (REQ-UI-207).
    /// </summary>
    /// <remarks>
    /// Dear ImGui keeps the pointer until the atlas is built, so the array is pinned for the
    /// lifetime of the process rather than copied.
    /// </remarks>
    private static nint GlyphRanges
    {
        get
        {
            if (field != 0)
            {
                return field;
            }

            ushort[] ranges =
            [
                0x0020, 0x00FF, // Latin-1: the accents and the guillemets.
                0x0152, 0x0153, // Œ and œ, which « Cœur d'émulation » (REQ-UI-202) needs.
                0x2010, 0x2015, // Hyphens, en dash and em dash — the "unbound" sign of DEC-08-5.
                0x2018, 0x201F, // Typographic quotes.
                0x2026, 0x2026, // Horizontal ellipsis.
                0x25B6, 0x25BA, // The small right-pointing triangles.
                0x0000,
            ];

            field = System.Runtime.InteropServices.GCHandle
                .Alloc(ranges, System.Runtime.InteropServices.GCHandleType.Pinned)
                .AddrOfPinnedObject();
            return field;
        }
    }
}
