namespace Zax.Profiles.Wire;

/// <summary>
/// The canonical writer of SPEC-03 REQ-PROF-051: fields in ascending number order, except
/// <c>TexRect</c> which writes its payload (field 10) before its width and height; scalars
/// always written, nulls included; strings written when not null, even empty; null messages and
/// empty lists left out; repeated fields never packed; minimal varints; reference keys handed
/// out from the identity of the objects; floats copied bit for bit.
/// </summary>
/// <remarks>
/// "Scalars always written" is what a profile built in memory gets. For a profile that came
/// from a file, the writer follows the field set the decoder recorded in
/// <see cref="ProfileWireLayout"/>, which is what makes the rewrite byte-identical for files
/// written by older versions of the original program, whose settings and patterns leave
/// trailing fields out.
/// </remarks>
internal sealed class ProfileWireEncoder
{
    private readonly ProtoWriter _writer;
    private readonly ProfileWireLayout? _layout;
    private readonly Dictionary<object, int> _keys = new(ReferenceEqualityComparer.Instance);
    private int _nextKey = 1;

    internal ProfileWireEncoder(ProfileWireLayout? layout, int initialCapacity)
    {
        _layout = layout;
        _writer = new ProtoWriter(initialCapacity);
    }

    internal byte[] Write(GameProfile profile)
    {
        WriteRoot(profile);
        return _writer.ToArray();
    }

    // Q-010: no layout means "a profile built in memory", and every scalar is written.
    private bool Allowed(object node, int fieldNumber)
        => _layout is null || _layout.IsPresent(node, fieldNumber);

    private void WriteRoot(GameProfile profile)
    {
        if (profile.Signature is not null && Allowed(profile, ProfileSchema.Root.Type))
        {
            _writer.WriteString(ProfileSchema.Root.Type, profile.Signature);
        }

        if (profile.VersionText is not null && Allowed(profile, ProfileSchema.Root.Ver))
        {
            _writer.WriteString(ProfileSchema.Root.Ver, profile.VersionText);
        }

        foreach (ProfileScript script in profile.Scripts)
        {
            int scope = _writer.BeginNested(ProfileSchema.Root.Scripts);
            WriteScript(script);
            _writer.EndNested(scope);
        }

        foreach (ProfilePattern pattern in profile.PermanentPatterns)
        {
            WriteEnvelope(ProfileSchema.Root.Patterns, pattern, WritePattern);
        }

        foreach (ProfilePattern pattern in profile.TemporaryPatterns)
        {
            WriteEnvelope(ProfileSchema.Root.TempPatterns, pattern, WritePattern);
        }

        if (profile.Settings is not null && Allowed(profile, ProfileSchema.Root.Settings))
        {
            int scope = _writer.BeginNested(ProfileSchema.Root.Settings);
            WriteSettings(profile.Settings);
            _writer.EndNested(scope);
        }

        WriteOptionalString(profile, ProfileSchema.Root.Id, profile.Id);
        WriteOptionalString(profile, ProfileSchema.Root.Title, profile.Title);
        WriteOptionalString(profile, ProfileSchema.Root.Maker, profile.Author);
        WriteOptionalString(profile, ProfileSchema.Root.GameDev, profile.Publisher);
        WriteOptionalString(profile, ProfileSchema.Root.Des, profile.Description);
    }

    private void WriteScript(ProfileScript script)
    {
        WriteOptionalString(script, ProfileSchema.Script.Name, script.Name);
        if (Allowed(script, ProfileSchema.Script.Enable))
        {
            _writer.WriteBool(ProfileSchema.Script.Enable, script.Enabled);
        }

        if (Allowed(script, ProfileSchema.Script.Error))
        {
            _writer.WriteBool(ProfileSchema.Script.Error, script.StoredErrorFlag);
        }

        WriteOptionalString(script, ProfileSchema.Script.Source, script.Source);

        // Field 5 repeats the name (REQ-PROF-038); both fields are written from the model.
        WriteOptionalString(script, ProfileSchema.Script.NameAgain, script.Name);
        WriteOptionalString(script, ProfileSchema.Script.TagStr, script.TagString);
    }

