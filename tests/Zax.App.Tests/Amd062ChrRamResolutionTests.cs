using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Profiles.Hash;
using Zax.Profiles.Tests;

namespace Zax.App.Tests;

/// <summary>
/// The in-game resolution of a « 3dh » profile driven by a real core on a cartridge with
/// CHR-RAM (AMD-062 REQ-PROF-080, SPEC-02 REQ-CORE-123).
/// </summary>
/// <remarks>
/// The criterion of REQ-PROF-080 has the second tile « appear only on screen (sprite) ». On a
/// mapper 0 cartridge with CHR-RAM that cannot happen: the pattern data a sprite is drawn from
/// <b>is</b> the CHR-RAM, and the PPU reads it at a 16-byte aligned address, so the stateless
/// scan of REQ-PROF-080 always holds it too. The two paths are therefore separated here: this
/// run shows the CHR-RAM path on a real core, and
/// <c>Zax.Engine3D.Tests.Amd062TileResolutionTests</c> shows the frame-tile path with no CHR-RAM
/// source attached, which is exactly « only on screen ». See Q-178.
/// </remarks>
public sealed class Amd062ChrRamResolutionTests
{
    /// <summary>Where the first tile is written: aligned, and outside what the PPU shows.</summary>
    private const ushort FirstTileAddress = 0x1000;

    /// <summary>Where the second tile is written: pattern 1 of the sprite table at $0000.</summary>
    private const ushort SecondTileAddress = 0x0010;

    private static byte[] RawOne => RawTile.FromPlanes(0x0F0F_0F0F_0F0F_0F0FUL, 0x00FF_00FF_00FF_00FFUL);

    private static byte[] RawTwo => RawTile.FromPlanes(0x1234_5678_9ABC_DEF0UL, 0x0FED_CBA9_8765_4321UL);

    // REQ-PROF-080: the first tile is resolved on the very frame the test writes it into
    // CHR-RAM, the second on the frame it is written and shown, and neither before.
    [Fact]
    public void A_tile_written_into_the_chr_ram_is_resolved_on_that_frame()
    {
        Run run = new();

        for (int frame = 0; frame < 10; frame++)
        {
            run.Frame();
            Assert.Equal(2, run.Tiles.PendingCount);
        }

        // Frame 10: the first tile goes into CHR-RAM, mirrored vertically and aligned.
        run.WriteTile(FirstTileAddress, RawTile.Flipped(RawOne, TileFlip.Vertical));
        run.Frame();
        Assert.Equal(1, run.Tiles.PendingCount);
        Assert.Equal(1, run.Engine.TilesResolvedWhilePlaying);

        for (int frame = 11; frame < 20; frame++)
        {
            run.Frame();
            Assert.Equal(1, run.Tiles.PendingCount);
        }

        // Frame 20: the second tile appears, as the pattern of a displayed sprite.
        run.WriteTile(SecondTileAddress, RawTwo);
        run.ShowSprite(tile: 1, x: 64, y: 64);
        run.Frame();

        Assert.Equal(0, run.Tiles.PendingCount);
        Assert.Equal(2, run.Engine.TilesResolvedWhilePlaying);
        Assert.Equal(2, run.Engine.Patterns.Permanent[0].Images.Count);
    }

    // REQ-PROF-080: resolution is session knowledge. A rollback of five frames after frame 20
    // puts neither tile back to pending.
    [Fact]
    public void A_rollback_puts_no_tile_back_to_pending()
    {
        Run run = new();
        byte[]? snapshot = null;

        for (int frame = 0; frame <= 25; frame++)
        {
            if (frame == 16)
            {
                // The state the rollback of five frames will come back to.
                snapshot = run.Core.SaveState(includeRomImage: false);
            }

            if (frame == 10)
            {
                run.WriteTile(FirstTileAddress, RawTile.Flipped(RawOne, TileFlip.Vertical));
            }

            if (frame == 20)
            {
                run.WriteTile(SecondTileAddress, RawTwo);
                run.ShowSprite(tile: 1, x: 64, y: 64);
            }

            run.Frame();
        }

        Assert.Equal(0, run.Tiles.PendingCount);

        Assert.True(run.Core.TryLoadState(snapshot!, 0, out _, out _, out string? error), error);
        run.Engine.ClearFrames();

        Assert.Equal(0, run.Tiles.PendingCount);
        foreach (ProfileTile? cell in run.Profile.PermanentPatterns[0].Grid!.Cells)
        {
            Assert.False(cell!.IsPending);
        }

        // The tiles are still there after a reset as well (REQ-CORE-122).
        run.Core.Reset();
        run.Frame();
        Assert.Equal(0, run.Tiles.PendingCount);
    }

    // REQ-ARCH-012 as REQ-PROF-080 states it: at equal initial resolved set — an empty cache —
    // two runs of the same ROM, profile and inputs give the same frames.
    [Fact]
    public void Two_runs_with_an_empty_cache_give_the_same_frames()
    {
        List<string> first = Capture();
        List<string> second = Capture();

        Assert.Equal(second.Count, first.Count);
        Assert.Equal(second, first);
    }

