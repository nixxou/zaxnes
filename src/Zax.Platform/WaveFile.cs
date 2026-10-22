using System.Buffers.Binary;

namespace Zax.Platform;

/// <summary>Decoded PCM audio: interleaved samples in <c>[−1 ; 1]</c>.</summary>
/// <param name="Samples">Interleaved frames, <paramref name="Channels"/> values per frame.</param>
/// <param name="Channels">1 for mono, 2 for stereo.</param>
/// <param name="SampleRate">Frames per second as the file declares them.</param>
public sealed record WaveAudio(float[] Samples, int Channels, int SampleRate)
{
    /// <summary>Frames in the clip.</summary>
    public int FrameCount => Channels == 0 ? 0 : Samples.Length / Channels;

    /// <summary>Duration in seconds.</summary>
    public double Duration => SampleRate == 0 ? 0.0 : (double)FrameCount / SampleRate;
}

/// <summary>
/// Reads the RIFF/WAVE files of <c>assets-neutral/audio/</c> (SPEC-11 §2): three menu tracks and
/// four interface sounds, all PCM at 44 100 Hz.
/// </summary>
/// <remarks>
/// Uncompressed PCM (8, 16, 24 and 32-bit integer) and IEEE float are accepted, which covers
/// everything the asset folder holds; any other codec is refused. Nothing here talks to the
/// audio device, so the decoder is testable without one.
/// </remarks>
public static class WaveFile
{
    private const int FormatPcm = 1;
    private const int FormatIeeeFloat = 3;
    private const int FormatExtensible = 0xFFFE;

    /// <summary>Reads a file from disk.</summary>
    /// <exception cref="InvalidDataException">The file is not a PCM or float WAVE.</exception>
    public static WaveAudio Load(string path) => Decode(File.ReadAllBytes(path));

    /// <summary>Reads a file already in memory.</summary>
    /// <exception cref="InvalidDataException">The file is not a PCM or float WAVE.</exception>
    public static WaveAudio Decode(ReadOnlySpan<byte> file)
    {
        if (file.Length < 12
            || !file[..4].SequenceEqual("RIFF"u8)
            || !file.Slice(8, 4).SequenceEqual("WAVE"u8))
        {
            throw new InvalidDataException("Not a RIFF/WAVE file.");
        }

        int format = 0;
        int channels = 0;
        int sampleRate = 0;
        int bitsPerSample = 0;
        ReadOnlySpan<byte> data = default;
        bool sawData = false;

        int offset = 12;
        while (offset + 8 <= file.Length)
        {
            ReadOnlySpan<byte> id = file.Slice(offset, 4);
            uint size = BinaryPrimitives.ReadUInt32LittleEndian(file.Slice(offset + 4, 4));
            int body = offset + 8;
            int available = file.Length - body;
            int length = size > (uint)available ? available : (int)size;

            if (id.SequenceEqual("fmt "u8) && length >= 16)
            {
                ReadOnlySpan<byte> chunk = file.Slice(body, length);
                format = BinaryPrimitives.ReadUInt16LittleEndian(chunk);
                channels = BinaryPrimitives.ReadUInt16LittleEndian(chunk[2..]);
                sampleRate = (int)BinaryPrimitives.ReadUInt32LittleEndian(chunk[4..]);
                bitsPerSample = BinaryPrimitives.ReadUInt16LittleEndian(chunk[14..]);

                if (format == FormatExtensible && length >= 26)
                {
                    // The real format sits in the first two bytes of the GUID.
                    format = BinaryPrimitives.ReadUInt16LittleEndian(chunk[24..]);
                }
            }
            else if (id.SequenceEqual("data"u8))
            {
                data = file.Slice(body, length);
                sawData = true;
            }

            // Chunks are padded to even sizes.
            offset = body + length + (length & 1);
        }

        if (!sawData || channels is < 1 or > 2 || sampleRate <= 0)
        {
            throw new InvalidDataException("The WAVE file has no usable fmt or data chunk.");
        }

        return new WaveAudio(Convert(data, format, bitsPerSample), channels, sampleRate);
    }

    private static float[] Convert(ReadOnlySpan<byte> data, int format, int bitsPerSample)
    {
        switch (format, bitsPerSample)
        {
            case (FormatPcm, 8):
            {
                float[] samples = new float[data.Length];
                for (int i = 0; i < samples.Length; i++)
                {
                    // 8-bit PCM is unsigned, centred on 128.
                    samples[i] = (data[i] - 128) / 128f;
                }

                return samples;
            }

            case (FormatPcm, 16):
            {
                int count = data.Length / 2;
                float[] samples = new float[count];
                for (int i = 0; i < count; i++)
                {
                    samples[i] = BinaryPrimitives.ReadInt16LittleEndian(data.Slice(i * 2, 2)) / 32768f;
                }

                return samples;
            }

            case (FormatPcm, 24):
            {
                int count = data.Length / 3;
                float[] samples = new float[count];
                for (int i = 0; i < count; i++)
                {
                    int value = data[(i * 3) + 0] | (data[(i * 3) + 1] << 8) | (data[(i * 3) + 2] << 16);
                    if ((value & 0x800000) != 0)
                    {
                        value |= unchecked((int)0xFF000000);
                    }

                    samples[i] = value / 8_388_608f;
                }

                return samples;
            }

            case (FormatPcm, 32):
            {
                int count = data.Length / 4;
                float[] samples = new float[count];
                for (int i = 0; i < count; i++)
                {
                    samples[i] = BinaryPrimitives.ReadInt32LittleEndian(data.Slice(i * 4, 4)) / 2_147_483_648f;
                }

                return samples;
            }

            case (FormatIeeeFloat, 32):
            {
                int count = data.Length / 4;
                float[] samples = new float[count];
                for (int i = 0; i < count; i++)
                {
                    samples[i] = BinaryPrimitives.ReadSingleLittleEndian(data.Slice(i * 4, 4));
                }

                return samples;
            }

            default:
                throw new InvalidDataException(
                    $"Unsupported WAVE encoding: format {format}, {bitsPerSample} bits per sample.");
        }
    }
}