    private void WritePattern(ProfilePattern pattern)
    {
        if (pattern.Grid is not null
            && pattern.Grid.Cells.Count > 0
            && Allowed(pattern, ProfileSchema.Pattern.Grid))
        {
            foreach (ProfileTile? cell in pattern.Grid.Cells)
            {
                _writer.WriteGroupStart(ProfileSchema.Pattern.Grid);
                if (cell is not null)
                {
                    WriteEnvelope(ProfileSchema.Cell.Envelope, cell, WriteCell);
                }

                _writer.WriteGroupEnd(ProfileSchema.Pattern.Grid);
            }
        }

        if (Allowed(pattern, ProfileSchema.Pattern.ConstraintSize))
        {
            WriteVec2I(ProfileSchema.Pattern.ConstraintSize, pattern.ConstraintSize);
        }

        if (pattern.TileSize is { } tileSize && Allowed(pattern, ProfileSchema.Pattern.TileSize))
        {
            WriteVec2I(ProfileSchema.Pattern.TileSize, tileSize);
        }

        if (Allowed(pattern, ProfileSchema.Pattern.Permanent))
        {
            _writer.WriteBool(ProfileSchema.Pattern.Permanent, pattern.Permanent);
        }

        if (Allowed(pattern, ProfileSchema.Pattern.PixelCount))
        {
            _writer.WriteInt32(ProfileSchema.Pattern.PixelCount, pattern.PixelCount);
        }

        if (Allowed(pattern, ProfileSchema.Pattern.ColorFlags))
        {
            _writer.WriteInt32(ProfileSchema.Pattern.ColorFlags, (int)pattern.ColorFlags);
        }

        if (Allowed(pattern, ProfileSchema.Pattern.TileDensity))
        {
            _writer.WriteInt32(ProfileSchema.Pattern.TileDensity, (int)pattern.TileDensity);
        }

        if (Allowed(pattern, ProfileSchema.Pattern.PixelDensity))
        {
            _writer.WriteInt32(ProfileSchema.Pattern.PixelDensity, (int)pattern.PixelDensity);
        }

        foreach (PatternVariant variant in pattern.Variants)
        {
            WriteEnvelope(ProfileSchema.Pattern.Variants, variant, WriteVariant);
        }

        if (Allowed(pattern, ProfileSchema.Pattern.SelfConnected))
        {
            _writer.WriteBool(ProfileSchema.Pattern.SelfConnected, pattern.SelfConnected);
        }

        if (Allowed(pattern, ProfileSchema.Pattern.SkipGeo))
        {
            _writer.WriteBool(ProfileSchema.Pattern.SkipGeo, pattern.SkipGeometry);
        }
    }

    private void WriteCell(ProfileTile tile)
    {
        int scope = _writer.BeginNested(ProfileSchema.Cell.Planes);
        if (Allowed(tile, ProfileSchema.TilePlanes.LowPlane))
        {
            _writer.WriteInt64(ProfileSchema.TilePlanes.LowPlane, TilePlaneCipher.Encode(tile.LowPlane));
        }

        if (Allowed(tile, ProfileSchema.TilePlanes.HighPlane))
        {
            _writer.WriteInt64(ProfileSchema.TilePlanes.HighPlane, TilePlaneCipher.Encode(tile.HighPlane));
        }

        if (Allowed(tile, ProfileSchema.TilePlanes.Flip))
        {
            _writer.WriteInt32(ProfileSchema.TilePlanes.Flip, (int)tile.Flip);
        }

        _writer.EndNested(scope);
    }

