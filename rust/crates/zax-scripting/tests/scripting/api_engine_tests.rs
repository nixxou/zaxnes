//! Behaviour of the members that read and write the engine state: instances, shapes, patterns,
//! the frame queries and injections, the palette, the pinned frame and the handles a script
//! keeps (SPEC-05 §4.2, REQ-LUA-043 to REQ-LUA-048, REQ-LUA-084 to REQ-LUA-087).

use super::support::{assert_close, img, Batch, PatternSpec, ScriptProfileBuilder, ScriptSession};

/// A session whose frame holds a pattern `[A B]` tagged `t` at cells (4, 4) and (5, 4), and a
/// lone `C` sprite tagged `s`; the script runs once, in `UpdateS`.
fn session_with(script: &str) -> ScriptSession {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a(), img::b()], PatternSpec::tagged("t"))
        .pattern(&[img::c()], PatternSpec::tagged("s"))
        .script("p", script)
        .start();
    session.frame(
        &Batch::new()
            .bg_cell(img::a(), 4, 4)
            .bg_cell(img::b(), 5, 4)
            .sprite(img::c(), 100, 100),
        1,
    );
    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
    session
}

// the properties of an instance

#[test]
fn the_properties_of_an_instance_read_and_write_in_place() {
    let mut session = session_with(
        r"
        function UpdateS()
          if shape.TagStr ~= 't' then return end
          shape.Alpha = 0.5
          shape.RenderStyle = 2
          style = shape.RenderStyle
          shape.Enable = false
          shape.Layer = TLayer.L3
          shape.CastShadow = false
          shape.RecvShadow = false
          shape.antiShadow = true
          shape.flip = true
          shape.solid = true
          shape.fullWarp = true
          shape.sizeZ = 9
          shape.geo = 3
          shape.fillrate = 0.25
          shape.emptyColor = 5
          shape.customPalette = 7
          alpha, enable, hidden = shape.alpha, shape.Enable, shape.RenderStyle
          layer, cast, recv, anti = shape.layer, shape.CastShadow, shape.RecvShadow, shape.antiShadow
          flip, solid, warp, depth, geo = shape.flip, shape.solid, shape.fullWarp, shape.sizeZ, shape.geo
          fill, empty, custom, palette = shape.fillrate, shape.emptyColor, shape.customPalette, shape.Palette
          isUi = shape.UI
          shape.UI = true
          nowUi, uiLayer = shape.ui, shape.Layer
          sp, bg, index = shape.Sp, shape.Bg, shape.Index
          tags = shape:ContainsTag('t') and shape:ContainsTag('*') and not shape:ContainsTag('x')
        end
        ",
    );

    assert_eq!(session.number("return alpha"), 0.5);
    assert_eq!(session.truth("enable"), 0.0);
    assert_eq!(session.number("return style"), 2.0);
    assert_eq!(session.number("return hidden"), 0.0);
    assert_eq!(session.number("return layer"), 4.0);
    assert_eq!(session.truth("cast"), 0.0);
    assert_eq!(session.truth("recv"), 0.0);
    assert_eq!(session.truth("anti"), 1.0);
    assert_eq!(session.truth("flip and solid and warp"), 1.0);
    assert_eq!(session.number("return depth"), 9.0);
    assert_eq!(session.number("return geo"), 3.0);
    assert_eq!(session.number("return fill"), 0.25);
    assert_eq!(session.number("return empty"), 5.0);
    assert_eq!(session.number("return custom"), 7.0);
    assert_eq!(session.number("return palette"), 7.0);
    assert_eq!(session.truth("isUi"), 0.0);
    assert_eq!(session.truth("nowUi"), 1.0);
    assert_eq!(session.number("return uiLayer"), 1.0);
    assert_eq!(session.truth("sp"), 0.0);
    assert_eq!(session.truth("bg"), 1.0);
    assert_eq!(session.number("return index"), 0.0);
    assert_eq!(session.truth("tags"), 1.0);
}

#[test]
fn a_sprite_instance_reports_sprite_and_not_background() {
    let mut session = session_with(
        "function UpdateS() if shape.TagStr == 's' then sp, bg = shape.Sp, shape.Bg end end",
    );

    assert_eq!(session.truth("sp"), 1.0);
    assert_eq!(session.truth("bg"), 0.0);
}

