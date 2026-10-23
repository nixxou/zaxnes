using Zax.Profiles.Catalogue;
using Zax.Profiles.Import;

namespace Zax.Profiles.Tests;

/// <summary>
/// What the import of AMD-030 §5 does with the payloads: the SPEC-03 check, the unmodified
/// write, the duplicates and the name clashes of §2, and the refusals of §3 and §5.
/// </summary>
public sealed class ThreeDSenImportTests
{
    private const string Destination = @"C:\zax\profiles";

    [Fact] // AMD-030 §5: « écrit sans aucune modification […] sous le nom <m_Name>.3dn »
    public void A_valid_profile_is_written_unmodified()
    {
        byte[] profile = Profile("ABCDEFGHIJKL", "Pinball");
        FakeProfileFileSystem files = new();
        ThreeDSenImporter importer = new(files, Destination);

        ThreeDSenImportSummary summary = importer.Import(
            "bundle", [new UnityTextAsset("Pinball", profile)]);

        Assert.Equal(1, summary.Imported);
        Assert.Equal(0, summary.AlreadyPresent);
        Assert.Equal(0, summary.Failed);
        Assert.True(summary.IsComplete);

        string path = Path.Combine(Destination, "Pinball.3dn");
        Assert.True(files.FileExists(path));
        Assert.Equal(profile, files.ReadAllBytes(path));
        Assert.Equal("Pinball.3dn", summary.Results[0].FileName);
    }

    [Fact] // AMD-030 §5: a profile that does not decode is a failure and the import carries on.
    public void A_payload_that_is_not_a_profile_fails_without_stopping_the_import()
    {
        FakeProfileFileSystem files = new();
        RecordingProfileLog log = new();
        ThreeDSenImporter importer = new(files, Destination, log);

        ThreeDSenImportSummary summary = importer.Import(
            "bundle",
            [
                new UnityTextAsset("broken", new byte[] { 1, 2, 3, 4, 5 }),
                new UnityTextAsset("good", Profile("ABCDEFGHIJKL", "Good")),
            ]);

        Assert.Equal(1, summary.Imported);
        Assert.Equal(1, summary.Failed);
        Assert.False(summary.IsComplete);
        Assert.False(files.FileExists(Path.Combine(Destination, "broken.3dn")));
        Assert.True(files.FileExists(Path.Combine(Destination, "good.3dn")));
        Assert.Equal(ThreeDSenProfileOutcome.Failed, summary.Results[0].Outcome);
        Assert.Contains("not a usable profile", summary.Results[0].Message, StringComparison.Ordinal);
        Assert.True(log.AnyContains("broken"));
    }

    [Fact] // AMD-030 §2: identical content is skipped.
    public void An_identical_profile_already_there_is_skipped()
    {
        byte[] profile = Profile("ABCDEFGHIJKL", "Pinball");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(Destination, "Pinball.3dn"), profile);
        ThreeDSenImporter importer = new(files, Destination);

        ThreeDSenImportSummary summary = importer.Import(
            "bundle", [new UnityTextAsset("Pinball", profile)]);