    private void WriteVariant(PatternVariant variant)
    {
        if (Allowed(variant, ProfileSchema.Variant.SizeZ))
        {
            _writer.WriteInt32(ProfileSchema.Variant.SizeZ, variant.Depth);
        }

        if (Allowed(variant, ProfileSchema.Variant.Geo))
        {
            _writer.WriteInt32(ProfileSchema.Variant.Geo, (int)variant.Geometry);
        }

        if (Allowed(variant, ProfileSchema.Variant.AntiShadow))
        {
            _writer.WriteBool(ProfileSchema.Variant.AntiShadow, variant.AntiShadow);
        }

        if (Allowed(variant, ProfileSchema.Variant.Flip))
        {
            _writer.WriteBool(ProfileSchema.Variant.Flip, variant.Flip);
        }

        if (Allowed(variant, ProfileSchema.Variant.Solid))
        {
            _writer.WriteBool(ProfileSchema.Variant.Solid, variant.Solid);
        }

        if (Allowed(variant, ProfileSchema.Variant.FullWarp))
        {
            _writer.WriteBool(ProfileSchema.Variant.FullWarp, variant.FullWarp);
        }

        if (Allowed(variant, ProfileSchema.Variant.Layer))
        {
            _writer.WriteInt32(ProfileSchema.Variant.Layer, (int)variant.Layer);
        }

        WriteOptionalVec3(variant, ProfileSchema.Variant.Scale, variant.Scale);
        WriteOptionalVec3(variant, ProfileSchema.Variant.Rot, variant.Rotation);
        WriteOptionalVec3(variant, ProfileSchema.Variant.Offset, variant.Offset);
        WriteOptionalVec3(variant, ProfileSchema.Variant.Pivot, variant.Pivot);

        if (Allowed(variant, ProfileSchema.Variant.Alpha))
        {
            _writer.WriteSingle(ProfileSchema.Variant.Alpha, variant.Alpha);
        }

        if (Allowed(variant, ProfileSchema.Variant.MeshAlgorithm))
        {
            _writer.WriteInt32(ProfileSchema.Variant.MeshAlgorithm, (int)variant.MeshAlgorithm);
        }

        WriteOptionalVec3(variant, ProfileSchema.Variant.DeltaAmp, variant.DeformAmplitude);
        WriteOptionalVec3(variant, ProfileSchema.Variant.DeltaPivot, variant.DeformPivot);
        WriteOptionalMat4(variant, ProfileSchema.Variant.DeltaSpeed, variant.DeformSpeed);
        WriteOptionalMat4(variant, ProfileSchema.Variant.DeltaOffset, variant.DeformOffset);

        if (Allowed(variant, ProfileSchema.Variant.RenderStyle))
        {
            _writer.WriteInt32(ProfileSchema.Variant.RenderStyle, (int)variant.RenderStyle);
        }

        if (variant.Texture is not null && Allowed(variant, ProfileSchema.Variant.Tex))
        {
            int scope = _writer.BeginNested(ProfileSchema.Variant.Tex);
            WriteTexture(variant.Texture);
            _writer.EndNested(scope);
        }

        WriteOptionalString(variant, ProfileSchema.Variant.TagStr, variant.TagString);

        if (Allowed(variant, ProfileSchema.Variant.EmptyColor))
        {
            _writer.WriteInt32(ProfileSchema.Variant.EmptyColor, variant.StoredEmptyColor);
        }
    }

    private void WriteTexture(VariantTexture texture)
    {
        // The only field written out of order in the whole format (REQ-PROF-013).
        if (texture.Pixels is not null && Allowed(texture, ProfileSchema.TexRect.Payload))
        {
            int scope = _writer.BeginNested(ProfileSchema.TexRect.Payload);
            WriteTexturePixels(texture.Pixels);
            _writer.EndNested(scope);
        }

        if (Allowed(texture, ProfileSchema.TexRect.Width))
        {
            _writer.WriteInt32(ProfileSchema.TexRect.Width, texture.Width);
        }

        if (Allowed(texture, ProfileSchema.TexRect.Height))
        {
            _writer.WriteInt32(ProfileSchema.TexRect.Height, texture.Height);
        }
    }