#[test]
fn the_bound_vectors_of_an_instance_write_through_req_lua_044() {
    let mut session = session_with(
        r"
        function UpdateS()
          if shape.TagStr ~= 't' then return end
          shape.Scale.x = 2
          shape.scale[1] = 3
          shape.Rot.content = Vector3(10, 20, 30)
          shape.Pivot.z = 0.25
          shape.DeformPivot.x = 0.1
          shape.texOffset.x = 0.5
          shape.texTilting[1] = 4
          shape.texOffset.content = Vector2(1, 2)
          sx, sy = shape.Scale.x, shape.Scale.y
          rx, rz = shape.Rot.x, shape.rot.z
          pz, dp = shape.pivot.z, shape.DeformPivot.x
          tx, ty, ox, oy = shape.texTilting.x, shape.texTilting.y, shape.texOffset.x, shape.texOffset.y
          copy = shape.Offset.content
          copy.x = 99
          untouched = shape.Offset.x
          shape.CData[0] = 5
          shape.CData[4] = 6
          c0, c4, c1 = shape.CData[0], shape.CData[4], shape.CData[1]
          shape.DeformSpeed.m00 = 3
          shape.DeformOffset[5] = 4
          ds, dob = shape.DeformSpeed[0], shape.DeformOffset.m11
        end
        ",
    );

    assert_eq!(session.number("return sx"), 2.0);
    assert_eq!(session.number("return sy"), 3.0);
    assert_eq!(session.number("return rx"), 10.0);
    assert_eq!(session.number("return rz"), 30.0);
    assert_eq!(session.number("return pz"), 0.25);
    assert_close(0.1, session.number("return dp"), 5);
    assert_eq!(session.number("return tx"), 1.0);
    assert_eq!(session.number("return ty"), 4.0);
    assert_eq!(session.number("return ox"), 1.0);
    assert_eq!(session.number("return oy"), 2.0);
    assert_eq!(session.number("return untouched"), 0.0);
    assert_eq!(session.number("return c0"), 5.0);
    assert_eq!(session.number("return c4"), 6.0);
    assert_eq!(session.number("return c1"), 0.0);
    assert_eq!(session.number("return ds"), 3.0);
    assert_eq!(session.number("return dob"), 4.0);
}

#[test]
fn a_negative_custom_data_index_raises_req_lua_082() {
    let mut session = session_with("");

    let message = session
        .try_evaluate("Frame:GetShapeWithTag('t').CData[-1] = 1")
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("index -1 is out of range for CData"),
        "{message}"
    );
}

#[test]
fn the_geometry_of_a_frame_instance_is_readable_req_lua_083() {
    let mut session = session_with(
        r"
        function UpdateS()
          if shape.TagStr ~= 't' then return end
          bl, tr, size = shape.BottomLeft, shape.TopRight, shape.Size
          pos, ori, wol = shape.Pos, shape.OriPos, shape.OriPosWoLayer
          center = shape.Center
          shape.Center = Vector3(1, 2, 3)
          newCenter = shape.Center
          getPos = shape:GetPos(Vector3(0, 0, 0))
          original = shape:GetOriginalPos(Vector3(1, 1, 1))
          woLayer = shape:GetPosWoLayer(Vector3(0, 1, 0))
          shape:CalculatePos()
          afterCalc = shape.Pos
          ovRect = shape:Overlap(Rect(0, 0, 400, 400)) and shape:OverlapRect(Rect(0, 0, 400, 400))
          noRect = shape:OverlapRect(Rect(300, 300, 5, 5))
        end
        ",
    );

    assert_eq!(session.number("return size.x"), 16.0);
    assert_eq!(session.number("return size.y"), 8.0);
    assert_eq!(session.number("return bl.x"), 32.0);
    assert_eq!(session.number("return tr.x"), 47.0);
    assert_eq!(session.number("return newCenter.z"), 3.0);
    assert_eq!(session.number("return getPos.x"), 32.0);
    assert_eq!(session.number("return original.x"), 47.0);
    assert_eq!(session.number("return woLayer.x"), 32.0);
    assert_eq!(session.truth("ovRect"), 1.0);
    assert_eq!(session.truth("noRect"), 0.0);
    // The positions of a frame are computed after the hooks, so the anchor only exists once the
    // script asks for it.
    assert_eq!(session.number("return pos.x"), 0.0);
    assert_eq!(session.number("return afterCalc.x"), 40.0);
}

