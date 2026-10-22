using Zax.Render.Assets;

namespace Zax.Render.Tests;

/// <summary>
/// The baseline JPEG decoder AMD-025 asks for, written from ITU-T T.81: one of the six textures
/// of the television model is a JPEG.
/// </summary>
/// <remarks>
/// The files below are assembled byte by byte from the standard's own structures — a
/// quantisation table, two Huffman tables, a frame header and a scan — so the test needs no
/// encoder and no sample file. The shipped model's own JPEG is decoded too when it is there.
/// </remarks>
public sealed class JpegTests
{
    [Fact] // AMD-025
    public void AMD_025_a_baseline_grey_jpeg_decodes_to_the_level_shift_of_the_standard()
    {
        // Every block holds a zero DC and an immediate end-of-block, so every coefficient is
        // zero: T.81 §A.3.1 shifts the samples by +128, which is mid grey.
        byte[] file = Jpeg.Grey(16, 16, [0, 0, 0, 0]);
        ImageData image = JpegImage.Decode(file);

        Assert.Equal(16, image.Width);
        Assert.Equal(16, image.Height);
        Assert.All(
            Enumerable.Range(0, 16 * 16),
            at => Assert.Equal((128, 128, 128, 255), image.Pixel(at % 16, at / 16)));
    }

    [Fact] // AMD-025
    public void AMD_025_the_dc_coefficient_of_each_block_sets_its_level()
    {
        // Four blocks, each with its own DC difference: the decoder has to keep the running
        // predictor of §F.2.1.3.1 and to scale the inverse transform by a quarter. With a
        // quantisation table of eights, a quantised DC of d comes out as d above the mid grey.
        byte[] file = Jpeg.Grey(16, 16, [3, -1, -2, 0]);
        ImageData image = JpegImage.Decode(file);

        // Blocks are stored in MCU order, left to right then top to bottom.
        Assert.Equal(131, image.Pixel(3, 3).R);       // DC 3
        Assert.Equal(130, image.Pixel(11, 3).R);      // DC 3 - 1
        Assert.Equal(128, image.Pixel(3, 11).R);      // DC 2 - 2
        Assert.Equal(128, image.Pixel(11, 11).R);     // DC unchanged
    }

    [Fact] // AMD-025
    public void AMD_025_the_decoder_refuses_what_it_cannot_read()
    {
        Assert.False(JpegImage.IsJpeg("not a jpeg"u8));
        Assert.Throws<InvalidDataException>(() => JpegImage.Decode("not a jpeg"u8));

        // Progressive files are rejected rather than half-decoded.
        byte[] progressive = Jpeg.Grey(16, 16, [0, 0, 0, 0], frameMarker: 0xC2);
        Assert.Throws<InvalidDataException>(() => JpegImage.Decode(progressive));
    }

    [Fact] // AMD-025
    public void AMD_025_the_jpeg_texture_of_the_shipped_model_decodes()
    {
        if (TestTelevision.Gltf is not { } model)
        {
            return;
        }

        // One of the six images of the model is a JPEG; whichever it is, every image came
        // back as a real picture rather than a flat placeholder. A metallic-roughness map
        // holds its two channels in green and blue, so all three are looked at.
        Assert.All(model.Images, image =>
        {
            Assert.True(image.Width >= 64);
            Assert.True(image.Height >= 64);

            HashSet<int> levels = [];
            for (int y = 0; y < image.Height; y += 16)
            {
                for (int x = 0; x < image.Width; x += 16)
                {
                    (byte r, byte g, byte b, _) = image.Pixel(x, y);
                    levels.Add((r << 16) | (g << 8) | b);
                }
            }

            Assert.True(levels.Count > 4, "a decoded texture should not be a flat colour");
        });
    }