    private void WriteTexturePixels(TexturePixels pixels)
    {
        if (Allowed(pixels, ProfileSchema.TexPayload.Data))
        {
            foreach (int word in pixels.Data)
            {
                _writer.WriteInt32(ProfileSchema.TexPayload.Data, word);
            }
        }

        if (Allowed(pixels, ProfileSchema.TexPayload.Format))
        {
            _writer.WriteInt32(ProfileSchema.TexPayload.Format, (int)pixels.Format);
        }

        // Field 3 is left out when every value is zero. For a payload that came from a file the
        // recorded field set decides instead, so that a stored all-zero plane is written back.
        bool writeExtra = pixels.Extra.Count > 0
            && Allowed(pixels, ProfileSchema.TexPayload.Extra)
            && (_layout is not null || !pixels.ExtraIsEmpty);
        if (writeExtra)
        {
            foreach (int word in pixels.Extra)
            {
                _writer.WriteInt32(ProfileSchema.TexPayload.Extra, word);
            }
        }
    }

    private void WriteSettings(ProfileSettings settings)
    {
        if (settings.Light is not null && Allowed(settings, ProfileSchema.Settings.Light))
        {
            int scope = _writer.BeginNested(ProfileSchema.Settings.Light);
            WriteLight(settings.Light);
            _writer.EndNested(scope);
        }

        WriteBoolField(settings, ProfileSchema.Settings.BgColor, settings.UseNesBackgroundColor);

        if (settings.CustomBackgroundColor is { } customBackground
            && Allowed(settings, ProfileSchema.Settings.CustomBgColor))
        {
            WriteArgb(ProfileSchema.Settings.CustomBgColor, customBackground);
        }

        if (settings.Clipping is { } clipping && Allowed(settings, ProfileSchema.Settings.Clipping))
        {
            WriteVec4(ProfileSchema.Settings.Clipping, clipping);
        }

        WriteBoolField(settings, ProfileSchema.Settings.MapEnable, settings.MapEnable);

        if (settings.Layers is not null && Allowed(settings, ProfileSchema.Settings.Layer))
        {
            int scope = _writer.BeginNested(ProfileSchema.Settings.Layer);
            WriteLayers(settings.Layers);
            _writer.EndNested(scope);
        }

        WriteInt32Field(settings, ProfileSchema.Settings.GraphicQuality, settings.GraphicQuality);
        WriteInt32Field(settings, ProfileSchema.Settings.VrRuntime, settings.VrRuntime);
        WriteInt32Field(settings, ProfileSchema.Settings.VSync, settings.VSync);
        WriteInt32Field(settings, ProfileSchema.Settings.FrameRate, settings.FrameRate);
        WriteSingleField(settings, ProfileSchema.Settings.GameSpeed, settings.GameSpeed);
        WriteSingleField(settings, ProfileSchema.Settings.Volume, settings.Volume);
        WriteInt32Field(settings, ProfileSchema.Settings.RenderMode, (int)settings.MeshAlgorithm);

        if (Allowed(settings, ProfileSchema.Settings.SavedPattern3D))
        {
            foreach (RecordedPatternSlot slot in settings.SavedPatterns)
            {
                int scope = _writer.BeginNested(ProfileSchema.Settings.SavedPattern3D);
                WriteRecordedSlot(slot);
                _writer.EndNested(scope);
            }
        }

        if (Allowed(settings, ProfileSchema.Settings.SaveAnimPattern3D))
        {
            foreach (RecordedPatternSlot slot in settings.SavedAnimPatterns)
            {
                int scope = _writer.BeginNested(ProfileSchema.Settings.SaveAnimPattern3D);
                WriteRecordedSlot(slot);
                _writer.EndNested(scope);
            }
        }

        WriteInt32Field(settings, ProfileSchema.Settings.ControlMethod, settings.ControlMethod);
        WriteBoolField(settings, ProfileSchema.Settings.Shadow, settings.Shadow);
        WriteBoolField(settings, ProfileSchema.Settings.SameBgPalette, settings.SameBackgroundPalette);
        WriteInt32Field(settings, ProfileSchema.Settings.SpriteTolerance, settings.SpriteTolerance);
        WriteInt32Field(settings, ProfileSchema.Settings.ScanDirection, settings.ScanDirection);
        WriteInt32Field(settings, ProfileSchema.Settings.Skybox, settings.Skybox);
        WriteInt32Field(settings, ProfileSchema.Settings.Core, settings.Core);
        WriteBoolField(settings, ProfileSchema.Settings.Mirroring, settings.Mirroring);
        WriteBoolField(settings, ProfileSchema.Settings.EmptyTile, settings.EmptyTile);
        WriteBoolField(settings, ProfileSchema.Settings.LocalCam, settings.LocalCamera);
        WriteSingleField(settings, ProfileSchema.Settings.SceneRatio, settings.SceneRatio);
        WriteBoolField(settings, ProfileSchema.Settings.EmulatedMenuButton, settings.EmulatedMenuButton);
        WriteInt32Field(settings, ProfileSchema.Settings.RenderMethod, settings.RenderMethod);
        WriteInt32Field(settings, ProfileSchema.Settings.ResScaleIndex, settings.ResolutionScaleIndex);
        WriteBoolField(settings, ProfileSchema.Settings.SkipFrame, settings.SkipFrame);
        WriteBoolField(settings, ProfileSchema.Settings.BgMusic, settings.BackgroundMusic);
        WriteInt32Field(settings, ProfileSchema.Settings.AntiAlias, settings.AntiAlias);
        WriteBoolField(settings, ProfileSchema.Settings.AutoTvMode, settings.AutoTvMode);
        WriteInt32Field(settings, ProfileSchema.Settings.TvMode, (int)settings.TvMode);
        WriteInt32Field(settings, ProfileSchema.Settings.TvBox, settings.TvBox);
        WriteInt32Field(settings, ProfileSchema.Settings.SortOption, settings.SortOption);
        WriteBoolField(settings, ProfileSchema.Settings.SameSpPalette, settings.SameSpritePalette);
        WriteSingleField(settings, ProfileSchema.Settings.Fov, settings.Fov);
    }