// the pattern-level members of an instance

#[test]
fn the_pattern_level_members_of_a_frame_instance_read_the_shape_req_lua_081() {
    let mut session = session_with(
        r"
        function UpdateS()
          if shape.TagStr ~= 't' then return end
          tw, th = shape.tSize.x, shape.tSize.y
          cw, ch = shape.constraintWidth, shape.constraintHeight
          shape.constraintWidth = 5
          shape.constraintHeight = 6
          cs = shape.constraintSize
          shape.constraintSize = IntVector2(7, 8)
          cs2 = shape.constraintSize
          sc = shape.selfConnected
          shape.selfConnected = false
          sc2 = shape.selfConnected
          shape.skipGeo = true
          skip = shape.skipGeo
          hasPattern = shape.pattern ~= nil and shape.refPattern ~= nil
          samePattern = shape.pattern == shape.Shape2D.pattern
          sameShape = shape.shape == shape.Shape2D
        end
        ",
    );

    assert_eq!(session.number("return tw"), 2.0);
    assert_eq!(session.number("return th"), 1.0);
    assert_eq!(session.number("return cw"), 99.0);
    assert_eq!(session.number("return ch"), 99.0);
    assert_eq!(session.number("return cs.x"), 5.0);
    assert_eq!(session.number("return cs.y"), 6.0);
    assert_eq!(session.number("return cs2.x"), 7.0);
    assert_eq!(session.truth("sc"), 1.0);
    assert_eq!(session.truth("sc2"), 0.0);
    assert_eq!(session.truth("skip"), 1.0);
    assert_eq!(session.truth("hasPattern"), 1.0);
    assert_eq!(session.truth("samePattern"), 1.0);
    assert_eq!(session.truth("sameShape"), 1.0);
}

#[test]
fn a_pattern_instance_reads_its_registry_pattern_req_lua_085() {
    let mut session = session_with("");

    session.evaluate(
        r"
        local p = Pattern.GetPatternWithTag('t')
        isPattern = p.Index == 0 and p.TagStr == 't'
        tw, th = p.tSize.x, p.tSize.y
        cw = p.constraintWidth
        p.constraintHeight = 12
        ch = p.constraintHeight
        sc = p.selfConnected
        p.selfConnected = false
        sc2 = p.selfConnected
        ref = p.refPattern
        refCount = ref.count
        samePatternObject = p.pattern == ref and p.refPattern3D == p
        noShape = p.Shape2D
        noGeometry = p.Pos.x
        isStatic = p.IsStatic
        lAxis = p.LAxis
        anim = p.AnimEnable
        ",
    );

    assert_eq!(session.truth("isPattern"), 1.0);
    assert_eq!(session.number("return tw"), 2.0);
    assert_eq!(session.number("return cw"), 99.0);
    assert_eq!(session.number("return ch"), 12.0);
    assert_eq!(session.truth("sc"), 1.0);
    assert_eq!(session.truth("sc2"), 0.0);
    assert_eq!(session.number("return refCount"), 1.0);
    assert_eq!(session.truth("samePatternObject"), 1.0);
    assert_eq!(session.text("return type(noShape)"), "nil");
    assert_eq!(session.number("return noGeometry"), 0.0);
    assert_eq!(session.truth("isStatic"), 1.0);
    assert_eq!(session.truth("lAxis"), 1.0);
    assert_eq!(session.truth("anim"), 0.0);
}

#[test]
fn the_registry_pattern_is_readable_and_writable_req_lua_085() {
    let mut session = session_with("");

    session.evaluate(
        r"
        local pattern = Pattern.GetPatternWithTag('t').pattern
        count, permanent = pattern.count, pattern.permanent
        size, tsize, bl, tr = pattern.Size, pattern.tSize, pattern.BottomLeft, pattern.TR
        first = pattern.pattern3D
        firstIsPattern = first == pattern[0] and pattern[0] == pattern.pattern3DList[0]
        listCount = pattern.pattern3DList.Count
        pattern.Disable = true
        disabled = pattern.Disable
        pattern.CSize = IntVector2(3, 4)
        csize = pattern.constraintSize
        pattern.SelfConnected = false
        pattern.SkipGeo = true
        flags = pattern.SelfConnected or pattern.SkipGeo
        ",
    );

    assert_eq!(session.number("return count"), 1.0);
    assert_eq!(session.truth("permanent"), 1.0);
    assert_eq!(session.number("return size.x"), 16.0);
    assert_eq!(session.number("return size.y"), 8.0);
    assert_eq!(session.number("return tsize.x"), 2.0);
    assert_eq!(session.truth("firstIsPattern"), 1.0);
    assert_eq!(session.number("return listCount"), 1.0);
    assert_eq!(session.truth("disabled"), 1.0);
    assert_eq!(session.number("return csize.y"), 4.0);
    assert_eq!(session.truth("flags"), 1.0);
}