        Assert.Equal(0, summary.Imported);
        Assert.Equal(1, summary.AlreadyPresent);
        Assert.Single(files.Paths);
        Assert.Equal("Pinball.3dn", summary.Results[0].FileName);
    }

    [Fact] // AMD-030 §2: « le nouveau profil reçoit un suffixe " (2)", " (3)"… »
    public void A_name_clash_with_other_bytes_is_suffixed()
    {
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(Destination, "Pinball.3dn"), Profile("OTHER0000001", "Older"));
        ThreeDSenImporter importer = new(files, Destination);

        ThreeDSenImportSummary first = importer.Import(
            "bundle", [new UnityTextAsset("Pinball", Profile("ABCDEFGHIJKL", "Pinball"))]);
        ThreeDSenImportSummary second = importer.Import(
            "bundle", [new UnityTextAsset("Pinball", Profile("MNOPQRSTUVWX", "Pinball 2"))]);

        Assert.Equal("Pinball (2).3dn", first.Results[0].FileName);
        Assert.Equal("Pinball (3).3dn", second.Results[0].FileName);
        Assert.True(files.FileExists(Path.Combine(Destination, "Pinball (2).3dn")));
        Assert.True(files.FileExists(Path.Combine(Destination, "Pinball (3).3dn")));

        // The file that was already there is left alone.
        Assert.Equal(
            Profile("OTHER0000001", "Older"),
            files.ReadAllBytes(Path.Combine(Destination, "Pinball.3dn")));
    }

    [Fact] // AMD-030 §2: a third import of the same bytes still finds them, under the suffix.
    public void A_suffixed_profile_is_recognised_on_the_next_import()
    {
        byte[] mine = Profile("ABCDEFGHIJKL", "Pinball");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(Destination, "Pinball.3dn"), Profile("OTHER0000001", "Older"));
        ThreeDSenImporter importer = new(files, Destination);

        importer.Import("bundle", [new UnityTextAsset("Pinball", mine)]);
        ThreeDSenImportSummary again = importer.Import("bundle", [new UnityTextAsset("Pinball", mine)]);

        Assert.Equal(0, again.Imported);
        Assert.Equal(1, again.AlreadyPresent);
        Assert.Equal("Pinball (2).3dn", again.Results[0].FileName);
    }

    [Theory] // AMD-030 §5: « les caractères interdits par le système de fichiers […] par _ »
    [InlineData("Blaster Master (U)", "Blaster Master (U).3dn")]
    [InlineData("Mike Tyson's Punch-Out!!", "Mike Tyson's Punch-Out!!.3dn")]
    [InlineData("A/B", "A_B.3dn")]
    [InlineData("C:D*E?F\"G<H>I|J", "C_D_E_F_G_H_I_J.3dn")]
    [InlineData("trailing. ", "trailing.3dn")]
    [InlineData("", "profil.3dn")]
    [InlineData("...", "profil.3dn")]
    public void The_file_name_substitutes_what_the_file_system_forbids(string name, string expected)
        => Assert.Equal(expected, ThreeDSenImporter.FileNameFor(name));

    [Fact] // AMD-030 §5: « n'écrit aucun fichier partiel »
    public void A_write_that_fails_leaves_no_partial_file()
    {
        FakeProfileFileSystem inner = new();
        FailingFileSystem files = new(inner, failOn: ".part");
        ThreeDSenImporter importer = new(files, Destination);

        ThreeDSenImportSummary summary = importer.Import(
            "bundle", [new UnityTextAsset("Pinball", Profile("ABCDEFGHIJKL", "Pinball"))]);

        Assert.Equal(0, summary.Imported);
        Assert.Equal(1, summary.Failed);
        Assert.Contains("could not be written", summary.Results[0].Message, StringComparison.Ordinal);
        Assert.Empty(inner.Paths);
    }

    [Fact] // AMD-030 §5: the temporary file goes away when the move fails.
    public void A_move_that_fails_removes_the_temporary_file()
    {
        FakeProfileFileSystem inner = new();
        FailingFileSystem files = new(inner, failOn: ".part", onMove: true);
        ThreeDSenImporter importer = new(files, Destination);

        ThreeDSenImportSummary summary = importer.Import(
            "bundle", [new UnityTextAsset("Pinball", Profile("ABCDEFGHIJKL", "Pinball"))]);

        Assert.Equal(1, summary.Failed);
        Assert.Empty(inner.Paths);
    }

    [Fact] // AMD-030 §2: the end-of-import message.
    public void The_summary_reports_the_three_counts()
    {
        byte[] present = Profile("ABCDEFGHIJKL", "Pinball");
        FakeProfileFileSystem files = new();
        files.AddFile(Path.Combine(Destination, "Pinball.3dn"), present);
        ThreeDSenImporter importer = new(files, Destination);

        ThreeDSenImportSummary summary = importer.Import(
            "bundle",
            [
                new UnityTextAsset("Pinball", present),
                new UnityTextAsset("Tetris", Profile("MNOPQRSTUVWX", "Tetris")),
                new UnityTextAsset("Dr Mario", Profile("ZYXWVUTSRQPO", "Dr Mario")),
                new UnityTextAsset("junk", new byte[] { 0xFF, 0xFF }),
            ]);

        Assert.Equal(2, summary.Imported);
        Assert.Equal(1, summary.AlreadyPresent);
        Assert.Equal(1, summary.Failed);
        Assert.Equal(
            // AMD-036 §1: the summary of AMD-030 §5 is English here, so the command line and the
            // log always print English; the interface builds its own from the counts.
            "3dSen: 2 profile(s) imported, 1 already present, 1 failed.", summary.Message);
    }

    [Fact] // AMD-030 §4: a bundle that cannot be read writes nothing at all.
    public void An_unreadable_bundle_writes_nothing()
    {
        string folder = TempFolder();
        try
        {
            string bundle = Path.Combine(folder, "defaultprofiles_assets_all_dead.bundle");
            File.WriteAllBytes(bundle, [1, 2, 3, 4, 5, 6, 7, 8]);

            FakeProfileFileSystem files = new();
            ThreeDSenImporter importer = new(files, Destination);

            Assert.Throws<UnityAssetFormatException>(() => importer.ImportFrom(bundle));
            Assert.Empty(files.Paths);
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    [Fact] // AMD-030 §3: « Si rien n'est trouvé, l'interface le dit clairement et rappelle le chemin attendu. »
    public void An_installation_without_a_bundle_names_the_expected_path()
    {
        string folder = TempFolder();
        try
        {
            Directory.CreateDirectory(Path.Combine(folder, "3dSen_Data"));

            UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
                () => ThreeDSenInstallation.LocateBundle(folder));

            Assert.Contains("no 3dSen profile bundle was found", error.Message, StringComparison.Ordinal);
            Assert.Contains(ThreeDSenInstallation.ExpectedLayout, error.Message, StringComparison.Ordinal);
            Assert.Contains("StreamingAssets/aa", error.Message, StringComparison.Ordinal);
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    [Fact] // AMD-030 §3
    public void A_path_that_does_not_exist_is_refused()
    {
        UnityAssetFormatException error = Assert.Throws<UnityAssetFormatException>(
            () => ThreeDSenInstallation.LocateBundle(
                Path.Combine(Path.GetTempPath(), "zax-no-such-" + Guid.NewGuid().ToString("N"))));

        Assert.Contains("neither a file nor a folder", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §3.1: the platform folders are tried in the order the amendment gives.
    public void The_first_platform_folder_of_the_list_wins()
    {
        string folder = TempFolder();
        try
        {
            string first = Layout(folder, "3dSen_Data", "StandaloneWindows", "aaaa");
            string later = Layout(folder, "3dSen_Data", "StandaloneLinux64", "0000");

            Assert.Equal(first, ThreeDSenInstallation.FindBundle(folder));
            Assert.True(File.Exists(later));
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    [Fact] // AMD-030 §3.1: the data folder's name is not assumed, only its suffix.
    public void Any_data_folder_name_is_accepted()
    {
        string folder = TempFolder();
        try
        {
            string bundle = Layout(folder, "SomeOtherName_Data", "StandaloneWindows64", "beef");

            Assert.Equal(bundle, ThreeDSenInstallation.FindBundle(folder));
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    [Fact] // AMD-030 §3.2: the macOS layout.
    public void The_bundle_is_found_inside_an_app_folder()
    {
        string folder = TempFolder();
        try
        {
            string bundle = Layout(
                folder,
                Path.Combine("3dSen.app", "Contents", "Resources", "Data"),
                "StandaloneOSX",
                "cafe");

            Assert.Equal(bundle, ThreeDSenInstallation.FindBundle(folder));
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    [Fact] // AMD-030 §2: « ou directement le fichier bundle »
    public void A_bundle_file_can_be_named_directly()
    {
        string folder = TempFolder();
        try
        {
            string bundle = Layout(folder, "3dSen_Data", "StandaloneWindows", "1234");

            Assert.Equal(bundle, ThreeDSenInstallation.FindBundle(bundle));
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    [Fact] // AMD-030 §5: the installation is only read — nothing is copied or changed.
    public void The_installation_is_left_untouched_by_an_import()
    {
        string folder = TempFolder();
        try
        {
            string bundle = Layout(folder, "3dSen_Data", "StandaloneWindows", "1234");
            byte[] contents = SyntheticUnityBundle.Build(
                [("Pinball", Profile("ABCDEFGHIJKL", "Pinball"))], new SyntheticBundleOptions());
            File.WriteAllBytes(bundle, contents);
            DateTime stamp = File.GetLastWriteTimeUtc(bundle);

            FakeProfileFileSystem files = new();
            ThreeDSenImportSummary summary = new ThreeDSenImporter(files, Destination).ImportFrom(folder);

            Assert.Equal(1, summary.Imported);
            Assert.Equal(bundle, summary.BundlePath);
            Assert.Equal(contents, File.ReadAllBytes(bundle));
            Assert.Equal(stamp, File.GetLastWriteTimeUtc(bundle));
            Assert.Single(Directory.GetFiles(Path.GetDirectoryName(bundle)!));
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    /// <summary>Builds the bytes of a profile SPEC-03 accepts.</summary>
    internal static byte[] Profile(string id, string title)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = title;
        byte[] bytes = ProfileFormat.Save(profile);

        // The decoder of SPEC-03 is what the import checks with, so the fixture must pass it.
        Assert.True(ProfileFormat.Load(bytes).IsSuccess);
        return bytes;
    }

    private static string TempFolder()
    {
        string folder = Path.Combine(Path.GetTempPath(), "zax-p8-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(folder);
        return folder;
    }

    /// <summary>Creates the layout of AMD-030 §3 and an empty bundle file inside it.</summary>
    private static string Layout(string root, string dataFolder, string platform, string fingerprint)
    {
        string directory = Path.Combine(
            root, dataFolder, "StreamingAssets", "aa", platform);
        Directory.CreateDirectory(directory);

        string bundle = Path.Combine(
            directory,
            ThreeDSenInstallation.BundlePrefix + fingerprint + ThreeDSenInstallation.BundleExtension);
        File.WriteAllBytes(bundle, []);
        return bundle;
    }

    /// <summary>
    /// A file system that refuses to touch the paths holding a given fragment, to check that a
    /// failed write leaves nothing behind.
    /// </summary>
    private sealed class FailingFileSystem(
        FakeProfileFileSystem inner,
        string failOn,
        bool onMove = false) : IProfileFileSystem
    {
        public bool FileExists(string path) => inner.FileExists(path);

        public bool DirectoryExists(string path) => inner.DirectoryExists(path);

        public void CreateDirectory(string path) => inner.CreateDirectory(path);

        public IReadOnlyList<string> ListFiles(string directory, string extension)
            => inner.ListFiles(directory, extension);

        public byte[] ReadAllBytes(string path) => inner.ReadAllBytes(path);

        public string ReadAllText(string path) => inner.ReadAllText(path);

        public void WriteAllBytes(string path, byte[] contents)
        {
            if (!onMove && path.Contains(failOn, StringComparison.Ordinal))
            {
                throw new IOException($"{path}: the disk is full");
            }

            inner.WriteAllBytes(path, contents);
        }

        public void WriteAllText(string path, string contents) => inner.WriteAllText(path, contents);

        public void DeleteFile(string path) => inner.DeleteFile(path);

        public void CopyFile(string sourcePath, string destinationPath, bool overwrite)
            => inner.CopyFile(sourcePath, destinationPath, overwrite);

        public void MoveFile(string sourcePath, string destinationPath, bool overwrite)
        {
            if (onMove && sourcePath.Contains(failOn, StringComparison.Ordinal))
            {
                throw new IOException($"{destinationPath}: the move failed");
            }

            inner.MoveFile(sourcePath, destinationPath, overwrite);
        }
    }
}
