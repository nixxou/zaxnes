namespace Zax.Nes.Tests;

/// <summary>
/// The bit-plane encoding of a tile record, which is also the recognition key of the
/// conversion engine and must be respected to the bit (REQ-CORE-034, REQ-CORE-036).
/// </summary>
public sealed class TileRecordTests
{
    [Fact]
    public void REQ_CORE_034_the_acceptance_vector_of_the_plane_encoding()
    {
        // Plane 0 = 80 00 00 00 00 00 00 01, plane 1 = 00 x 8.
        byte[] plane0Rows = [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        ulong plane0 = TileRecord.PackPlane(plane0Rows);
        ulong plane1 = TileRecord.PackPlane(new byte[8]);

        Assert.Equal(0x0100000000000080UL, plane0);
        Assert.Equal(0UL, plane1);

        var record = new TileRecord(plane0, plane1, 0, 0, 0, false, false, false, 0, true);

        // Source pixel (0,0) is 1, (7,7) is 1, everything else is 0.
        Assert.Equal(1, record.SourcePixel(0, 0));
        Assert.Equal(1, record.SourcePixel(7, 7));

        for (int row = 0; row < 8; row++)
        {
            for (int column = 0; column < 8; column++)
            {
                bool lit = (row, column) is (0, 0) or (7, 7);
                Assert.Equal(lit ? 1 : 0, record.SourcePixel(column, row));
            }
        }
    }

    [Fact]
    public void REQ_CORE_034_bit_7_of_a_row_byte_is_the_leftmost_pixel()
    {
        // Row 0 = $80: only column 0 is set.
        ulong plane0 = TileRecord.PackPlane([0x80, 0, 0, 0, 0, 0, 0, 0]);
        var record = new TileRecord(plane0, 0, 0, 0, 0, false, false, false, 0, true);

        Assert.Equal(1, record.SourcePixel(0, 0));
        Assert.Equal(0, record.SourcePixel(7, 0));

        // Row 0 = $01: only column 7.
        plane0 = TileRecord.PackPlane([0x01, 0, 0, 0, 0, 0, 0, 0]);
        record = record with { Plane0 = plane0 };

        Assert.Equal(0, record.SourcePixel(0, 0));
        Assert.Equal(1, record.SourcePixel(7, 0));
    }

    [Fact]
    public void REQ_CORE_034_the_two_planes_give_colours_zero_to_three()
    {
        // Row 0: plane 0 = %11000000, plane 1 = %10100000 → colours 3, 1, 2, 0, ...
        ulong plane0 = TileRecord.PackPlane([0b1100_0000, 0, 0, 0, 0, 0, 0, 0]);
        ulong plane1 = TileRecord.PackPlane([0b1010_0000, 0, 0, 0, 0, 0, 0, 0]);
        var record = new TileRecord(plane0, plane1, 0, 0, 0, false, false, false, 0, true);

        Assert.Equal(3, record.SourcePixel(0, 0));
        Assert.Equal(1, record.SourcePixel(1, 0));
        Assert.Equal(2, record.SourcePixel(2, 0));
        Assert.Equal(0, record.SourcePixel(3, 0));
    }

    [Fact]
    public void REQ_CORE_034_planes_are_stored_without_flips_applied()
    {
        ulong plane0 = TileRecord.PackPlane([0x80, 0, 0, 0, 0, 0, 0, 0]);

        var plain = new TileRecord(plane0, 0, 0, 0, 0, false, false, false, 0, false);
        var flippedH = plain with { MirrorH = true };
        var flippedV = plain with { MirrorV = true };

        // The stored planes are identical: only the lookup changes.
        Assert.Equal(plain.Plane0, flippedH.Plane0);
        Assert.Equal(plain.Plane0, flippedV.Plane0);

        Assert.Equal(1, plain.DisplayedPixel(0, 0));
        Assert.Equal(1, flippedH.DisplayedPixel(7, 0));
        Assert.Equal(0, flippedH.DisplayedPixel(0, 0));
        Assert.Equal(1, flippedV.DisplayedPixel(0, 7));
        Assert.Equal(0, flippedV.DisplayedPixel(0, 0));

        var flippedBoth = plain with { MirrorH = true, MirrorV = true };
        Assert.Equal(1, flippedBoth.DisplayedPixel(7, 7));
    }

    [Fact]
    public void REQ_CORE_036_an_empty_record_is_still_a_record()
    {
        var empty = new TileRecord(0, 0, 8, 16, 5, false, false, false, 0, true);

        Assert.True(empty.IsEmpty);
        Assert.Equal(8, empty.X);
        Assert.Equal(16, empty.Y);
        Assert.Equal(5, empty.Index);
    }

    [Fact]
    public void REQ_CORE_034_the_minimal_rom_patterns_match_the_notation_of_section_5_1()
    {
        // M($0000, 3): plane 0 = $0303030303030303, plane 1 = $FCFC...
        (ulong plane0, ulong plane1) = TestRoms.Pattern(0, 3);
        Assert.Equal(0x0303030303030303UL, plane0);
        Assert.Equal(0xFCFCFCFCFCFCFCFCUL, plane1);

        // M($1000, $21): plane 0 = $2121..., plane 1 = $DEDE... (vector V2).
        (plane0, plane1) = TestRoms.Pattern(1, 0x21);
        Assert.Equal(0x2121212121212121UL, plane0);
        Assert.Equal(0xDEDEDEDEDEDEDEDEUL, plane1);

        // Tile 0 of table $0000 is entirely zero.
        (plane0, plane1) = TestRoms.Pattern(0, 0);
        Assert.Equal(0UL, plane0);
        Assert.Equal(0UL, plane1);
    }

    [Fact]
    public void REQ_CORE_034_out_of_range_coordinates_are_rejected()
    {
        var record = new TileRecord(0, 0, 0, 0, 0, false, false, false, 0, true);

        Assert.Throws<ArgumentOutOfRangeException>(() => record.SourcePixel(8, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => record.SourcePixel(0, 8));
        Assert.Throws<ArgumentOutOfRangeException>(() => record.DisplayedPixel(8, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => TileRecord.PackPlane(new byte[7]));
    }
}