#[test]
fn the_tag_queries_of_the_patterns_give_shared_lists_req_lua_048() {
    let mut session = session_with("");

    session.evaluate(
        r"
        local list = Pattern.GetPatternsWithTag('t')
        n = list.Count
        local item = list[0]
        has = list:Contains(item)
        at = list:IndexOf(item)
        other = list:IndexOf(Pattern.GetPatternWithTag('s'))
        none = list:IndexOf(5)
        empty = Pattern.GetPatternsWithTag('missing').Count
        missing = Pattern.GetPatternWithTag('missing')
        ",
    );

    assert_eq!(session.number("return n"), 1.0);
    assert_eq!(session.truth("has"), 1.0);
    assert_eq!(session.number("return at"), 0.0);
    assert_eq!(session.number("return other"), -1.0);
    assert_eq!(session.number("return none"), -1.0);
    assert_eq!(session.number("return empty"), 0.0);
    assert_eq!(session.text("return type(missing)"), "nil");
}

#[test]
fn the_texture_slot_is_shared_and_assignable_req_lua_097() {
    let mut session = session_with("");

    session.evaluate(
        r"
        local t = Pattern.GetPatternWithTag('t')
        local s = Pattern.GetPatternWithTag('s')
        w, h = t.slot.w, t.slot.h
        sw = s.slot.w
        s.slot = t.slot
        shared = s.slot == t.slot
        s.slot = nil
        unshared = s.slot == s.slot and s.slot ~= t.slot
        ",
    );

    assert_eq!(session.number("return w"), 16.0);
    assert_eq!(session.number("return h"), 8.0);
    assert_eq!(session.number("return sw"), 8.0);
    assert_eq!(session.truth("shared"), 1.0);
    assert_eq!(session.truth("unshared"), 1.0);
    let message = session
        .try_evaluate("Pattern.GetPatternWithTag('t').slot = 5")
        .unwrap_err()
        .to_string();
    assert!(message.contains("invalid arguments to slot"), "{message}");
}

// the shapes

#[test]
fn the_members_of_a_shape_describe_its_extent_req_lua_081() {
    let mut session = session_with("");

    session.evaluate(
        r"
        local shape = Frame:GetShapeWithTag('t').Shape2D
        ts, te, tz = shape.tStart, shape.TEnd, shape.TSize
        ps, pe, pz = shape.pStart, shape.End, shape.Size
        bl, tr = shape.BL, shape.TopRight
        bg, hidden, palette = shape.bg, shape.Hidden, shape.palIndex
        tiles, inside, border, done = shape.TileCount, shape.Inside, shape.AtBorder, shape.Completed
        age, count = shape.Age, shape.InsCount
        first = shape.shape3D
        sameFirst = first == shape[0] and first == shape:Shape3D() and shape:Shape3D(0) == first
        layer, geo, depth = shape.layer, shape.geo, shape.sizeZ
        center = shape.center
        refPattern = shape.refPattern
        sameCount = shape.Count == count
        ",
    );

    assert_eq!(session.number("return ts.x"), 5.0);
    assert_eq!(session.number("return te.x"), 6.0);
    assert_eq!(session.number("return tz.x"), 2.0);
    assert_eq!(session.number("return ps.x"), 32.0);
    assert_eq!(session.number("return pe.x"), 47.0);
    assert_eq!(session.number("return pz.y"), 8.0);
    assert_eq!(session.number("return bl.x"), 32.0);
    assert_eq!(session.truth("bg"), 1.0);
    assert_eq!(session.truth("hidden"), 0.0);
    assert_eq!(session.number("return tiles"), 2.0);
    assert_eq!(session.number("return age"), 1.0);
    assert_eq!(session.number("return count"), 1.0);
    assert_eq!(session.truth("sameFirst"), 1.0);
    assert_eq!(session.number("return layer"), 2.0);
    assert_eq!(session.number("return depth"), 3.0);
    assert_eq!(session.truth("sameCount"), 1.0);
    assert_eq!(session.text("return type(refPattern)"), "userdata");
}

