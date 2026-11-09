using Zax.Profiles.Catalogue;
using Zax.Profiles.Hash;
using Zax.Profiles.Wire;
using Zax.Profiles.Import;

namespace Zax.Profiles.Tests;

/// <summary>
/// The library and the imports with both formats (AMD-062 REQ-PROF-062, REQ-PROF-063, and the
/// amended AMD-030 §5-6 / AMD-039 §4).
/// </summary>
public sealed class HashProfileLibraryTests
{
    private static readonly string Destination = Path.Combine("C:", "user", "profiles");

    /// <summary>A file the user points at, outside the profile folder.</summary>
    private static string Source(string name) => Path.Combine("C:", "downloads", name);

    // REQ-PROF-062: the user profile folder is scanned for .3dn and .3dh, and the content
    // decides the format.
    [HashAssetFact]
    public void The_user_folder_scans_both_extensions()
    {
        FakeProfileFileSystem files = new();
        byte[] hashProfile = NeutralAssets.ReadHashProfile("BalloonFight.bytes");
        byte[] plainProfile = NeutralAssets.ReadProfile("Mega Man 2.bytes");

        // One file of each format, under each extension. A « 3dh » profile named .3dn is what
        // the import from an installation writes (AMD-030 §5 as AMD-062 amends it), and a
        // 3dn profile named .3dh is read just as well: the content decides (REQ-PROF-070).
        files.AddFile(Path.Combine(Destination, "a-hash.3dh"), hashProfile);
        files.AddFile(Path.Combine(Destination, "b-plain.3dn"), plainProfile);
        files.AddFile(Path.Combine(Destination, "c-misnamed.3dn"), Zelda());
        files.AddFile(Path.Combine(Destination, "d-misnamed.3dh"), NeutralAssets.ReadProfile("Bomberman.bytes"));

        UserProfileFolder folder = new(files, Destination);
        IReadOnlyList<UserProfile> profiles = folder.Scan();

        Assert.Equal(4, profiles.Count);

        // The identifier of a « 3dh » entry is the one of its envelope (REQ-PROF-073, AMD-037).
        Assert.Equal(
            HashEnvelopeCodec.Decode(hashProfile).Id,
            Find(profiles, "a-hash.3dh").Id);
        Assert.Equal(
            HashEnvelopeCodec.Decode(Zelda()).Id,
            Find(profiles, "c-misnamed.3dn").Id);

        // A 3dn keeps its own, carried or derived (AMD-037 §2).
        ProfileHeader plainHeader = ProfileFormat.LoadHeader(plainProfile)!.Value;
        Assert.Equal(
            plainHeader.HasId ? plainHeader.Id : UserProfileFolder.DerivedIdentifier(plainProfile),
            Find(profiles, "b-plain.3dn").Id);
    }

    private static UserProfile Find(IReadOnlyList<UserProfile> profiles, string fileName)
        => profiles.Single(profile => profile.FilePath.EndsWith(fileName, StringComparison.Ordinal));

    // REQ-PROF-063: an imported file is copied as it is, with no conversion.
    [HashAssetFact]
    public void An_imported_hash_profile_is_copied_as_it_is()
    {
        FakeProfileFileSystem files = new();
        byte[] bytes = NeutralAssets.ReadHashProfile("Super Mario Bros.bytes");
        files.AddFile(Source("Super Mario Bros.3dh"), bytes);

        UserProfileFolder folder = new(files, Destination);
        ProfileImportResult result = folder.Import(Source("Super Mario Bros.3dh"));

        Assert.Equal(ProfileImportOutcome.Added, result.Outcome);
        Assert.Equal("Jhj6E7iOysxT", result.Profile!.Id);

        string written = Path.Combine(Destination, "Super Mario Bros.3dh");
        Assert.True(files.FileExists(written));
        Assert.Equal(bytes, files.ReadAllBytes(written));
        Assert.True(ProfileFormat.IsHashProfile(files.ReadAllBytes(written)));
    }

    // REQ-PROF-063 and AMD-056: the dialogs are unchanged, and the cause shown for a refused
    // « 3dh » is the third text of REQ-UI-084.
    [HashAssetFact]
    public void A_refused_hash_profile_shows_the_obsolete_format_message()
    {
        FakeProfileFileSystem files = new();
        HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(
            NeutralAssets.ReadHashProfile("BalloonFight.bytes"));

        // An envelope version outside "1", "2" and "5" is a refusal of REQ-PROF-072.
        byte[] bytes = SyntheticHashProfile.EncodeEnvelope(
            envelope.Payload,
            version: "4",
            ids: envelope.Ids,
            payloadHashOverride: envelope.PayloadHash);
        files.AddFile(Source("bad.3dh"), bytes);

        UserProfileFolder folder = new(files, Destination);
        ProfileImportPlan plan = folder.Inspect(Source("bad.3dh"));

        Assert.Equal(ProfileImportOutcome.Rejected, plan.Outcome);
        Assert.Equal(ProfileImportMessages.ObsoleteFormat(Source("bad.3dh")), plan.Message);
        Assert.False(plan.Writes);

        // Loading it is what names the cause of REQ-PROF-072 in the log.
        ProfileLoadResult loaded = ProfileFormat.Load(bytes);
        Assert.False(loaded.IsSuccess);
        Assert.Equal(HashProfileMessages.Version4, loaded.Message);
    }

