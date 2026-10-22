using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Nes;

namespace Zax.App.Tests;

/// <summary>
/// The run-time save states of SPEC-07 §3.5 and the files of SPEC-09 §3.5: content, slots,
/// quick slot, round-trip determinism, size bound and the refusal of a corrupt file.
/// </summary>
public sealed class SaveStateTests
{
    /// <summary>
    /// REQ-PERS-056 and REQ-LOOP-040: the file opens with <c>ZAXNESAV</c> (AMD-029 §2) and a
    /// format version, and carries the four sections core / Lua / camera / conversion.
    /// </summary>
    [Fact]
    public void REQ_PERS_056_the_file_has_the_signature_and_the_four_sections()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);

        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));

        string path = harness.Paths.SlotStateFile("3");
        Assert.True(File.Exists(path));

        byte[] file = File.ReadAllBytes(path);
        Assert.True(file.AsSpan(0, 8).SequenceEqual("ZAXNESAV"u8));
        Assert.True(SaveStateFile.TryRead(file, out SaveStateContent? content));

        Assert.NotNull(content);
        Assert.Equal(NesCore.CoreName, content!.Header.CoreId);
        Assert.Equal("g1", content.Header.GameId);
        Assert.Equal(harness.Now.ToUnixTimeSeconds(), content.Header.SavedAt);
        Assert.Equal("3", content.Header.SlotId);
        Assert.NotEmpty(content.Core);
        Assert.Equal(Zax.Scripting.PersistentData.BlockSize, content.Lua.Length);
        Assert.Equal(SaveStateFile.CameraSectionBytes, content.Camera.Length);
        Assert.NotEmpty(content.Conversion);
        Assert.Equal(40, content.Header.RomSha1.Length);
    }

    /// <summary>
    /// REQ-PERS-056 acceptance: two saves with no emulated frame in between produce byte-for-byte
    /// identical sections.
    /// </summary>
    [Fact]
    public void REQ_PERS_056_two_saves_without_a_frame_are_identical()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);
        harness.Run(ZaxCommands.Pause);

        SaveStateContent first = harness.Host.CaptureSaveState("1")!;
        SaveStateContent second = harness.Host.CaptureSaveState("1")!;

        Assert.Equal(first.Core, second.Core);
        Assert.Equal(first.Lua, second.Lua);
        Assert.Equal(first.Camera, second.Camera);
        Assert.Equal(first.Conversion, second.Conversion);
    }

    /// <summary>
    /// REQ-ARCH-012 and REQ-ARCH-013: saving at frame N, running M more frames, reloading and
    /// replaying the same inputs reproduces the same core state byte for byte.
    /// </summary>
    [Fact]
    public void REQ_ARCH_012_a_savestate_replays_identically()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.RunFrames(90);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));

        harness.RunExactFrames(120, InputAction.None, InputAction.A, InputAction.Right);
        byte[] firstRun = harness.Host.Session!.Core.SaveState(includeRomImage: false);

        harness.Run(ZaxCommands.LoadSlot, CommandArgs.Of(1));
        Assert.Equal(EmulationState.Running, harness.Host.State);

        harness.RunExactFrames(120, InputAction.None, InputAction.A, InputAction.Right);
        byte[] secondRun = harness.Host.Session!.Core.SaveState(includeRomImage: false);

        Assert.Equal(firstRun, secondRun);
    }

    /// <summary>
    /// REQ-ARCH-013 and REQ-PERS-056: the announced bound holds for the file actually written.
    /// </summary>
    [Fact]
    public void REQ_ARCH_013_the_file_stays_under_the_announced_bound()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(60);

        int bound = harness.Host.MaxSaveStateSize;
        Assert.True(bound > 0);

        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(2));

        long written = new FileInfo(harness.Paths.SlotStateFile("2")).Length;
        Assert.True(written <= bound, $"{written} bytes written, bound announced {bound}");
    }

    /// <summary>
    /// REQ-LOOP-046 and REQ-PERS-056: a truncated or foreign file is refused with
    /// <c>Corrupted save state.</c>, the current game is cleared and the state is Stopped.
    /// </summary>
    [Fact]
    public void REQ_LOOP_046_a_corrupt_savestate_is_refused()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(4));

        // Truncate the file in place; the index still says the slot is occupied.
        string path = harness.Paths.SlotStateFile("4");
        byte[] file = File.ReadAllBytes(path);
        File.WriteAllBytes(path, file.AsSpan(0, file.Length / 3).ToArray());

        harness.Run(ZaxCommands.LoadSlot, CommandArgs.Of(4));

        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.Null(harness.Host.CurrentGameId);
        Assert.False(harness.Host.IsLoading);
        Assert.Equal(SaveStateFile.CorruptedMessage, harness.Host.Notifications.Current!.Value.Text);
    }

    /// <summary>
    /// REQ-PERS-056: a file whose signature or version is unknown is refused by the reader, so
    /// a foreign file never reaches the core.
    /// </summary>
    [Theory]
    [InlineData("WRONGSIG")]
    [InlineData("ZAXNESAV")]
    [InlineData("SEN3DSAV")]
    public void REQ_PERS_056_the_signature_and_version_are_checked(string signature)
    {
        byte[] file = new byte[64];
        System.Text.Encoding.ASCII.GetBytes(signature).CopyTo(file, 0);

        // Format version 99 for the right signature, so both rows are refused.
        file[8] = 99;

        Assert.False(SaveStateFile.TryRead(file, out SaveStateContent? content));
        Assert.Null(content);
    }

    /// <summary>
    /// AMD-029 §2 and REQ-PERS-056: a state written under the « Sen » working name — signature
    /// <c>SEN3DSAV</c>, core name <c>Sen</c> — still loads whole. The current signature is the
    /// only one written, so this file is built by patching the one the host produces.
    /// </summary>
    [Fact]
    public void AMD_029_a_savestate_of_the_old_name_is_still_read()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(45);

        SaveStateContent written = harness.Host.CaptureSaveState("1")!;
        byte[] file = SaveStateFile.Write(written);

        // Both signatures are eight bytes, so nothing after them moves.
        Assert.Equal(SaveStateFile.Signature.Length, SaveStateFile.LegacySignature.Length);
        SaveStateFile.LegacySignature.CopyTo(file.AsSpan(0, 8));
        Assert.True(SaveStateFile.HasKnownSignature(file.AsSpan(0, 8)));

        Assert.True(SaveStateFile.TryRead(file, out SaveStateContent? content));
        Assert.NotNull(content);
        Assert.Equal(written.Core, content!.Core);
        Assert.Equal(written.Lua, content.Lua);
        Assert.Equal(written.Camera, content.Camera);
        Assert.Equal(written.Conversion, content.Conversion);

        // And the core block itself: the legacy core name is accepted, a third one never is.
        NesCore core = new();
        Assert.True(
            core.TryLoadState(ReplaceCoreName(content.Core, NesCore.LegacyCoreName), 0, out _, out _, out string? error),
            error);

        NesCore foreign = new();
        Assert.False(foreign.TryLoadState(ReplaceCoreName(content.Core, "Nestopia"), 0, out _, out _, out error));
        Assert.Equal(NesCore.ForeignCoreMessage, error);
    }

    /// <summary>
    /// Rewrites the core name a state block carries. The block opens with the magic, the format
    /// version and then the name as a length-prefixed UTF-8 string, which is what
    /// <c>StateWriter.WriteString</c> produces.
    /// </summary>
    private static byte[] ReplaceCoreName(byte[] block, string name)
    {
        int storedLength = BitConverter.ToInt32(block, 8);
        byte[] replacement = System.Text.Encoding.UTF8.GetBytes(name);

        byte[] result = new byte[block.Length - storedLength + replacement.Length];
        block.AsSpan(0, 8).CopyTo(result);
        BitConverter.GetBytes(replacement.Length).CopyTo(result, 8);
        replacement.CopyTo(result, 12);
        block.AsSpan(12 + storedLength).CopyTo(result.AsSpan(12 + replacement.Length));
        return result;
    }

    /// <summary>
    /// REQ-PERS-056: a section length that does not match the file is refused.
    /// </summary>
    [Fact]
    public void REQ_PERS_056_an_inconsistent_section_length_is_refused()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(10);

        SaveStateContent content = harness.Host.CaptureSaveState("1")!;
        byte[] file = SaveStateFile.Write(content);

        // The core section's length prefix sits right after the signature, the version and the
        // metadata section; raising it past the end of the file must be refused.
        int metadataLength = BitConverter.ToInt32(file, 12);
        int coreLengthOffset = 8 + 4 + 4 + metadataLength;
        BitConverter.GetBytes(int.MaxValue).CopyTo(file, coreLengthOffset);

        Assert.False(SaveStateFile.TryRead(file, out _));
    }

    /// <summary>
    /// REQ-LOOP-043 and REQ-PERS-052: the quick save writes the quick slot with no thumbnail and
    /// shows <c>Quick Save</c> for 4 s; the quick load restores it and shows <c>Quick Load</c>.
    /// </summary>
    [Fact]
    public void REQ_LOOP_043_the_quick_slot_has_no_thumbnail()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.Thumbnail = new byte[EmulationHost.ThumbnailWidth * EmulationHost.ThumbnailHeight * 3];

        harness.RunFrames(60);
        harness.Run(ZaxCommands.QuickSave);

        SaveSlotRecord quick = harness.Host.Slots.Find(SaveSlotIndex.QuickSlotId)!;
        Assert.True(quick.IsOccupied);
        Assert.Null(quick.Thumbnail);
        Assert.False(File.Exists(harness.Paths.SlotThumbnailFile(SaveSlotIndex.QuickSlotId)));
        Assert.Equal("Quick Save", harness.Host.Notifications.Current!.Value.Text);
        Assert.Equal(4.0, harness.Host.Notifications.Current!.Value.Seconds);

        harness.RunFrames(60);
        long after = harness.Host.Session!.Core.FrameCount;

        harness.Run(ZaxCommands.QuickLoad);

        Assert.Equal(60, harness.Host.Session!.Core.FrameCount);
        Assert.NotEqual(after, harness.Host.Session.Core.FrameCount);
        Assert.Equal("Quick Load", harness.Host.Notifications.Current!.Value.Text);
    }

    /// <summary>
    /// REQ-LOOP-045 and vector T8: a quick load with an empty quick slot does nothing — no
    /// state change and no notification.
    /// </summary>
    [Fact]
    public void REQ_LOOP_045_an_empty_quick_slot_does_nothing()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);

        long frames = harness.Host.Session!.Core.FrameCount;
        harness.Run(ZaxCommands.QuickLoad);

        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal(frames, harness.Host.Session!.Core.FrameCount);
        Assert.Null(harness.Host.Notifications.Current);
    }

    /// <summary>
    /// REQ-LOOP-041: a quick save in the middle of a game does not change the emulation state,
    /// and with no current game nothing happens at all.
    /// </summary>
    [Fact]
    public void REQ_LOOP_041_a_save_does_not_change_the_state()
    {
        using LoopHarness harness = LoopHarness.Create();

        // Stopped, no game: nothing happens.
        harness.Run(ZaxCommands.QuickSave);
        Assert.False(harness.Host.Slots.Find(SaveSlotIndex.QuickSlotId)!.IsOccupied);

        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);
        harness.Run(ZaxCommands.QuickSave);
        Assert.Equal(EmulationState.Running, harness.Host.State);

        harness.Run(ZaxCommands.Pause);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));
        Assert.Equal(EmulationState.Paused, harness.Host.State);
    }

    /// <summary>
    /// REQ-LOOP-042 and REQ-PERS-051: a numbered slot records the game, the pause capture and
    /// the timestamp, persists the metadata immediately, and an occupied slot is overwritten
    /// without asking.
    /// </summary>
    [Fact]
    public void REQ_LOOP_042_a_numbered_slot_carries_the_game_the_thumbnail_and_the_date()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        byte[] capture = new byte[EmulationHost.ThumbnailWidth * EmulationHost.ThumbnailHeight * 3];
        capture[3] = 0x40;
        harness.Thumbnail = capture;

        harness.RunFrames(60);
        harness.Run(ZaxCommands.Pause);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(5));

        SaveSlotRecord slot = harness.Host.Slots.Find("5")!;
        Assert.Equal("g1", slot.GameId);
        Assert.Equal(harness.Now.ToUnixTimeSeconds(), slot.SavedAt);
        Assert.Equal("5.png", slot.Thumbnail);
        Assert.True(File.Exists(harness.Paths.SlotThumbnailFile("5")));

        // The index was written straight away, before the process could die.
        Assert.True(File.Exists(harness.Paths.SaveIndexFile));
        SaveSlotIndex reread = SaveSlotIndex.Load(harness.Paths, harness.Now);
        Assert.Equal("g1", reread.Find("5")!.GameId);

        // Overwriting is silent.
        harness.Now = harness.Now.AddMinutes(5);
        harness.Run(ZaxCommands.Resume);
        harness.RunFrames(30);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(5));
        Assert.Equal(harness.Now.ToUnixTimeSeconds(), harness.Host.Slots.Find("5")!.SavedAt);
    }

    /// <summary>
    /// REQ-LOOP-044 step 2 and vector T9: loading a slot whose game is gone reports
    /// <c>The game profile doesn't exist anymore.</c> and goes to Stopped.
    /// </summary>
    [Fact]
    public void REQ_LOOP_044_a_slot_of_a_deleted_game_reports_the_exact_message()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(2));

        harness.Catalog.Remove("g1");
        harness.Run(ZaxCommands.LoadSlot, CommandArgs.Of(2));

        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.Equal(SaveStateFile.GameMissingMessage, harness.Host.Notifications.Current!.Value.Text);
    }

    /// <summary>
    /// REQ-LOOP-044 acceptance and REQ-PERS-053: a slot is loadable from Stopped without the
    /// ROM ever being asked for, because the core block is self-contained.
    /// </summary>
    [Fact]
    public void REQ_PERS_053_a_slot_loads_without_the_rom_file()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(120);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(7));

        harness.Run(ZaxCommands.Stop);
        Assert.Equal(EmulationState.Stopped, harness.Host.State);

        // The ROM file disappears and the association is cleared.
        File.Delete(Path.Combine(harness.Root, "g1.nes"));
        harness.Catalog.Add("g1", romPath: null);

        harness.Run(ZaxCommands.LoadSlot, CommandArgs.Of(7));

        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal("g1", harness.Host.CurrentGameId);
        Assert.Equal(120, harness.Host.Session!.Core.FrameCount);
    }

    /// <summary>
    /// REQ-LOOP-040 acceptance: the camera comes back to where it was when the state was saved.
    /// </summary>
    [Fact]
    public void REQ_LOOP_040_the_camera_comes_back_with_the_state()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);

        harness.Scene.Camera.Current.Distance = 200f;
        harness.Scene.Camera.SnapToSetpoints();
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(6));

        harness.Scene.Camera.Current.Distance = 40f;
        harness.Scene.Camera.SnapToSetpoints();

        harness.Run(ZaxCommands.LoadSlot, CommandArgs.Of(6));

        Assert.Equal(200f, harness.Scene.Camera.Current.Distance, 1e-3f);
    }

    /// <summary>
    /// REQ-PERS-054: deleting a slot removes its file and empties it in the index.
    /// </summary>
    [Fact]
    public void REQ_PERS_054_deleting_a_slot_removes_its_file()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(30);
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(8));
        Assert.True(File.Exists(harness.Paths.SlotStateFile("8")));

        harness.Run(ZaxCommands.DeleteSlot, CommandArgs.Of(8));

        Assert.False(harness.Host.Slots.Find("8")!.IsOccupied);
        Assert.False(File.Exists(harness.Paths.SlotStateFile("8")));
    }

    /// <summary>
    /// REQ-PERS-050 and AMD-010 REQ-UI-208: the model keeps SPEC-09's 36 numbered slots plus
    /// the quick slot, while the menus expose the first ten.
    /// </summary>
    [Fact]
    public void REQ_PERS_050_the_model_holds_thirty_seven_slots()
    {
        SaveSlotIndex index = new();

        Assert.Equal(SaveSlotIndex.NumberedSlotCount + 1, index.Slots.Count);
        Assert.NotNull(index.Find("1"));
        Assert.NotNull(index.Find("36"));
        Assert.NotNull(index.Find(SaveSlotIndex.QuickSlotId));
        Assert.Null(index.Find("37"));
        Assert.Equal(10, SaveSlotIndex.ExposedSlotCount);
        Assert.False(SaveSlotIndex.IsValidSlotId("0"));
        Assert.False(SaveSlotIndex.IsValidSlotId("37"));
    }
}