#[test]
fn the_distance_between_two_shapes_is_squared_req_lua_081() {
    let mut session = session_with("");

    session.evaluate(
        r"
        local a = Frame:GetShapeWithTag('t').Shape2D
        local b = Frame:GetShapeWithTag('s').Shape2D
        d = a:Distance(b)
        expected = (a[0].Pos - b[0].Pos).sqrMagnitude
        self = a:Distance(a)
        ",
    );

    assert_close(
        session.number("return expected"),
        session.number("return d"),
        3,
    );
    assert_eq!(session.number("return self"), 0.0);
    let message = session
        .try_evaluate("Frame:GetShapeWithTag('t').Shape2D:Distance(5)")
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("invalid arguments to Distance"),
        "{message}"
    );
}

#[test]
fn a_shape_calculates_the_positions_of_its_instances() {
    let mut session = session_with(
        r"
        function UpdateS()
          if shape.TagStr ~= 't' then return end
          shape.Offset.x = 10
          shape2D:CalculatePos()
          pos = shape.Pos.x
        end
        ",
    );

    assert_eq!(session.number("return pos"), 32.0 + 8.0 + 10.0);
}

// the handles (REQ-LUA-028, REQ-LUA-043, REQ-LUA-045)

#[test]
fn a_handle_kept_from_an_earlier_frame_reads_as_gone_req_lua_028() {
    let mut session = session_with(
        "function UpdateS() if shape.TagStr == 't' then kept, keptShape = shape, shape2D end end",
    );

    assert_eq!(session.number("return kept.sizeZ"), 3.0);
    session.frame(&Batch::new().bg_cell(img::c(), 1, 1), 2);

    assert_eq!(session.number("return kept.sizeZ"), 0.0);
    assert_eq!(session.truth("kept.Enable"), 0.0);
    assert_eq!(session.number("return keptShape.TileCount"), 0.0);
    assert_eq!(session.truth("keptShape.AtBorder"), 1.0);
    session.evaluate("kept.Alpha = 0.5; kept.Offset.x = 4");
    assert_eq!(session.number("return kept.Alpha"), 0.0);
    assert_eq!(session.number("return kept.Offset.x"), 0.0);
}

#[test]
fn the_same_object_is_the_same_value_req_lua_043() {
    let mut session = session_with("");

    assert_eq!(
        session.truth("Pattern.GetPatternWithTag('t') == Pattern.GetPatternWithTag('t')"),
        1.0
    );
    assert_eq!(
        session.truth("Pattern.GetPatternWithTag('t') ~= Pattern.GetPatternWithTag('s')"),
        1.0
    );
    assert_eq!(session.truth("FrameManager.Ins == Frame"), 1.0);
    assert_eq!(
        session.truth("Script:Get('p') == Script:GetScript('p')"),
        1.0
    );
    assert_eq!(
        session.truth("Setting.Layer == Layer and Setting.Clipping == Clipping"),
        1.0
    );
    assert_eq!(
        session.truth("Gamepad[0] == Gamepad1 and Gamepad[1] == Gamepad2"),
        1.0
    );
    assert_eq!(session.truth("Gamepad1 ~= Gamepad2"), 1.0);
    assert_eq!(session.truth("Vector3(1, 2, 3) ~= Vector2(1, 2)"), 1.0);
}

#[test]
fn a_host_value_is_a_table_key_req_lua_045() {
    let mut session = session_with("");

    session.evaluate(
        r"
        local seen = {}
        seen[Pattern.GetPatternWithTag('t')] = 'a'
        seen[Pattern.GetPatternWithTag('s')] = 'b'
        a, b = seen[Pattern.GetPatternWithTag('t')], seen[Pattern.GetPatternWithTag('s')]
        ",
    );

    assert_eq!(session.text("return a .. b"), "ab");
}