    private static List<string> Capture()
    {
        Run run = new();
        List<string> frames = [];
        for (int frame = 0; frame <= 25; frame++)
        {
            if (frame == 10)
            {
                run.WriteTile(FirstTileAddress, RawTile.Flipped(RawOne, TileFlip.Vertical));
            }

            if (frame == 20)
            {
                run.WriteTile(SecondTileAddress, RawTwo);
                run.ShowSprite(tile: 1, x: 64, y: 64);
            }

            run.Frame();
            ConversionFrame converted = run.Engine.ConvertingFrame;
            frames.Add(
                $"{frame}:{run.Tiles.PendingCount}:{converted.Shapes.Count}:{converted.Instances.Count}");
        }

        return frames;
    }

    /// <summary>
    /// One run: a core on a mapper 0 cartridge with 32 KiB of CHR-RAM, and an engine loaded
    /// with a « 3dh » profile whose two tiles are nowhere in the ROM file.
    /// </summary>
    private sealed class Run
    {
        private int _counter;

        public Run()
        {
            TileId one = TileId.Compute(RawOne);
            TileId two = TileId.Compute(RawTwo);
            byte[] bytes = SyntheticHashProfile.Build(
                manifest: [one, two],
                tileKeys: [one.ToText(), two.ToText()]);

            ProfileLoadResult result = ProfileFormat.Load(bytes);
            Assert.True(result.IsSuccess, result.Message);
            Profile = result.Document!.Profile;
            Tiles = result.Document.Tiles!;

            Core = new NesCore();
            Assert.True(Core.LoadRom(ChrRamRom(), "ChrRam.nes").Success);

            // REQ-PROF-078 and REQ-PROF-082: the ROM holds neither tile and the cache is
            // disabled, so both start pending.
            Tiles.PrepareForRom(Profile.Id, ChrRamRom(), ResolvedTileCache.Disabled);
            Assert.Equal(2, Tiles.PendingCount);

            Engine = new ConversionEngine { ChrRamSource = new CoreChrRam(Core) };
            Engine.LoadProfile(Profile, applySettings: false, Tiles);
            Core.VideoCallback = OnFrame;
            Core.Reset();
        }

        public NesCore Core { get; }

        public ConversionEngine Engine { get; }

        public GameProfile Profile { get; }

        public HashProfileTiles Tiles { get; }

        public void Frame()
        {
            Core.RunFrame(0, FrameOutput.All);
            _counter++;
        }

        /// <summary>Writes sixteen bytes at a PPU address through $2006 and $2007.</summary>
        public void WriteTile(ushort address, byte[] raw)
        {
            _ = Core.Bus.Read(0x2002);
            Core.Bus.Write(0x2000, 0x00);
            Core.Bus.Write(0x2006, (byte)(address >> 8));
            Core.Bus.Write(0x2006, (byte)address);
            foreach (byte value in raw)
            {
                Core.Bus.Write(0x2007, value);
            }
        }

        /// <summary>Puts sprite 0 on screen and turns sprite rendering on.</summary>
        public void ShowSprite(byte tile, byte x, byte y)
        {
            Core.Bus.Write(0x2003, 0x00);
            Core.Bus.Write(0x2004, (byte)(y - 1));
            Core.Bus.Write(0x2004, tile);
            Core.Bus.Write(0x2004, 0x00);
            Core.Bus.Write(0x2004, x);

            // Sprite palette entry 1 must not be the backdrop, or the sprite draws nothing.
            Core.Bus.Write(0x2006, 0x3F);
            Core.Bus.Write(0x2006, 0x11);
            Core.Bus.Write(0x2007, 0x21);
            Core.Bus.Write(0x2006, 0x3F);
            Core.Bus.Write(0x2006, 0x12);
            Core.Bus.Write(0x2007, 0x11);

            Core.Bus.Write(0x2000, 0x00);
            Core.Bus.Write(0x2001, 0x10);
        }

        private void OnFrame(in FrameBatch batch) => Engine.Convert(batch, _counter);

        /// <summary>A mapper 0 image with no CHR-ROM: 32 KiB of CHR-RAM (REQ-CORE-012).</summary>
        private static byte[] ChrRamRom()
        {
            const int prgSize = 2 * 16 * 1024;
            byte[] image = new byte[16 + prgSize];
            image[0] = 0x4E;
            image[1] = 0x45;
            image[2] = 0x53;
            image[3] = 0x1A;
            image[4] = 0x02;
            image[5] = 0x00;
            image[6] = 0x01;

            // JMP $8000: the program does nothing, the test drives the PPU itself.
            image[16] = 0x4C;
            image[17] = 0x00;
            image[18] = 0x80;

            int vectors = 16 + prgSize - 6;
            for (int vector = 0; vector < 3; vector++)
            {
                image[vectors + (2 * vector)] = 0x00;
                image[vectors + (2 * vector) + 1] = 0x80;
            }

            return image;
        }
    }
}