    private void WriteLight(LightSettings light)
    {
        WriteOptionalVec3(light, ProfileSchema.Light.Direction, light.Direction);
        if (light.Color is { } color && Allowed(light, ProfileSchema.Light.Color))
        {
            WriteArgb(ProfileSchema.Light.Color, color);
        }

        if (Allowed(light, ProfileSchema.Light.Intensity))
        {
            _writer.WriteSingle(ProfileSchema.Light.Intensity, light.Intensity);
        }
    }

    private void WriteLayers(LayerSettingsSet layers)
    {
        if (Allowed(layers, ProfileSchema.Layers.Items))
        {
            foreach (LayerSettings layer in layers.Layers)
            {
                int scope = _writer.BeginNested(ProfileSchema.Layers.Items);
                WriteLayerParams(layer);
                _writer.EndNested(scope);
            }
        }

        if (Allowed(layers, ProfileSchema.Layers.DefLayer))
        {
            _writer.WriteInt32(ProfileSchema.Layers.DefLayer, (int)layers.DefaultLayer);
        }

        if (Allowed(layers, ProfileSchema.Layers.Auto))
        {
            _writer.WriteBool(ProfileSchema.Layers.Auto, layers.Auto);
        }
    }

    private void WriteLayerParams(LayerSettings layer)
    {
        if (Allowed(layer, ProfileSchema.LayerParams.Offset))
        {
            _writer.WriteSingle(ProfileSchema.LayerParams.Offset, layer.Offset);
        }

        if (Allowed(layer, ProfileSchema.LayerParams.Aligned))
        {
            _writer.WriteInt32(ProfileSchema.LayerParams.Aligned, (int)layer.Aligned);
        }

        if (Allowed(layer, ProfileSchema.LayerParams.CastShadow))
        {
            _writer.WriteBool(ProfileSchema.LayerParams.CastShadow, layer.StoredCastShadow);
        }

        if (Allowed(layer, ProfileSchema.LayerParams.RecvShadow))
        {
            _writer.WriteBool(ProfileSchema.LayerParams.RecvShadow, layer.StoredRecvShadow);
        }
    }