// the frame (REQ-LUA-084 to REQ-LUA-087)

#[test]
fn the_frame_queries_find_the_instances_req_lua_084() {
    let mut session = session_with("");

    session.evaluate(
        r"
        count, counter, time = Frame.shapeCount, Frame.frameCounter, Frame.time
        byTag = Frame:GetShapeWithTag('s')
        isSprite = byTag.Sp
        local list = Frame:GetShapesWithTag('*')
        all = list.Count
        local pos = Frame:GetShapeWithPos(34, 212, '*')
        noTag = Frame:GetShapeWithPos(34, 212)
        outside = Frame:GetShapeWithPos(0, 0, '*')
        first = Frame:GetShape(0) == list[0] and list:Contains(Frame:GetShape(1))
        index = list:IndexOf(Frame:GetShape(1))
        hit = pos ~= nil and pos.TagStr
        ",
    );

    assert_eq!(session.number("return count"), 2.0);
    assert_eq!(session.number("return counter"), 1.0);
    assert_close(1.0 / 60.0, session.number("return time"), 5);
    assert_eq!(session.truth("isSprite"), 1.0);
    assert_eq!(session.number("return all"), 2.0);
    assert_eq!(session.text("return type(noTag)"), "nil");
    assert_eq!(session.text("return type(outside)"), "nil");
    assert_eq!(session.truth("first"), 1.0);
    assert_eq!(session.number("return index"), 1.0);
    assert_eq!(session.text("return hit"), "t");
}

#[test]
fn the_palette_reads_and_writes_the_frame_palette_req_lua_087() {
    let mut session = session_with("");
    Batch::new()
        .bg_cell(img::a(), 4, 4)
        .palette(5, 0x16)
        .convert_and_publish(&mut session.engine, 2);

    session.evaluate(
        r"
        Palette.SetColorIndex(6, 0x45, 1)
        set = Palette.GetColorIndex(6, 1)
        masked = Palette.FrameColorIndex(6)
        Palette.SetColorIndex(7, 0x45)
        wrote = Palette.FrameColorIndex(7)
        background = Palette.bgColor
        c = Palette.FrameColor(5)
        ",
    );

    // Right after a publication the frame being converted is the published one, so the three
    // reads see the same buffer.
    assert_eq!(session.number("return set"), f64::from(0x05));
    assert_eq!(session.number("return masked"), f64::from(0x05));
    assert_eq!(session.number("return wrote"), f64::from(0x05));
    assert_eq!(session.number("return background.a"), 1.0);
    assert_close(252.0 / 255.0, session.number("return c.r"), 5);
    for call in [
        "Palette.FrameColor(64)",
        "Palette.GetColorIndex(-1)",
        "Palette.SetColorIndex(70, 1)",
    ] {
        let message = session.try_evaluate(call).unwrap_err().to_string();
        assert!(message.contains("palette entry"), "{call}: {message}");
    }
}

#[test]
fn a_script_can_inject_shapes_into_the_frame_req_lua_086() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script(
            "p",
            r"
            function LateUpdate()
              if done then return end
              done = true
              local p = Pattern.GetPatternWithTag('t')
              added = Frame:AddNewShape(p, Vector3(16, 16, 0), 4)
              created = Frame:CreateNewShape(p.pattern, Vector3(24, 32, 5), 4)
              inserted = Frame:InsertShape(p.pattern, Vector3(40, 40, 0), 4)
              fromInstance = Frame:CreateNewShape(p, Vector3(8, 8, 0), 4)
              addedKind = added.TagStr
              createdCount = created.InsCount
            end
            ",
        )
        .start();
    session.frame(&Batch::new().bg_cell(img::a(), 20, 20), 1);

    assert!(
        session.log.errors().is_empty(),
        "{:?}",
        session.log.errors()
    );
    assert_eq!(session.text("return addedKind"), "t");
    assert_eq!(session.number("return createdCount"), 1.0);
    assert_eq!(session.text("return type(created)"), "userdata");
    assert_eq!(session.text("return type(fromInstance)"), "userdata");
    let message = session
        .try_evaluate("Frame:AddNewShape(nil, Vector3(0, 0, 0), 0)")
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("invalid arguments to AddNewShape"),
        "{message}"
    );
}

