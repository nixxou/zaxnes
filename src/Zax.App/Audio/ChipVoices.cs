namespace Zax.App.Audio;

/// <summary>The three voice kinds of the synthesiser of AMD-024 REQ-PUB-005.</summary>
public enum ChipWave
{
    /// <summary>A square wave with a settable duty cycle.</summary>
    Pulse,

    /// <summary>A triangle wave, the softer voice.</summary>
    Triangle,

    /// <summary>A pseudo-random bit stream, the percussion voice.</summary>
    Noise,
}

/// <summary>
/// The oscillators and the envelope the synthesised interface sounds and chiptunes of AMD-024
/// REQ-PUB-005 are built from.
/// </summary>
/// <remarks>
/// Deliberately plain: a duty-cycled pulse, a triangle and a 15-bit shift register, the three
/// timbres a chiptune needs. Nothing here models the NES APU — that is <c>Zax.Nes</c>'s job and
/// it drives the emulation voice, not the interface.
/// </remarks>
public static class ChipVoices
{
    /// <summary>Concert A, MIDI note 69.</summary>
    public const int ConcertA = 69;

    /// <summary>The frequency in hertz of a MIDI note number.</summary>
    public static double Frequency(double midiNote)
        => 440.0 * Math.Pow(2.0, (midiNote - ConcertA) / 12.0);

    /// <summary>
    /// The value in [−1, 1] of a pulse of duty <paramref name="duty"/> at the phase
    /// <paramref name="phase"/>, taken modulo one.
    /// </summary>
    public static float Pulse(double phase, float duty)
        => Fraction(phase) < duty ? 1f : -1f;

    /// <summary>The value in [−1, 1] of a triangle at <paramref name="phase"/>.</summary>
    public static float Triangle(double phase)
    {
        double f = Fraction(phase);
        return (float)((4.0 * Math.Abs(f - 0.5)) - 1.0);
    }

    /// <summary>
    /// A linear attack then exponential decay envelope: <paramref name="attack"/> seconds up to
    /// 1, then a decay with the time constant <paramref name="decay"/>, cut by a 4 ms release at
    /// the end of <paramref name="length"/>.
    /// </summary>
    /// <param name="time">Time since the note started, in seconds.</param>
    /// <param name="length">Length of the note, in seconds.</param>
    /// <param name="attack">Attack time in seconds.</param>
    /// <param name="decay">Decay time constant in seconds; 0 holds the level.</param>
    public static float Envelope(double time, double length, double attack, double decay)
    {
        const double release = 0.004;
        if (time < 0.0 || time > length)
        {
            return 0f;
        }

        float level = attack > 0.0 && time < attack ? (float)(time / attack) : 1f;
        if (decay > 0.0)
        {
            level *= (float)Math.Exp(-(time - Math.Min(time, attack)) / decay);
        }

        double remaining = length - time;
        if (remaining < release)
        {
            level *= (float)(remaining / release);
        }

        return Math.Clamp(level, 0f, 1f);
    }

    /// <summary>
    /// A 15-bit maximal-length shift register, the classic chiptune noise source: tap 1 XOR
    /// tap 0, shifted in at the top.
    /// </summary>
    public sealed class NoiseSource
    {
        private ushort _register;

        /// <summary>Creates the source with a non-zero seed.</summary>
        /// <param name="seed">Any value; 0 is replaced by 1 since the register must not be empty.</param>
        public NoiseSource(ushort seed = 1)
        {
            _register = seed == 0 ? (ushort)1 : seed;
        }

        /// <summary>The next value, −1 or +1.</summary>
        public float Next()
        {
            int bit = (_register ^ (_register >> 1)) & 1;
            _register = (ushort)((_register >> 1) | (bit << 14));
            return (_register & 1) == 0 ? -1f : 1f;
        }
    }

    private static double Fraction(double value)
    {
        double f = value - Math.Floor(value);
        return f < 0.0 ? f + 1.0 : f;
    }
}
