namespace Zax.Profiles.Wire;

/// <summary>
/// Turns the bytes of a profile into the domain model (SPEC-03 §3.1 to §3.7).
/// </summary>
/// <remarks>
/// <para>
/// Three conventions make this more than a plain protobuf decode. Objects that may be shared —
/// patterns, 3D variants and tiles — travel inside a <b>reference envelope</b> that either
/// carries a new object with a fresh key or names a key already seen; keys are common to the
/// whole file, start at 1 and are handed out in prefix order, so an envelope whose key is out
/// of sequence or whose reference is unknown makes the file invalid (REQ-PROF-011). Grid cells
/// are <b>groups</b>, an empty one meaning an empty cell (REQ-PROF-012). Tile planes are
/// <b>masked</b> with the key of REQ-PROF-030.
/// </para>
/// <para>
/// Unknown field numbers are skipped, as the protobuf rule asks, and counted so that a test
/// can show the built-in profiles contain none (REQ-PROF-001).
/// </para>
/// </remarks>
internal sealed class ProfileWireDecoder
{
    private readonly ProtoReader _reader;
    private readonly ProfileWireLayout _layout = new();
    private readonly Dictionary<int, object> _objectsByKey = [];
    private int _nextKey = 1;
    private int _unknownFieldCount;

    internal ProfileWireDecoder(byte[] data)
    {
        _reader = new ProtoReader(data);
    }

    internal ProfileDocument ReadDocument()
    {
        GameProfile profile = ReadRoot(_reader.End);
        return new ProfileDocument(profile, _layout, _unknownFieldCount);
    }

