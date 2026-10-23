// Dumps golden data of the C# NES core over the public test ROMs, for the Rust port.
// Usage: golden-core <roms dir> <out.jsonl> [frames]
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Zax.Nes;
using Zax.Nes.Rom;

string romsDir = args[0];
string outPath = args[1];
int frames = args.Length > 2 ? int.Parse(args[2]) : 1200;
int[] checkpoints = [1, 60, 300, 600, 1200, 1800, 3600];

using var writer = new StreamWriter(outPath, false, new UTF8Encoding(false)) { NewLine = "\n" };
var files = Directory.GetFiles(romsDir, "*.nes", SearchOption.AllDirectories).OrderBy(f => f, StringComparer.Ordinal).ToList();
int n = 0;
foreach (string file in files)
{
    string rel = Path.GetRelativePath(romsDir, file).Replace(Path.DirectorySeparatorChar, '/');
    var record = new Dictionary<string, object?> { ["rom"] = rel, ["rom_sha256"] = Hex(SHA256.HashData(File.ReadAllBytes(file))) };
    try
    {
        var core = new NesCore();
        RomLoadResult load = core.LoadRom(File.ReadAllBytes(file), Path.GetFileNameWithoutExtension(file));
        record["load_ok"] = load.Success;
        if (!load.Success)
        {
            record["load_failure"] = load.Failure.ToString();
            record["load_error"] = load.Error;
            writer.WriteLine(JsonSerializer.Serialize(record));
            continue;
        }

        record["region"] = load.Region.ToString();
        record["sample_rate"] = core.SampleRate;
        core.Reset();
        using var audioHash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        float[] buf = new float[8192];
        long samples = 0;
        var frameHashes = new Dictionary<string, string>();
        var tileHashes = new Dictionary<string, string>();
        var cycles = new Dictionary<string, long>();
        var audioAt = new Dictionary<string, string>();
        for (int f = 1; f <= frames; f++)
        {
            core.RunFrame(0);
            int got;
            while ((got = core.ReadAudio(buf)) > 0)
            {
                byte[] q = new byte[got * 2];
                for (int i = 0; i < got; i++)
                {
                    int v = (int)Math.Round(Math.Clamp(buf[i], -1f, 1f) * 32767f);
                    q[2 * i] = (byte)v;
                    q[2 * i + 1] = (byte)(v >> 8);
                }

                audioHash.AppendData(q);
                samples += got;
                if (got < buf.Length) break;
            }

            if (checkpoints.Contains(f))
            {
                string k = f.ToString();
                frameHashes[k] = Hex(SHA256.HashData(core.PixelFrame));
                tileHashes[k] = TileHash(core.TileRecords);
                cycles[k] = core.CycleCount;
                audioAt[k] = Hex(audioHash.GetCurrentHash()) + ":" + samples;
            }
        }

        record["frames"] = frames;
        record["pixel_sha256"] = frameHashes;
        record["tiles_sha256"] = tileHashes;
        record["cycles"] = cycles;
        record["audio_i16_sha256_samples"] = audioAt;

        // blargg protocol: $6001-$6003 = DE B0 61, $6000 = status, text from $6004
        if (core.ReadMemory(0x6001) == 0xDE && core.ReadMemory(0x6002) == 0xB0 && core.ReadMemory(0x6003) == 0x61)
        {
            record["blargg_status"] = core.ReadMemory(0x6000);
            var sb = new StringBuilder();
            for (ushort a = 0x6004; a < 0x7000; a++)
            {
                byte b = core.ReadMemory(a);
                if (b == 0) break;
                sb.Append((char)b);
            }

            record["blargg_text"] = sb.ToString();
        }
    }
    catch (Exception e)
    {
        record["exception"] = e.GetType().Name + ": " + e.Message;
    }

    writer.WriteLine(JsonSerializer.Serialize(record));
    n++;
}

Console.WriteLine($"golden-core: {n} ROMs run, {files.Count} files, {frames} frames each -> {outPath}");

static string Hex(byte[] b) => Convert.ToHexString(b).ToLowerInvariant();

static string TileHash(ReadOnlySpan<TileRecord> records)
{
    // Canonical little-endian serialisation of every field, in order.
    var bytes = new List<byte>(records.Length * 32);
    foreach (TileRecord r in records)
    {
        bytes.AddRange(BitConverter.GetBytes(r.Plane0));
        bytes.AddRange(BitConverter.GetBytes(r.Plane1));
        bytes.AddRange(BitConverter.GetBytes(r.X));
        bytes.AddRange(BitConverter.GetBytes(r.Y));
        bytes.AddRange(BitConverter.GetBytes(r.Index));
        bytes.Add((byte)((r.MirrorH ? 1 : 0) | (r.MirrorV ? 2 : 0) | (r.Hidden ? 4 : 0) | (r.Background ? 8 : 0)));
        bytes.Add(r.Palette);
    }

    return records.Length + ":" + Hex(SHA256.HashData(bytes.ToArray()));
}
