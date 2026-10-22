namespace Zax.Profiles.Wire;

/// <summary>
/// Field numbers and wire conventions of the profile format (SPEC-03 §4). The message names
/// are the spec's own names of convenience; only the numbers travel on the wire.
/// </summary>
internal static class ProfileSchema
{
    /// <summary>Root message, <c>ProfileFile</c> (§4.1).</summary>
    internal static class Root
    {
        internal const int Type = 1;
        internal const int Ver = 2;
        internal const int Scripts = 3;
        internal const int Patterns = 4;
        internal const int TempPatterns = 5;
        internal const int Settings = 6;
        internal const int Id = 7;
        internal const int Title = 8;
        internal const int Maker = 9;
        internal const int GameDev = 10;
        internal const int Des = 11;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);

        /// <summary>The subset read for the library header (REQ-PROF-062).</summary>
        internal static readonly int[] HeaderFields = [Type, Id, Title, Maker, GameDev, Des];
    }

    /// <summary>Reference envelope, <c>RefEnvelope</c> (REQ-PROF-010).</summary>
    internal static class Envelope
    {
        internal const int ExistingKey = 1;
        internal const int NewKey = 2;
        internal const int Message = 10;
    }

    /// <summary>Script message (REQ-PROF-038).</summary>
    internal static class Script
    {
        internal const int Name = 1;
        internal const int Enable = 2;
        internal const int Error = 3;
        internal const int Source = 4;
        internal const int NameAgain = 5;
        internal const int TagStr = 6;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3, 4, 5, 6);
    }

    /// <summary>Pattern message (§4.3).</summary>
    internal static class Pattern
    {
        internal const int Grid = 1;
        internal const int ConstraintSize = 2;
        internal const int TileSize = 3;
        internal const int Permanent = 4;
        internal const int PixelCount = 5;
        internal const int ColorFlags = 6;
        internal const int TileDensity = 7;
        internal const int PixelDensity = 8;
        internal const int Variants = 10;
        internal const int SelfConnected = 11;
        internal const int SkipGeo = 12;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12);
    }

    /// <summary>Grid cell: a group that holds one reference envelope, or nothing (REQ-PROF-012).</summary>
    internal static class Cell
    {
        internal const int Envelope = 1;
        internal const int Planes = 1;
    }

    /// <summary>Tile planes message (REQ-PROF-030, REQ-PROF-031).</summary>
    internal static class TilePlanes
    {
        internal const int LowPlane = 1;
        internal const int HighPlane = 2;
        internal const int Flip = 3;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3);
    }

    /// <summary>3D variant message, <c>ShapeParams</c> (§4.4).</summary>
    internal static class Variant
    {
        internal const int SizeZ = 1;
        internal const int Geo = 2;
        internal const int AntiShadow = 3;
        internal const int Flip = 4;
        internal const int Solid = 5;
        internal const int FullWarp = 6;
        internal const int Layer = 7;
        internal const int Scale = 8;
        internal const int Rot = 9;
        internal const int Offset = 10;
        internal const int Pivot = 11;
        internal const int Alpha = 12;
        internal const int MeshAlgorithm = 13;
        internal const int DeltaAmp = 14;
        internal const int DeltaPivot = 15;
        internal const int DeltaSpeed = 16;
        internal const int DeltaOffset = 17;
        internal const int RenderStyle = 18;
        internal const int Tex = 19;
        internal const int TagStr = 20;
        internal const int EmptyColor = 23;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 23);
    }

    /// <summary>Texture rectangle, <c>TexRect</c> (§4.5). Field 10 is written first.</summary>
    internal static class TexRect
    {
        internal const int Width = 1;
        internal const int Height = 2;
        internal const int Payload = 10;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 10);
    }

    /// <summary>Texture payload, <c>TexPayload</c> (§4.5).</summary>
    internal static class TexPayload
    {
        internal const int Data = 1;
        internal const int Format = 2;
        internal const int Extra = 3;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3);
    }

    /// <summary>Embedded settings message (§4.6).</summary>
    internal static class Settings
    {
        internal const int Light = 1;
        internal const int BgColor = 2;
        internal const int CustomBgColor = 3;
        internal const int Clipping = 4;
        internal const int MapEnable = 5;
        internal const int Layer = 6;
        internal const int GraphicQuality = 7;
        internal const int VrRuntime = 8;
        internal const int VSync = 9;
        internal const int FrameRate = 10;
        internal const int GameSpeed = 11;
        internal const int Volume = 12;
        internal const int RenderMode = 13;
        internal const int SavedPattern3D = 14;
        internal const int SaveAnimPattern3D = 15;
        internal const int ControlMethod = 16;
        internal const int Shadow = 17;
        internal const int SameBgPalette = 18;
        internal const int SpriteTolerance = 19;
        internal const int ScanDirection = 20;
        internal const int Skybox = 21;
        internal const int Core = 22;
        internal const int Mirroring = 23;
        internal const int EmptyTile = 24;
        internal const int LocalCam = 25;
        internal const int SceneRatio = 26;
        internal const int EmulatedMenuButton = 27;
        internal const int RenderMethod = 28;
        internal const int ResScaleIndex = 29;
        internal const int SkipFrame = 30;
        internal const int BgMusic = 31;
        internal const int AntiAlias = 32;
        internal const int AutoTvMode = 33;
        internal const int TvMode = 34;
        internal const int TvBox = 35;
        internal const int SortOption = 36;
        internal const int SameSpPalette = 37;
        internal const int Fov = 38;

        internal const int FirstField = 1;
        internal const int LastField = 38;

        internal static readonly ulong Mask = BuildRangeMask(FirstField, LastField);
    }

    /// <summary>Light sub-message (§4.6).</summary>
    internal static class Light
    {
        internal const int Direction = 1;
        internal const int Color = 2;
        internal const int Intensity = 3;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3);
    }

    /// <summary>Layers sub-message (§4.6).</summary>
    internal static class Layers
    {
        internal const int Items = 1;
        internal const int DefLayer = 2;
        internal const int Auto = 3;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3);
    }

    /// <summary>Parameters of one layer (§4.6).</summary>
    internal static class LayerParams
    {
        internal const int Offset = 1;
        internal const int Aligned = 2;
        internal const int CastShadow = 3;
        internal const int RecvShadow = 4;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 2, 3, 4);
    }

    /// <summary>Memorised slot, <c>RecordedSlot</c> (REQ-PROF-044). Field 2 does not exist.</summary>
    internal static class RecordedSlot
    {
        internal const int Name = 1;
        internal const int Pattern = 3;

        internal static readonly ulong Mask = ProfileWireLayout.MaskOf(1, 3);
    }

    /// <summary>Base value types (§4.7).</summary>
    internal static class Values
    {
        internal const int X = 1;
        internal const int Y = 2;
        internal const int Z = 3;
        internal const int W = 4;
        internal const int ArgbValue = 1;
        internal const int Mat4FirstCell = 1;
    }

    private static ulong BuildRangeMask(int first, int last)
    {
        ulong mask = 0;
        for (int field = first; field <= last; field++)
        {
            mask |= ProfileWireLayout.Bit(field);
        }

        return mask;
    }
}