    // REQ-PROF-072: a « 3dh » whose payload was damaged still appears in the list of games with
    // the Id and the Title of its envelope (REQ-PROF-073), and is refused at load time.
    [HashAssetFact]
    public void A_damaged_hash_profile_still_enters_the_library()
    {
        FakeProfileFileSystem files = new();
        HashProfileEnvelope envelope = HashEnvelopeCodec.Decode(
            NeutralAssets.ReadHashProfile("Super Mario Bros.bytes"));
        envelope.Payload![200] ^= 0x01;
        byte[] bytes = SyntheticHashProfile.EncodeEnvelope(
            envelope.Payload,
            ids: envelope.Ids,
            payloadHashOverride: envelope.PayloadHash,
            id: envelope.Id,
            title: envelope.Title);
        files.AddFile(Path.Combine(Destination, "damaged.3dh"), bytes);

        UserProfileFolder folder = new(files, Destination);
        UserProfile profile = Assert.Single(folder.Scan());

        Assert.Equal("Jhj6E7iOysxT", profile.Id);
        Assert.Equal(envelope.Title, profile.Header.Title);

        ProfileLoadResult loaded = ProfileFormat.Load(bytes);
        Assert.False(loaded.IsSuccess);
        Assert.Equal(HashProfileMessages.DamagedPayload, loaded.Message);
    }

    // AMD-030 §5 as AMD-062 amends it: a synthetic bundle whose TextAsset is the « 3dh »
    // BalloonFight.bytes imports that file byte for byte.
    [HashAssetFact]
    public void A_bundle_carrying_a_hash_profile_imports_it_byte_for_byte()
    {
        byte[] bytes = NeutralAssets.ReadHashProfile("BalloonFight.bytes");
        FakeProfileFileSystem files = new();

        ThreeDSenImportSummary summary = new ThreeDSenImporter(files, Destination).Import(
            "/3dSen/bundle",
            ThreeDSenProfileBundle.ReadTextAssets(
                SyntheticUnityBundle.Build(
                    [("BalloonFight", bytes)], SyntheticUnityBundle.ThreeDSen2)));

        Assert.Equal(1, summary.Imported);
        Assert.Equal(0, summary.Failed);

        // The file keeps the .3dn extension even though its content is a « 3dh »: the format is
        // recognised by the content (REQ-PROF-070).
        string written = Path.Combine(Destination, "BalloonFight.3dn");
        Assert.True(files.FileExists(written));
        Assert.Equal(bytes, files.ReadAllBytes(written));
        Assert.True(ProfileFormat.IsHashProfile(files.ReadAllBytes(written)));

        // A second import of the same bundle adds nothing (AMD-030 §2).
        ThreeDSenImportSummary again = new ThreeDSenImporter(files, Destination).Import(
            "/3dSen/bundle",
            ThreeDSenProfileBundle.ReadTextAssets(
                SyntheticUnityBundle.Build(
                    [("BalloonFight", bytes)], SyntheticUnityBundle.ThreeDSen2)));
        Assert.Equal(0, again.Imported);
        Assert.Equal(1, again.AlreadyPresent);
    }

    // AMD-039 §4, last bullet: a 2.0 profile and a 2.1 one imported in turn. For a game whose
    // 3dn carries an Id, both files carry the same one; otherwise the 3dn takes the derived
    // identifier of AMD-037 and the « 3dh » the one of its envelope — two library entries.
    [HashAssetFact]
    public void A_2_0_and_a_2_1_profile_of_the_same_game_keep_their_identifiers()
    {
        int sameId = 0;
        int distinct = 0;
        foreach (string file in NeutralAssets.HashProfileFiles())
        {
            string name = Path.GetFileName(file);
            byte[] hashBytes = File.ReadAllBytes(file);
            byte[] plainBytes = File.ReadAllBytes(
                Path.Combine(NeutralAssets.ProfilesDirectory!, NeutralAssets.CounterpartName(name)));

            FakeProfileFileSystem files = new();
            files.AddFile(Path.Combine(Destination, "a.3dn"), plainBytes);
            files.AddFile(Path.Combine(Destination, "b.3dh"), hashBytes);

            IReadOnlyList<UserProfile> profiles = new UserProfileFolder(files, Destination).Scan();
            string envelopeId = HashEnvelopeCodec.Decode(hashBytes).Id!;
            ProfileHeader plainHeader = ProfileFormat.LoadHeader(plainBytes)!.Value;

            if (plainHeader.HasId)
            {
                // Both files carry the same identifier, so the second is the duplicate of the
                // first and one library entry remains (REQ-PROF-062, unchanged).
                Assert.Equal(plainHeader.Id, envelopeId);
                UserProfile only = Assert.Single(profiles);
                Assert.Equal(envelopeId, only.Id);
                Assert.EndsWith("a.3dn", only.FilePath, StringComparison.Ordinal);
                sameId++;
            }
            else
            {
                // The 3dn takes the derived identifier of AMD-037, the « 3dh » the one of its
                // envelope: two distinct entries, which AMD-039 §4 accepts.
                Assert.Equal(2, profiles.Count);
                Assert.Equal(UserProfileFolder.DerivedIdentifier(plainBytes), Find(profiles, "a.3dn").Id);
                Assert.Equal(envelopeId, Find(profiles, "b.3dh").Id);
                distinct++;
            }
        }

        // REQ-PROF-073, notes: 56 of the 101 counterparts carry an identifier.
        Assert.Equal(56, sameId);
        Assert.Equal(45, distinct);
    }

    private static byte[] Zelda() => NeutralAssets.ReadHashProfile("Legend Of Zelda.bytes");
}