    private void WriteRecordedSlot(RecordedPatternSlot slot)
    {
        WriteOptionalString(slot, ProfileSchema.RecordedSlot.Name, slot.Name);
        if (slot.Pattern is not null && Allowed(slot, ProfileSchema.RecordedSlot.Pattern))
        {
            int scope = _writer.BeginNested(ProfileSchema.RecordedSlot.Pattern);
            WritePattern(slot.Pattern);
            _writer.EndNested(scope);
        }
    }

    private void WriteEnvelope<T>(int fieldNumber, T value, Action<T> writeMessage)
        where T : class
    {
        int envelope = _writer.BeginNested(fieldNumber);
        if (_keys.TryGetValue(value, out int existingKey))
        {
            _writer.WriteInt32(ProfileSchema.Envelope.ExistingKey, existingKey);
        }
        else
        {
            // A new object takes its key before the objects it contains (REQ-PROF-011).
            int key = _nextKey++;
            _keys[value] = key;
            _writer.WriteInt32(ProfileSchema.Envelope.NewKey, key);
            int message = _writer.BeginNested(ProfileSchema.Envelope.Message);
            writeMessage(value);
            _writer.EndNested(message);
        }

        _writer.EndNested(envelope);
    }

    private void WriteOptionalString(object node, int fieldNumber, string? value)
    {
        if (value is not null && Allowed(node, fieldNumber))
        {
            _writer.WriteString(fieldNumber, value);
        }
    }

    private void WriteOptionalVec3(object node, int fieldNumber, Vec3F? value)
    {
        if (value is { } vector && Allowed(node, fieldNumber))
        {
            int scope = _writer.BeginNested(fieldNumber);
            _writer.WriteSingle(ProfileSchema.Values.X, vector.X);
            _writer.WriteSingle(ProfileSchema.Values.Y, vector.Y);
            _writer.WriteSingle(ProfileSchema.Values.Z, vector.Z);
            _writer.EndNested(scope);
        }
    }

    private void WriteOptionalMat4(object node, int fieldNumber, Mat4F? value)
    {
        if (value is not { } matrix || !Allowed(node, fieldNumber))
        {
            return;
        }

        int scope = _writer.BeginNested(fieldNumber);
        for (int cell = 0; cell < Mat4F.CellCount; cell++)
        {
            _writer.WriteSingle(ProfileSchema.Values.Mat4FirstCell + cell, matrix[cell]);
        }

        _writer.EndNested(scope);
    }

    private void WriteVec2I(int fieldNumber, Vec2I value)
    {
        int scope = _writer.BeginNested(fieldNumber);
        _writer.WriteInt32(ProfileSchema.Values.X, value.X);
        _writer.WriteInt32(ProfileSchema.Values.Y, value.Y);
        _writer.EndNested(scope);
    }

    private void WriteVec4(int fieldNumber, Vec4F value)
    {
        int scope = _writer.BeginNested(fieldNumber);
        _writer.WriteSingle(ProfileSchema.Values.X, value.X);
        _writer.WriteSingle(ProfileSchema.Values.Y, value.Y);
        _writer.WriteSingle(ProfileSchema.Values.Z, value.Z);
        _writer.WriteSingle(ProfileSchema.Values.W, value.W);
        _writer.EndNested(scope);
    }

    private void WriteArgb(int fieldNumber, ArgbColor value)
    {
        int scope = _writer.BeginNested(fieldNumber);
        _writer.WriteInt32(ProfileSchema.Values.ArgbValue, value.Packed);
        _writer.EndNested(scope);
    }

    private void WriteInt32Field(object node, int fieldNumber, int value)
    {
        if (Allowed(node, fieldNumber))
        {
            _writer.WriteInt32(fieldNumber, value);
        }
    }

    private void WriteBoolField(object node, int fieldNumber, bool value)
    {
        if (Allowed(node, fieldNumber))
        {
            _writer.WriteBool(fieldNumber, value);
        }
    }

    private void WriteSingleField(object node, int fieldNumber, float value)
    {
        if (Allowed(node, fieldNumber))
        {
            _writer.WriteSingle(fieldNumber, value);
        }
    }
}
