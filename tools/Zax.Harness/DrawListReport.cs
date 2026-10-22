using System.Globalization;
using System.Text;
using Zax.Engine3D;
using Zax.Profiles;

namespace Zax.Harness;

/// <summary>
/// A text report of the draw list of one frame, grouped by 3D object: the material path a tile
/// takes (SPEC-06 REQ-SCN-034), the palette table it addresses and the NES colours its texels
/// end up on. It is the measurement tool of the colour-fidelity comparison of lot L7b.
/// </summary>
internal static class DrawListReport
{
    /// <summary>Builds the report of <paramref name="list"/>.</summary>
    /// <param name="list">The draw list of the displayed frame.</param>
    public static string Describe(FrameDrawList list)
    {
        ArgumentNullException.ThrowIfNull(list);

        Dictionary<string, Group> groups = [];
        foreach (DrawTile tile in list.Tiles)
        {
            FrameInstance instance = tile.Instance;
            string pattern = instance.Shape.ReferencePattern is { } reference
                ? string.Create(
                    CultureInfo.InvariantCulture,
                    $"p{reference.RegistrationIndex}({reference.Width}x{reference.Height})")
                : "p-";
            string key = string.Create(
                CultureInfo.InvariantCulture,
                $"{pattern}.{instance.Index} bg={instance.Background} pal={tile.PaletteTable}"
                    + $" fmt={tile.TextureFormat} mat={tile.Material} layer={instance.Layer}");

            if (!groups.TryGetValue(key, out Group? group))
            {
                group = new Group(key, tile);
                groups.Add(key, group);
            }

            group.Add(tile, list.Palette);
            group.Measure(tile);
        }

        StringBuilder text = new();
        text.Append(CultureInfo.InvariantCulture, $"{list.Tiles.Count} tile(s), {groups.Count} group(s)\n");
        foreach (Group group in groups.Values.OrderByDescending(g => g.Tiles))
        {
            text.Append(CultureInfo.InvariantCulture, $"  {group.Key,-64} {group.Tiles,5} tiles  ");
            text.Append(group.Describe());
            text.Append('\n');
            text.Append(CultureInfo.InvariantCulture, $"  {string.Empty,-64} {group.DescribeDepth()}\n");
        }

        return text.ToString();
    }

    private sealed class Group(string key, DrawTile first)
    {
        private readonly DrawTile _first = first;
        private readonly Dictionary<int, int> _colors = [];
        private float _zMin = float.PositiveInfinity;
        private float _zMax = float.NegativeInfinity;

        public string Key { get; } = key;

        public int Tiles { get; private set; }

        public void Add(in DrawTile tile, ReadOnlySpan<Rgb24> palette)
        {
            Tiles++;
            if (tile.Texture is not { } texture || Tiles > 4)
            {
                return;
            }

            for (int y = 0; y < Math.Min(texture.Height, 64); y++)
            {
                for (int x = 0; x < Math.Min(texture.Width, 64); x++)
                {
                    int packed = texture.Format == TextureFormat.Direct
                        ? texture.Color(x, y).Packed
                        : palette[
                            (Math.Clamp(tile.PaletteTable, 0, 15) * 4)
                                + ObjectTexture.PaletteColumn(texture.Texel(x, y))].Packed;
                    _colors[packed] = _colors.GetValueOrDefault(packed) + 1;
                }
            }
        }

        /// <summary>
        /// Accumulates the scene z the group's meshes actually occupy, so that a capture can be
        /// read against the depth interval of REQ-E3D-205 and the placement of REQ-E3D-405/406.
        /// </summary>
        public void Measure(in DrawTile tile)
        {
            if (tile.Mesh is not { IsEmpty: false } mesh)
            {
                return;
            }

            Vec3 min = mesh.BoundsMin;
            Vec3 max = mesh.BoundsMax;
            for (int corner = 0; corner < 8; corner++)
            {
                Vec3 local = new(
                    (corner & 1) == 0 ? min.X : max.X,
                    (corner & 2) == 0 ? min.Y : max.Y,
                    (corner & 4) == 0 ? min.Z : max.Z);
                float z = tile.Transform.TransformPoint(local).Z;
                _zMin = MathF.Min(_zMin, z);
                _zMax = MathF.Max(_zMax, z);
            }
        }

        /// <summary>The placement line of the group: what REQ-E3D-205 feeds and what comes out.</summary>
        public string DescribeDepth()
        {
            FrameInstance instance = _first.Instance;
            return string.Create(
                CultureInfo.InvariantCulture,
                $"depth {instance.Depth:F1}  offset ({instance.Offset.X:F1}, {instance.Offset.Y:F1}, {instance.Offset.Z:F1})"
                    + $"  pivot ({instance.Pivot.X:F2}, {instance.Pivot.Y:F2}, {instance.Pivot.Z:F2})"
                    + $"  scale ({instance.Scale.X:F2}, {instance.Scale.Y:F2}, {instance.Scale.Z:F2})"
                    + $"  Pos.z {instance.Pos.Z:F1}  Ctr.z {instance.Center.Z:F1}"
                    + $"  drawn z [{_zMin:F1}, {_zMax:F1}]");
        }

        public string Describe()
        {
            string size = _first.Texture is { } texture
                ? string.Create(CultureInfo.InvariantCulture, $"{texture.Width}x{texture.Height}")
                : "no texture";
            string colors = string.Join(
                " ",
                _colors.OrderByDescending(pair => pair.Value)
                    .Take(5)
                    .Select(pair => string.Create(
                        CultureInfo.InvariantCulture,
                        $"#{pair.Key:X6}x{pair.Value}")));
            return $"tex {size}  {colors}";
        }
    }
}
