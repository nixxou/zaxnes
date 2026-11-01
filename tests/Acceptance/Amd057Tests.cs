using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-057 — the walls between two neighbouring tiles are kept in the drawn mesh, and the depth
/// nudge of a sprite tile follows its index. The four automatic criteria of the amendment: the
/// whole box of REQ-E3D-501 on tiles that touch, the joints of REQ-SCN-065 with the walls
/// present, the amended TV-W2 of REQ-SCN-037, and two overlapping sprites settled by their
/// index.
/// </summary>
/// <remarks>
/// The depth nudge is the renderer's alone (DEC-06-Q4, DEC-R2-T4), so it is read where the
/// renderer puts it: on the scene positions <see cref="SceneExport"/> computes, which run the
/// very chain the vertex shader runs.
/// </remarks>
public sealed class Amd057Tests
{
    private static readonly Vector2 Viewport = new(640f, 480f);

    /// <summary>
    /// AMD-057 §2, first criterion — the drawn mesh of a tile is its whole mesh of
    /// REQ-E3D-501: a full tile is a closed box of 24 vertices and 12 triangles whether or not
    /// a neighbouring tile of the same object covers one of its walls.
    /// </summary>
    [Fact] // AMD-057, REQ-E3D-501, REQ-E3D-540
    public void Two_full_tiles_of_one_object_that_touch_each_keep_their_walls()
    {
        // A 3 x 3 block of full tiles in one shape: the middle one has all four of its walls
        // covered by a neighbour, the edges have two or three.
        SceneState scene = new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 8, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(3), column: 14, row: 14, columns: 3, rows: 3)
            .Build();

        List<DrawTile> tiles = [.. scene.DrawList!.Tiles];
        Assert.Equal(9, tiles.Count);