    /// <summary>
    /// A baseline, Huffman-coded, single-component JPEG, built from the structures of T.81:
    /// SOI, DQT, SOF0, two DHT, SOS and EOI.
    /// </summary>
    private static class Jpeg
    {
        /// <summary>
        /// A grey image whose blocks carry the DC differences <paramref name="dcDeltas"/> and
        /// no AC coefficient at all.
        /// </summary>
        /// <param name="width">Width in pixels, a multiple of 8.</param>
        /// <param name="height">Height in pixels, a multiple of 8.</param>
        /// <param name="dcDeltas">One DC difference per block, in MCU order.</param>
        /// <param name="frameMarker">The frame marker, 0xC0 for a baseline file.</param>
        public static byte[] Grey(int width, int height, int[] dcDeltas, byte frameMarker = 0xC0)
        {
            List<byte> file = [0xFF, 0xD8];

            // DQT: table 0, 8-bit, all eights, which cancels the 1/8 of the inverse transform
            // on the DC term — a quantised DC of d is then d levels above the mid grey.
            List<byte> quant = [0x00];
            quant.AddRange(Enumerable.Repeat((byte)8, 64));
            Segment(file, 0xDB, quant);

            // SOF: 8-bit samples, one component, no subsampling, quantisation table 0.
            Segment(file, frameMarker,
            [
                8,
                (byte)(height >> 8), (byte)height,
                (byte)(width >> 8), (byte)width,
                1,
                1, 0x11, 0,
            ]);

            // DHT, DC table 0: three codes of two bits, for the symbols 0, 1 and 2 — the
            // number of extra bits the difference is written on (§F.1.2.1).
            Segment(file, 0xC4, [0x00, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2]);

            // DHT, AC table 0: one code of two bits, the end-of-block symbol.
            Segment(file, 0xC4, [0x10, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00]);

            // SOS: one component, DC table 0 and AC table 0, the baseline spectral selection.
            Segment(file, 0xDA, [1, 1, 0x00, 0, 63, 0]);

            BitWriter bits = new();
            foreach (int delta in dcDeltas)
            {
                int size = Size(delta);

                // The DC symbol is the number of bits, on the two-bit code of the table.
                bits.Write(size, 2);
                if (size > 0)
                {
                    // The difference itself, in the one's-complement form of §F.1.2.1.
                    int value = delta > 0 ? delta : delta + (1 << size) - 1;
                    bits.Write(value, size);
                }

                // End of block: the only AC code of the table, 0b00.
                bits.Write(0, 2);
            }

            file.AddRange(bits.Finish());
            file.AddRange([0xFF, 0xD9]);
            return [.. file];
        }

        /// <summary>The category of §F.1.2.1: how many bits the difference needs.</summary>
        private static int Size(int value)
        {
            int magnitude = Math.Abs(value);
            int size = 0;
            while (magnitude > 0)
            {
                size++;
                magnitude >>= 1;
            }

            return size;
        }

        private static void Segment(List<byte> file, byte marker, IReadOnlyList<byte> payload)
        {
            file.Add(0xFF);
            file.Add(marker);
            int length = payload.Count + 2;
            file.Add((byte)(length >> 8));
            file.Add((byte)length);
            file.AddRange(payload);
        }

        private sealed class BitWriter
        {
            private readonly List<byte> _bytes = [];
            private int _current;
            private int _count;

            public void Write(int value, int length)
            {
                for (int bit = length - 1; bit >= 0; bit--)
                {
                    _current = (_current << 1) | ((value >> bit) & 1);
                    _count++;
                    if (_count == 8)
                    {
                        Flush();
                    }
                }
            }

            public IReadOnlyList<byte> Finish()
            {
                while (_count != 0)
                {
                    // §F.1.2.3 pads the last byte with ones.
                    _current = (_current << 1) | 1;
                    _count++;
                    if (_count == 8)
                    {
                        Flush();
                    }
                }

                return _bytes;
            }

            private void Flush()
            {
                byte value = (byte)_current;
                _bytes.Add(value);

                // Byte stuffing: a 0xFF in the entropy-coded data is followed by a zero.
                if (value == 0xFF)
                {
                    _bytes.Add(0x00);
                }

                _current = 0;
                _count = 0;
            }
        }
    }
}