    private GameProfile ReadRoot(int limit)
    {
        GameProfile profile = new();
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.Root.Type:
                    profile.Signature = _reader.ReadString();
                    break;
                case ProfileSchema.Root.Ver:
                    profile.VersionText = _reader.ReadString();
                    break;
                case ProfileSchema.Root.Scripts:
                    profile.Scripts.Add(ReadScript(_reader.ReadLengthLimit()));
                    break;
                case ProfileSchema.Root.Patterns:
                    profile.PermanentPatterns.Add(ReadPatternEnvelope(_reader.ReadLengthLimit()));
                    break;
                case ProfileSchema.Root.TempPatterns:
                    profile.TemporaryPatterns.Add(ReadPatternEnvelope(_reader.ReadLengthLimit()));
                    break;
                case ProfileSchema.Root.Settings:
                    profile.Settings = ReadSettings(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Root.Id:
                    profile.Id = _reader.ReadString();
                    break;
                case ProfileSchema.Root.Title:
                    profile.Title = _reader.ReadString();
                    break;
                case ProfileSchema.Root.Maker:
                    profile.Author = _reader.ReadString();
                    break;
                case ProfileSchema.Root.GameDev:
                    profile.Publisher = _reader.ReadString();
                    break;
                case ProfileSchema.Root.Des:
                    profile.Description = _reader.ReadString();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(profile, present, ProfileSchema.Root.Mask);
        return profile;
    }

    private ProfileScript ReadScript(int limit)
    {
        ProfileScript script = new(null, false, null, null);
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.Script.Name:
                    script.Name = _reader.ReadString();
                    break;
                case ProfileSchema.Script.Enable:
                    script.Enabled = _reader.ReadBool();
                    break;
                case ProfileSchema.Script.Error:
                    script.StoredErrorFlag = _reader.ReadBool();
                    break;
                case ProfileSchema.Script.Source:
                    script.Source = _reader.ReadString();
                    break;
                case ProfileSchema.Script.NameAgain:
                    // Redundant copy of the name; field 1 is authoritative (REQ-PROF-038).
                    _ = _reader.ReadString();
                    break;
                case ProfileSchema.Script.TagStr:
                    script.TagString = _reader.ReadString();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(script, present, ProfileSchema.Script.Mask);
        return script;
    }

    private ProfilePattern ReadPatternEnvelope(int limit)
        => ReadEnvelope<ProfilePattern>(limit, ReadPattern);

    private ProfilePattern ReadPattern(int limit)
    {
        ProfilePattern pattern = new();
        List<ProfileTile?>? cells = null;
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.Pattern.Grid when type == WireType.StartGroup:
                    cells ??= [];
                    cells.Add(ReadCellGroup(field));
                    break;
                case ProfileSchema.Pattern.ConstraintSize:
                    pattern.ConstraintSize = ReadVec2I(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Pattern.TileSize:
                    pattern.TileSize = ReadVec2I(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Pattern.Permanent:
                    pattern.Permanent = _reader.ReadBool();
                    break;
                case ProfileSchema.Pattern.PixelCount:
                    pattern.PixelCount = _reader.ReadInt32();
                    break;
                case ProfileSchema.Pattern.ColorFlags:
                    pattern.ColorFlags = (PatternColorFlags)_reader.ReadInt32();
                    break;
                case ProfileSchema.Pattern.TileDensity:
                    pattern.TileDensity = (PatternDensity)_reader.ReadInt32();
                    break;
                case ProfileSchema.Pattern.PixelDensity:
                    pattern.PixelDensity = (PatternDensity)_reader.ReadInt32();
                    break;
                case ProfileSchema.Pattern.Variants:
                    pattern.Variants.Add(ReadVariantEnvelope(_reader.ReadLengthLimit()));
                    break;
                case ProfileSchema.Pattern.SelfConnected:
                    pattern.SelfConnected = _reader.ReadBool();
                    break;
                case ProfileSchema.Pattern.SkipGeo:
                    pattern.SkipGeometry = _reader.ReadBool();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        if (cells is not null)
        {
            Vec2I size = pattern.TileSize ?? default;
            pattern.Grid = new PatternGrid(size.X, size.Y, cells);
        }

        _layout.Record(pattern, present, ProfileSchema.Pattern.Mask);
        return pattern;
    }

    private ProfileTile? ReadCellGroup(int groupField)
    {
        ProfileTile? tile = null;
        while (_reader.TryReadGroupTag(groupField, out int field, out WireType type))
        {
            if (field == ProfileSchema.Cell.Envelope && type == WireType.Length)
            {
                tile = ReadEnvelope<ProfileTile>(_reader.ReadLengthLimit(), ReadCell);
            }
            else
            {
                SkipUnknown(field, type);
            }
        }

        return tile;
    }

    private ProfileTile ReadCell(int limit)
    {
        ProfileTile? tile = null;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            if (field == ProfileSchema.Cell.Planes && type == WireType.Length)
            {
                tile = ReadTilePlanes(_reader.ReadLengthLimit());
            }
            else
            {
                SkipUnknown(field, type);
            }
        }

        // A cell whose planes message is absent does not occur in any built-in profile; an
        // all-transparent tile is the faithful reading of "absent message, default value".
        return tile ?? new ProfileTile(0, 0, TileFlip.None);
    }

    private ProfileTile ReadTilePlanes(int limit)
    {
        long storedLow = 0;
        long storedHigh = 0;
        int flip = 0;
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.TilePlanes.LowPlane:
                    storedLow = _reader.ReadInt64();
                    break;
                case ProfileSchema.TilePlanes.HighPlane:
                    storedHigh = _reader.ReadInt64();
                    break;
                case ProfileSchema.TilePlanes.Flip:
                    flip = _reader.ReadInt32();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        ProfileTile tile = new(
            TilePlaneCipher.Decode(storedLow),
            TilePlaneCipher.Decode(storedHigh),
            (TileFlip)flip);
        _layout.Record(tile, present, ProfileSchema.TilePlanes.Mask);
        return tile;
    }

    private PatternVariant ReadVariantEnvelope(int limit)
        => ReadEnvelope<PatternVariant>(limit, ReadVariant);

    private PatternVariant ReadVariant(int limit)
    {
        PatternVariant variant = new();
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.Variant.SizeZ:
                    variant.Depth = _reader.ReadInt32();
                    break;
                case ProfileSchema.Variant.Geo:
                    variant.Geometry = (VariantGeometry)_reader.ReadInt32();
                    break;
                case ProfileSchema.Variant.AntiShadow:
                    variant.AntiShadow = _reader.ReadBool();
                    break;
                case ProfileSchema.Variant.Flip:
                    variant.Flip = _reader.ReadBool();
                    break;
                case ProfileSchema.Variant.Solid:
                    variant.Solid = _reader.ReadBool();
                    break;
                case ProfileSchema.Variant.FullWarp:
                    variant.FullWarp = _reader.ReadBool();
                    break;
                case ProfileSchema.Variant.Layer:
                    variant.Layer = (ProfileLayer)_reader.ReadInt32();
                    break;
                case ProfileSchema.Variant.Scale:
                    variant.Scale = ReadVec3F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.Rot:
                    variant.Rotation = ReadVec3F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.Offset:
                    variant.Offset = ReadVec3F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.Pivot:
                    variant.Pivot = ReadVec3F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.Alpha:
                    variant.Alpha = _reader.ReadSingle();
                    break;
                case ProfileSchema.Variant.MeshAlgorithm:
                    variant.MeshAlgorithm = (MeshAlgorithm)_reader.ReadInt32();
                    break;
                case ProfileSchema.Variant.DeltaAmp:
                    variant.DeformAmplitude = ReadVec3F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.DeltaPivot:
                    variant.DeformPivot = ReadVec3F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.DeltaSpeed:
                    variant.DeformSpeed = ReadMat4F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.DeltaOffset:
                    variant.DeformOffset = ReadMat4F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.RenderStyle:
                    variant.RenderStyle = (VariantRenderStyle)_reader.ReadInt32();
                    break;
                case ProfileSchema.Variant.Tex:
                    variant.Texture = ReadTexture(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Variant.TagStr:
                    variant.TagString = _reader.ReadString();
                    break;
                case ProfileSchema.Variant.EmptyColor:
                    variant.StoredEmptyColor = (sbyte)_reader.ReadInt32();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(variant, present, ProfileSchema.Variant.Mask);
        return variant;
    }

    private VariantTexture ReadTexture(int limit)
    {
        VariantTexture texture = new(0, 0, null);
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.TexRect.Width:
                    texture.Width = _reader.ReadInt32();
                    break;
                case ProfileSchema.TexRect.Height:
                    texture.Height = _reader.ReadInt32();
                    break;
                case ProfileSchema.TexRect.Payload:
                    texture.Pixels = ReadTexturePixels(_reader.ReadLengthLimit());
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(texture, present, ProfileSchema.TexRect.Mask);
        return texture;
    }

    private TexturePixels ReadTexturePixels(int limit)
    {
        TexturePixels pixels = new(TextureFormat.Indexed);
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.TexPayload.Data:
                    pixels.Data.Add(_reader.ReadInt32());
                    break;
                case ProfileSchema.TexPayload.Format:
                    pixels.Format = (TextureFormat)_reader.ReadInt32();
                    break;
                case ProfileSchema.TexPayload.Extra:
                    pixels.Extra.Add((short)_reader.ReadInt32());
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(pixels, present, ProfileSchema.TexPayload.Mask);
        return pixels;
    }

    private ProfileSettings ReadSettings(int limit)
    {
        ProfileSettings settings = new();
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.Settings.Light:
                    settings.Light = ReadLight(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Settings.BgColor:
                    settings.UseNesBackgroundColor = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.CustomBgColor:
                    settings.CustomBackgroundColor = ReadArgb(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Settings.Clipping:
                    settings.Clipping = ReadVec4F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Settings.MapEnable:
                    settings.MapEnable = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.Layer:
                    settings.Layers = ReadLayers(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Settings.GraphicQuality:
                    settings.GraphicQuality = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.VrRuntime:
                    settings.VrRuntime = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.VSync:
                    settings.VSync = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.FrameRate:
                    settings.FrameRate = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.GameSpeed:
                    settings.GameSpeed = _reader.ReadSingle();
                    break;
                case ProfileSchema.Settings.Volume:
                    settings.Volume = _reader.ReadSingle();
                    break;
                case ProfileSchema.Settings.RenderMode:
                    settings.MeshAlgorithm = (GlobalMeshAlgorithm)_reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.SavedPattern3D:
                    settings.SavedPatterns.Add(ReadRecordedSlot(_reader.ReadLengthLimit()));
                    break;
                case ProfileSchema.Settings.SaveAnimPattern3D:
                    settings.SavedAnimPatterns.Add(ReadRecordedSlot(_reader.ReadLengthLimit()));
                    break;
                case ProfileSchema.Settings.ControlMethod:
                    settings.ControlMethod = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.Shadow:
                    settings.Shadow = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.SameBgPalette:
                    settings.SameBackgroundPalette = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.SpriteTolerance:
                    settings.SpriteTolerance = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.ScanDirection:
                    settings.ScanDirection = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.Skybox:
                    settings.Skybox = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.Core:
                    settings.Core = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.Mirroring:
                    settings.Mirroring = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.EmptyTile:
                    settings.EmptyTile = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.LocalCam:
                    settings.LocalCamera = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.SceneRatio:
                    settings.SceneRatio = _reader.ReadSingle();
                    break;
                case ProfileSchema.Settings.EmulatedMenuButton:
                    settings.EmulatedMenuButton = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.RenderMethod:
                    settings.RenderMethod = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.ResScaleIndex:
                    settings.ResolutionScaleIndex = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.SkipFrame:
                    settings.SkipFrame = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.BgMusic:
                    settings.BackgroundMusic = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.AntiAlias:
                    settings.AntiAlias = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.AutoTvMode:
                    settings.AutoTvMode = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.TvMode:
                    settings.TvMode = (TvRegionMode)_reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.TvBox:
                    settings.TvBox = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.SortOption:
                    settings.SortOption = _reader.ReadInt32();
                    break;
                case ProfileSchema.Settings.SameSpPalette:
                    settings.SameSpritePalette = _reader.ReadBool();
                    break;
                case ProfileSchema.Settings.Fov:
                    settings.Fov = _reader.ReadSingle();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(settings, present, ProfileSchema.Settings.Mask);
        return settings;
    }

    private LightSettings ReadLight(int limit)
    {
        LightSettings light = new();
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.Light.Direction:
                    light.Direction = ReadVec3F(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Light.Color:
                    light.Color = ReadArgb(_reader.ReadLengthLimit());
                    break;
                case ProfileSchema.Light.Intensity:
                    light.Intensity = _reader.ReadSingle();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(light, present, ProfileSchema.Light.Mask);
        return light;
    }

    private LayerSettingsSet ReadLayers(int limit)
    {
        LayerSettingsSet layers = new();
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.Layers.Items:
                    layers.Layers.Add(ReadLayerParams(_reader.ReadLengthLimit()));
                    break;
                case ProfileSchema.Layers.DefLayer:
                    layers.DefaultLayer = (ProfileLayer)_reader.ReadInt32();
                    break;
                case ProfileSchema.Layers.Auto:
                    layers.Auto = _reader.ReadBool();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(layers, present, ProfileSchema.Layers.Mask);
        return layers;
    }

    private LayerSettings ReadLayerParams(int limit)
    {
        LayerSettings layer = new();
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.LayerParams.Offset:
                    layer.Offset = _reader.ReadSingle();
                    break;
                case ProfileSchema.LayerParams.Aligned:
                    layer.Aligned = (LayerAlignment)_reader.ReadInt32();
                    break;
                case ProfileSchema.LayerParams.CastShadow:
                    layer.StoredCastShadow = _reader.ReadBool();
                    break;
                case ProfileSchema.LayerParams.RecvShadow:
                    layer.StoredRecvShadow = _reader.ReadBool();
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(layer, present, ProfileSchema.LayerParams.Mask);
        return layer;
    }

    private RecordedPatternSlot ReadRecordedSlot(int limit)
    {
        RecordedPatternSlot slot = new(null, null);
        ulong present = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            present |= ProfileWireLayout.Bit(field);
            switch (field)
            {
                case ProfileSchema.RecordedSlot.Name:
                    slot.Name = _reader.ReadString();
                    break;
                case ProfileSchema.RecordedSlot.Pattern:
                    // A direct message here: memorised slots carry no reference envelope.
                    slot.Pattern = ReadPattern(_reader.ReadLengthLimit());
                    break;
                default:
                    SkipUnknown(field, type);
                    present &= ~ProfileWireLayout.Bit(field);
                    break;
            }
        }

        _layout.Record(slot, present, ProfileSchema.RecordedSlot.Mask);
        return slot;
    }

    private Vec2I ReadVec2I(int limit)
    {
        int x = 0;
        int y = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            switch (field)
            {
                case ProfileSchema.Values.X:
                    x = _reader.ReadInt32();
                    break;
                case ProfileSchema.Values.Y:
                    y = _reader.ReadInt32();
                    break;
                default:
                    SkipUnknown(field, type);
                    break;
            }
        }

        return new Vec2I(x, y);
    }

    private Vec3F ReadVec3F(int limit)
    {
        float x = 0f;
        float y = 0f;
        float z = 0f;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            switch (field)
            {
                case ProfileSchema.Values.X:
                    x = _reader.ReadSingle();
                    break;
                case ProfileSchema.Values.Y:
                    y = _reader.ReadSingle();
                    break;
                case ProfileSchema.Values.Z:
                    z = _reader.ReadSingle();
                    break;
                default:
                    SkipUnknown(field, type);
                    break;
            }
        }

        return new Vec3F(x, y, z);
    }

    private Vec4F ReadVec4F(int limit)
    {
        float x = 0f;
        float y = 0f;
        float z = 0f;
        float w = 0f;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            switch (field)
            {
                case ProfileSchema.Values.X:
                    x = _reader.ReadSingle();
                    break;
                case ProfileSchema.Values.Y:
                    y = _reader.ReadSingle();
                    break;
                case ProfileSchema.Values.Z:
                    z = _reader.ReadSingle();
                    break;
                case ProfileSchema.Values.W:
                    w = _reader.ReadSingle();
                    break;
                default:
                    SkipUnknown(field, type);
                    break;
            }
        }

        return new Vec4F(x, y, z, w);
    }

    private Mat4F ReadMat4F(int limit)
    {
        Span<float> cells = stackalloc float[Mat4F.CellCount];
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            int index = field - ProfileSchema.Values.Mat4FirstCell;
            if ((uint)index < Mat4F.CellCount && type == WireType.Fixed32)
            {
                cells[index] = _reader.ReadSingle();
            }
            else
            {
                SkipUnknown(field, type);
            }
        }

        return new Mat4F(cells);
    }

    private ArgbColor ReadArgb(int limit)
    {
        int packed = 0;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            if (field == ProfileSchema.Values.ArgbValue)
            {
                packed = _reader.ReadInt32();
            }
            else
            {
                SkipUnknown(field, type);
            }
        }

        return new ArgbColor(packed);
    }

    /// <summary>
    /// Reads a reference envelope: field 1 names an object already seen, fields 2 and 10 carry
    /// a new one (REQ-PROF-010, REQ-PROF-011).
    /// </summary>
    private T ReadEnvelope<T>(int limit, Func<int, T> readMessage)
        where T : class
    {
        T? message = null;
        int? newKey = null;
        int? existingKey = null;
        while (_reader.TryReadTag(limit, out int field, out WireType type))
        {
            switch (field)
            {
                case ProfileSchema.Envelope.ExistingKey:
                    existingKey = _reader.ReadInt32();
                    break;
                case ProfileSchema.Envelope.NewKey:
                    newKey = _reader.ReadInt32();
                    if (newKey != _nextKey)
                    {
                        throw new ProfileFormatException(
                            $"Reference key {newKey} is out of sequence; {_nextKey} was expected.");
                    }

                    // The key belongs to this object before the objects it contains, so the
                    // counter moves on before the payload is read (prefix order).
                    _nextKey++;
                    break;
                case ProfileSchema.Envelope.Message:
                    if (newKey is null)
                    {
                        throw new ProfileFormatException(
                            "A reference envelope carries a message without a new key.");
                    }

                    message = readMessage(_reader.ReadLengthLimit());
                    break;
                default:
                    // Fields 3, 4 and 8 of the convention (dynamic type) never appear in a
                    // profile; REQ-PROF-010 lets a reader skip them.
                    SkipUnknown(field, type);
                    break;
            }
        }

        if (existingKey is not null)
        {
            if (message is not null || newKey is not null)
            {
                throw new ProfileFormatException(
                    $"Reference envelope {existingKey} both names an object and carries one.");
            }

            if (!_objectsByKey.TryGetValue(existingKey.Value, out object? target))
            {
                throw new ProfileFormatException($"Reference to the unknown key {existingKey}.");
            }

            if (target is not T typed)
            {
                throw new ProfileFormatException(
                    $"Reference key {existingKey} names a {target.GetType().Name}, not a {typeof(T).Name}.");
            }

            return typed;
        }

        if (message is null || newKey is null)
        {
            throw new ProfileFormatException("A reference envelope carries neither a key nor an object.");
        }

        _objectsByKey[newKey.Value] = message;
        return message;
    }

    private void SkipUnknown(int fieldNumber, WireType wireType)
    {
        _unknownFieldCount++;
        _reader.Skip(fieldNumber, wireType);
    }
}
