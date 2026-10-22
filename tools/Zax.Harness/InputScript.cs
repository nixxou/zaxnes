using System.Globalization;

namespace Zax.Harness;

/// <summary>
/// A scripted controller replay for <c>render --rom</c>: which port 1 buttons are held on which
/// emulated frames.
/// </summary>
/// <remarks>
/// SPEC-01 REQ-ARCH-031 asks for "load a ROM and a profile, replay, export a capture"; without
/// any input most games never leave their title screen, so a capture of a real game scene needs
/// a few button presses. The spec format is a comma separated list of
/// <c>&lt;first&gt;[-&lt;last&gt;]:&lt;buttons&gt;</c>, frame numbers counting from the reset,
/// the buttons joined by <c>+</c>. Ranges may overlap: the held buttons are the union.
/// </remarks>
internal sealed class InputScript
{
    /// <summary>A script that holds nothing.</summary>
    public static InputScript None { get; } = new([]);

    private readonly (int First, int Last, ushort Buttons)[] _spans;

    private InputScript((int First, int Last, ushort Buttons)[] spans) => _spans = spans;

    /// <summary>Parses the <c>--press</c> value; null or empty gives <see cref="None"/>.</summary>
    /// <exception cref="ArgumentException">The text is not a valid script.</exception>
    public static InputScript Parse(string? text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return None;
        }

        List<(int, int, ushort)> spans = [];
        foreach (string entry in text.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            int colon = entry.IndexOf(':', StringComparison.Ordinal);
            if (colon <= 0)
            {
                throw new ArgumentException($"--press: '{entry}' is not <frames>:<buttons>.");
            }

            string frames = entry[..colon];
            int dash = frames.IndexOf('-', StringComparison.Ordinal);
            string firstText = dash < 0 ? frames : frames[..dash];
            string lastText = dash < 0 ? frames : frames[(dash + 1)..];

            if (!int.TryParse(firstText, CultureInfo.InvariantCulture, out int first)
                || !int.TryParse(lastText, CultureInfo.InvariantCulture, out int last)
                || first < 0
                || last < first)
            {
                throw new ArgumentException($"--press: '{frames}' is not a frame or a frame range.");
            }

            ushort buttons = 0;
            foreach (string name in entry[(colon + 1)..].Split('+', StringSplitOptions.RemoveEmptyEntries))
            {
                buttons |= Button(name.Trim());
            }

            spans.Add((first, last, buttons));
        }

        return new InputScript([.. spans]);
    }

    /// <summary>The port 1 input word of REQ-CORE-022 for emulated frame <paramref name="frame"/>.</summary>
    public ushort At(int frame)
    {
        ushort word = 0;
        foreach ((int first, int last, ushort buttons) in _spans)
        {
            if (frame >= first && frame <= last)
            {
                word |= buttons;
            }
        }

        return word;
    }

    // SPEC-02 §4.4: A, B, Select, Start, Up, Down, Left, Right in bits 0 to 7.
    private static ushort Button(string name) => name.ToLowerInvariant() switch
    {
        "a" => 1 << 0,
        "b" => 1 << 1,
        "select" => 1 << 2,
        "start" => 1 << 3,
        "up" => 1 << 4,
        "down" => 1 << 5,
        "left" => 1 << 6,
        "right" => 1 << 7,
        _ => throw new ArgumentException(
            $"--press: '{name}' is not a button (a, b, select, start, up, down, left, right)."),
    };
}