        // One object: the sealing variants of AMD-027 would have given the middle tile 8
        // vertices and the corners 16.
        Assert.Single(tiles.Select(static tile => tile.Instance).Distinct());
        foreach (DrawTile tile in tiles)
        {
            Assert.NotNull(tile.Mesh);
            Assert.Equal(24, tile.Mesh.VertexCount);
            Assert.Equal(12, tile.Mesh.TriangleCount);
        }
    }

    /// <summary>
    /// AMD-057 §2, amended TV-W2 — the front face of a sprite tile in <c>L1 FRONT</c>, whose
    /// depth interval is <c>[0, D]</c> (REQ-E3D-403), lands at <c>0.001 × (i mod 64) − 0.1</c>:
    /// −0.1 for the index 0, −0.095 for the index 5, and −0.095 again for the index 69, the
    /// lower half of the 8 × 16 whose upper half is the index 5.
    /// </summary>
    [Fact] // AMD-057, REQ-SCN-037, REQ-E3D-405, TV-W2
    public void Tvw2_the_front_face_of_a_sprite_tile_follows_its_index()
    {
        Assert.Equal(0.1f, SceneConstants.SpriteNudge);
        Assert.Equal(0.001f, SceneConstants.SpriteIndexNudge);

        Assert.Equal(-0.1f, FrontFaceOfSprite(index: 0), 5);
        Assert.Equal(-0.095f, FrontFaceOfSprite(index: 5), 5);
        Assert.Equal(-0.095f, FrontFaceOfSprite(index: 69), 5);

        // The same tile as a background tile is not nudged at all (REQ-SCN-037).
        Assert.Equal(0f, FrontFaceOfSprite(index: null), 5);
    }

    /// <summary>
    /// AMD-057 §2 — a sprite "behind the background" takes the same step the other way:
    /// <c>0.001 × (i mod 64) + 0.1</c>, so the lower index is still the nearer one.
    /// </summary>
    [Fact] // AMD-057, REQ-SCN-037
    public void A_sprite_behind_the_background_steps_the_same_way_from_plus_one_tenth()
    {
        Assert.Equal(0.1f, FrontFaceOfSprite(index: 0, behind: true), 5);
        Assert.Equal(0.105f, FrontFaceOfSprite(index: 5, behind: true), 5);
        Assert.Equal(0.105f, FrontFaceOfSprite(index: 69, behind: true), 5);
    }

    /// <summary>
    /// AMD-057 §2, second criterion — two flat sprites that overlap, of the indices 3 and 10:
    /// the sprite 3, the one the NES gives priority to, is drawn in front, and it is drawn
    /// there every frame because the two depths are apart by a fixed 0.007 unit, never equal.
    /// </summary>
    [Fact] // AMD-057, REQ-SCN-037, REQ-CORE-043
    public void Two_overlapping_flat_sprites_are_settled_by_their_index()
    {
        // Two flat sprites (D = 1) of two different images, on exactly the same cell, so they
        // overlap pixel for pixel and nothing but the index can decide between them.
        SceneState scene = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x16)
            .Palette(2, 0x12)
            .Pattern(SceneFixture.Solid(1, 0), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(2, 1), depth: 1, ProfileLayer.L1)
            .Sprite(SceneFixture.Solid(1, 0), 120, 112, palette: 0, index: 3)
            .Sprite(SceneFixture.Solid(2, 1), 120, 112, palette: 1, index: 10)
            .Build();

        List<DrawTile> tiles = [.. scene.DrawList!.Tiles];
        Assert.Equal(2, tiles.Count);

        DrawTile third = tiles.Single(static tile => tile.SpriteIndex == 3);
        DrawTile tenth = tiles.Single(static tile => tile.SpriteIndex == 10);

        // The engine hands over the same depth for both: the nudge is the renderer's.
        Assert.Equal(third.TileCenter.Z, tenth.TileCenter.Z, 5);

        CameraView view = View(scene);
        IReadOnlyList<ExportedObject> placed = SceneExport.Objects(scene, view);
        Assert.Equal(2, placed.Count);

        float near = placed[tiles.IndexOf(third)].Positions.Min(static p => p.Z);
        float far = placed[tiles.IndexOf(tenth)].Positions.Min(static p => p.Z);

        // +z goes away from the eye (REQ-SCN-001): the sprite 3 is in front of the sprite 10,
        // by the seven thousandths their indices are apart, and never at the same depth.
        Assert.Equal(-0.097f, near, 5);
        Assert.Equal(-0.09f, far, 5);
        Assert.True(near < far, "the sprite of the lower index must be the nearer one");
    }

    /// <summary>
    /// The scene <c>z</c> of the front face of one tile placed at the screen cell (15, 14) in
    /// <c>L1 FRONT</c>, as the renderer puts it.
    /// </summary>
    /// <param name="index">
    /// The sprite index of REQ-CORE-043, or null for the same tile as a background tile.
    /// </param>
    /// <param name="behind">The "behind the background" priority bit.</param>
    private static float FrontFaceOfSprite(int? index, bool behind = false)
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(3, 0x30)
            .Pattern(SceneFixture.Solid(3), depth: 4, ProfileLayer.L1);

        // L1 FRONT with D = 4 puts the tile at z in [0, 4] (REQ-E3D-403, TV-W1), so the front
        // face is at z = 0 before the renderer's nudge.
        if (index is { } sprite)
        {
            fixture.Sprite(SceneFixture.Solid(3), 120, 112, behindBackground: behind, index: sprite);
        }
        else
        {
            fixture.Tile(SceneFixture.Solid(3), 120, 112);
        }

        SceneState scene = fixture.Build();
        DrawTile tile = Assert.Single(scene.DrawList!.Tiles);
        Assert.Equal(index is not null, tile.Sprite);
        Assert.Equal(index ?? -1, tile.SpriteIndex);

        ExportedObject placed = Assert.Single(SceneExport.Objects(scene, View(scene)));
        return placed.Positions.Min(static p => p.Z);
    }

    private static CameraView View(SceneState scene)
    {
        scene.Camera.SnapToSetpoints();
        return scene.Camera.BuildView(scene.Clipping, Viewport.X / Viewport.Y);
    }
}