#[test]
fn the_pinned_frame_can_be_queried_and_restored_req_e3d_233() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script("p", "-- nothing")
        .start();
    session.frame(&Batch::new().bg_cell(img::a(), 4, 4), 1);

    session.evaluate(
        r"
        Frame:PinNewFrame()
        pinned = Frame:GetShapesFromPinnedFrame(Rect(0, 0, 400, 400))
        n = pinned.Count
        first = pinned[0]
        has = pinned:Contains(first) and pinned:IndexOf(first) == 0
        byVectors = Frame:GetShapesFromPinnedFrame(Vector2(0, 0), Vector2(400, 400)).Count
        tiles = first.TileCount
        ",
    );
    assert_eq!(session.number("return n"), 1.0);
    assert_eq!(session.truth("has"), 1.0);
    assert_eq!(session.number("return byVectors"), 1.0);
    assert_eq!(session.number("return tiles"), 1.0);

    // The pinned shapes stay readable while the pinned frame is held, and no longer after.
    session.frame(&Batch::new(), 2);
    assert_eq!(session.number("return first.TileCount"), 1.0);
    session.evaluate("Frame:ReleasePinnedFrame()");
    assert_eq!(session.number("return first.TileCount"), 0.0);

    session.evaluate("Frame:PinNewFrame(); Frame:AddRectFromPinnedFrame(0, 0, 400, 400); Frame:RestoreFromPinnedFrame(); Frame:RestoreFromPinnedFrame(Rect(0, 0, 10, 10))");
    let message = session
        .try_evaluate("Frame:GetShapesFromPinnedFrame(5)")
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("invalid arguments to GetShapesFromPinnedFrame"),
        "{message}"
    );
}

// the subjective camera (REQ-LUA-088, SPEC-06 REQ-SCN-021)

#[test]
fn the_camera_can_follow_a_shape_req_lua_088() {
    let mut session = session_with(
        r"
        function UpdateS()
          if shape.TagStr == 't' then Camera.FpvShape = shape2D; followed = Camera.FpvShape == shape2D end
        end
        ",
    );

    session.frame(
        &Batch::new().bg_cell(img::a(), 4, 4).bg_cell(img::b(), 5, 4),
        2,
    );

    assert_eq!(session.truth("followed"), 1.0);
    assert_eq!(session.number("return Camera.LookAt.x"), 40.0);
    session.evaluate("Camera.FpvShape = nil");
    assert_eq!(session.text("return type(Camera.FpvShape)"), "nil");
    let message = session
        .try_evaluate("Camera.FpvShape = 5")
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("invalid arguments to FpvShape"),
        "{message}"
    );
}

// the scripts (REQ-LUA-007, REQ-LUA-094)

#[test]
fn the_script_table_finds_the_scripts_and_the_memory() {
    let mut session = ScriptProfileBuilder::new()
        .script_with("one", "x = 1", "a b", true)
        .script_with("two", "x = 2", "*", false)
        .load();

    session.evaluate(
        r"
        count = Script.Count
        byName, byIndex = Script:Get('two'), Script:GetScript(0)
        missing, outside = Script:Get('none'), Script:GetScript(5)
        name, enabled, failed, tags = byName.Name, byName.enable, byName.Error, Script:GetScript('one').TagStr
        byName.Enable = true
        nowEnabled = Script:Get('two').Enable
        Script:WriteMem(0x10, 300)
        mem = Script:ReadMem(0x10)
        ",
    );

    assert_eq!(session.number("return count"), 2.0);
    assert_eq!(session.text("return byIndex.name"), "one");
    assert_eq!(
        session.text("return type(missing) .. type(outside)"),
        "nilnil"
    );
    assert_eq!(session.text("return name"), "two");
    assert_eq!(session.truth("enabled"), 0.0);
    assert_eq!(session.truth("failed"), 0.0);
    assert_eq!(session.text("return tags"), "a b");
    assert_eq!(session.truth("nowEnabled"), 1.0);
    assert_eq!(session.number("return mem"), 44.0);
}

// the pads (REQ-LUA-032, REQ-LUA-091, REQ-LUA-092)

#[test]
fn the_script_pad_feeds_the_input_mask_req_lua_032() {
    let mut session = ScriptProfileBuilder::new()
        .script(
            "pad",
            r"
            function Start() ScriptPad:Enable(true); ScriptPad.PadIndex = 1 end
            function InputProcess(index) seen = index; return result end
            ",
        )
        .start();

    for (result, expected) in [("8", 8), ("8.9", 8), ("nil", 0), ("true", 0), ("'x'", 0)] {
        session.evaluate(&format!("result = {result}"));
        let mask = session.runtime(|runtime, state| runtime.invoke_input_process(state, 5, 0.5));
        assert_eq!(mask, expected, "{result}");
        assert_eq!(
            session.number("return ScriptPad.State"),
            f64::from(expected)
        );
    }

    assert_eq!(session.number("return seen"), 1.0);
    assert_eq!(session.number("return frame"), 5.0);
    session.evaluate("result = 4");
    session.runtime(|runtime, state| runtime.invoke_input_process(state, 6, 0.5));
    assert_eq!(session.truth("ScriptPad:Pressed(4)"), 1.0);
    assert_eq!(session.truth("ScriptPad:Pressed(1)"), 0.0);
}

#[test]
fn the_script_pad_is_silent_when_off_or_in_error_req_lua_032() {
    let mut session = ScriptProfileBuilder::new()
        .script("pad", "function InputProcess() error('boom') end")
        .start();

    let off = session.runtime(|runtime, state| runtime.invoke_input_process(state, 1, 0.0));
    assert_eq!(off, 0);

    session.evaluate("ScriptPad:Enable(true)");
    let failed = session.runtime(|runtime, state| runtime.invoke_input_process(state, 2, 0.0));
    assert_eq!(failed, 0);
    assert!(session.script_failed("pad"));
    assert!(session.log.errors()[0].contains("hook InputProcess raised"));
    let silent = session.runtime(|runtime, state| runtime.invoke_input_process(state, 3, 0.0));
    assert_eq!(silent, 0);
}

#[test]
fn the_pads_and_the_vr_state_read_zero_on_desktop_req_lua_091() {
    let mut session = ScriptProfileBuilder::new()
        .script("a", "-- nothing")
        .start();

    assert_eq!(session.number("return Gamepad.State"), 0.0);
    assert_eq!(session.truth("Gamepad:Pressed(1)"), 0.0);
    assert_eq!(session.number("return Gamepad:GetInput(0)"), 0.0);
    assert_eq!(session.number("return Gamepad[1].State"), 0.0);
    assert_eq!(session.truth("Gamepad1:Pressed(255)"), 0.0);
    assert_eq!(session.number("return ScriptPad.LeftHand.pos.x"), 0.0);
    assert_eq!(session.number("return ScriptPad.RH.rot.y"), 0.0);
    assert_eq!(session.number("return ScriptPad.Head.vel.z"), 0.0);
    assert_eq!(session.number("return ScriptPad:HandPos().x"), 0.0);
    assert_eq!(session.number("return ScriptPad:RotPos().z"), 0.0);
    assert_eq!(session.number("return ZapperOffset.x"), 0.0);
    session.evaluate("ZapperOffset.x = 5; ScriptPad.LH.pos = Vector3.one");
    assert_eq!(session.number("return ZapperOffset.x"), 5.0);
    assert_eq!(session.number("return ScriptPad.LH.pos.x"), 0.0);
    assert!(session.try_evaluate("Gamepad.Pressed(1)").is_err());
    assert!(session.try_evaluate("Gamepad1.Pressed(1)").is_err());
}

#[test]
fn a_hook_can_restore_shapes_from_the_pinned_frame_req_e3d_233() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a()], PatternSpec::tagged("t"))
        .script(
            "p",
            r"
            function Update()
              if frame == 2 then
                Frame:AddRectFromPinnedFrame(0, 0, 400, 400)
                Frame:RestoreFromPinnedFrame()
              end
            end
            ",
        )
        .start();
    session.frame(&Batch::new().bg_cell(img::a(), 4, 4), 1);
    session.evaluate("Frame:PinNewFrame()");
    let before = session.engine.converting_frame().shapes().len();

    session.frame(&Batch::new().bg_cell(img::c(), 1, 1), 2);

    assert_eq!(session.log.errors(), [] as [std::string::String; 0]);
    assert!(session.engine.converting_frame().shapes().len() > before.saturating_sub(1));
    assert!(session.engine.converting_frame().shapes().len() >= 2);
}
