using Zax.Profiles;
using Zax.Profiles.Import;
using Zax.Profiles.Tests;

namespace Zax.App.Tests;

/// <summary>
/// The command line of AMD-030 §2: <c>--import-3dsen &lt;folder|file&gt;</c> imports, prints the
/// summary and exits with 0 when it all worked.
/// </summary>
public sealed class ThreeDSenCliTests
{
    [Fact] // AMD-030 §2
    public void The_option_takes_a_folder_or_a_file()
    {
        AppOptions options = AppOptions.Parse(["--import-3dsen", @"C:\Games\3dSen"]);

        Assert.Equal(@"C:\Games\3dSen", options.ImportThreeDSenPath);
        Assert.False(options.SmokeTest);
        Assert.Null(options.RomPath);
    }

    [Fact] // AMD-030 §2: a malformed command line is a usage error, which Program maps to 2.
    public void The_option_without_a_value_is_a_usage_error()
    {
        ArgumentException error = Assert.Throws<ArgumentException>(
            () => AppOptions.Parse(["--import-3dsen"]));

        Assert.Contains("--import-3dsen", error.Message, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §2
    public void The_help_names_the_option()
        => Assert.Contains("--import-3dsen", AppOptions.Usage, StringComparison.Ordinal);

    [Fact] // AMD-030 §2: « importe puis quitte avec le code 0 si tout s'est bien passé »
    public void An_import_that_works_exits_with_zero_and_prints_the_summary()
    {
        using Sandbox sandbox = new();
        sandbox.WriteBundle(
            [("Pinball", Profile("ABCDEFGHIJKL")), ("Tetris", Profile("MNOPQRSTUVWX"))]);

        int code = sandbox.Run(sandbox.InstallRoot);

        Assert.Equal(0, code);
        Assert.Contains("2 profile(s) imported", sandbox.Output, StringComparison.Ordinal);
        Assert.Contains("0 failed", sandbox.Output, StringComparison.Ordinal);
        Assert.Empty(sandbox.Errors);
        Assert.Equal(2, Directory.GetFiles(sandbox.ProfilesDirectory, "*.3dn").Length);
    }

    [Fact] // AMD-030 §2: the bundle may be named directly.
    public void The_bundle_file_can_be_named_on_the_command_line()
    {
        using Sandbox sandbox = new();
        string bundle = sandbox.WriteBundle([("Pinball", Profile("ABCDEFGHIJKL"))]);

        Assert.Equal(0, sandbox.Run(bundle));
        Assert.True(File.Exists(Path.Combine(sandbox.ProfilesDirectory, "Pinball.3dn")));
    }

    [Fact] // AMD-030 §2: a second import adds nothing and still succeeds.
    public void A_second_import_reports_the_profiles_as_already_present()
    {
        using Sandbox sandbox = new();
        sandbox.WriteBundle([("Pinball", Profile("ABCDEFGHIJKL"))]);

        Assert.Equal(0, sandbox.Run(sandbox.InstallRoot));
        sandbox.Clear();
        Assert.Equal(0, sandbox.Run(sandbox.InstallRoot));

        Assert.Contains("0 profile(s) imported, 1 already present", sandbox.Output, StringComparison.Ordinal);
        Assert.Single(Directory.GetFiles(sandbox.ProfilesDirectory, "*.3dn"));
    }

    [Fact] // AMD-030 §5: one bad payload fails, the rest is imported, and the code is not 0.
    public void A_bundle_with_one_bad_payload_exits_with_one_and_imports_the_rest()
    {
        using Sandbox sandbox = new();
        sandbox.WriteBundle(
            [("junk", [1, 2, 3, 4]), ("Pinball", Profile("ABCDEFGHIJKL"))]);

        int code = sandbox.Run(sandbox.InstallRoot);

        Assert.Equal(1, code);
        Assert.Contains("1 profile(s) imported, 0 already present, 1 failed", sandbox.Output, StringComparison.Ordinal);
        Assert.Contains("junk", sandbox.Errors, StringComparison.Ordinal);
        Assert.Single(Directory.GetFiles(sandbox.ProfilesDirectory, "*.3dn"));
    }

    [Fact] // AMD-030 §3: nothing found — a clear message and no file written.
    public void An_installation_without_a_bundle_exits_with_one()
    {
        using Sandbox sandbox = new();
        Directory.CreateDirectory(Path.Combine(sandbox.InstallRoot, "3dSen_Data"));

        int code = sandbox.Run(sandbox.InstallRoot);

        Assert.Equal(1, code);
        Assert.Contains("no 3dSen profile bundle was found", sandbox.Errors, StringComparison.Ordinal);
        Assert.Contains(ThreeDSenInstallation.ExpectedLayout, sandbox.Errors, StringComparison.Ordinal);
        Assert.Empty(sandbox.Output);
        Assert.False(Directory.Exists(sandbox.ProfilesDirectory)
            && Directory.GetFiles(sandbox.ProfilesDirectory).Length > 0);
    }

    [Fact] // AMD-030 §5: a corrupt bundle never crashes and writes nothing.
    public void A_corrupt_bundle_exits_with_one_without_writing()
    {
        using Sandbox sandbox = new();
        string bundle = sandbox.WriteBundle([("Pinball", Profile("ABCDEFGHIJKL"))]);
        byte[] whole = File.ReadAllBytes(bundle);
        File.WriteAllBytes(bundle, whole[..(whole.Length / 2)]);

        int code = sandbox.Run(sandbox.InstallRoot);

        Assert.Equal(1, code);
        Assert.NotEmpty(sandbox.Errors);
        Assert.DoesNotContain("profile(s) imported", sandbox.Output, StringComparison.Ordinal);
        Assert.False(Directory.Exists(sandbox.ProfilesDirectory)
            && Directory.GetFiles(sandbox.ProfilesDirectory).Length > 0);
    }

    [Fact] // AMD-030 §3
    public void A_path_that_does_not_exist_exits_with_one()
    {
        using Sandbox sandbox = new();

        int code = sandbox.Run(Path.Combine(sandbox.Root, "nowhere"));

        Assert.Equal(1, code);
        Assert.Contains("neither a file nor a folder", sandbox.Errors, StringComparison.Ordinal);
    }

    [Fact] // AMD-030 §5: the profile is written byte for byte as the bundle holds it.
    public void The_written_file_is_the_payload_unchanged()
    {
        using Sandbox sandbox = new();
        byte[] profile = Profile("ABCDEFGHIJKL");
        sandbox.WriteBundle([("Blaster Master (U)", profile)]);

        Assert.Equal(0, sandbox.Run(sandbox.InstallRoot));

        string written = Path.Combine(sandbox.ProfilesDirectory, "Blaster Master (U).3dn");
        Assert.Equal(profile, File.ReadAllBytes(written));
    }

    /// <summary>A profile the SPEC-03 decoder accepts.</summary>
    private static byte[] Profile(string id)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Id = id;
        profile.Title = id;
        return ProfileFormat.Save(profile);
    }

    /// <summary>
    /// A temporary 3dSen installation and a temporary profile folder, both outside the
    /// repository: no file of 3dSen and no imported profile ever lands in the tree.
    /// </summary>
    private sealed class Sandbox : IDisposable
    {
        private readonly StringWriter _output = new();
        private readonly StringWriter _errors = new();

        public Sandbox()
        {
            Root = Path.Combine(Path.GetTempPath(), "zax-p8-cli-" + Guid.NewGuid().ToString("N"));
            InstallRoot = Path.Combine(Root, "3dSen");
            ProfilesDirectory = Path.Combine(Root, "userdata", "profiles");
            Directory.CreateDirectory(InstallRoot);
        }

        public string Root { get; }

        public string InstallRoot { get; }

        public string ProfilesDirectory { get; }

        public string Output => _output.ToString();

        public string Errors => _errors.ToString();

        /// <summary>Writes a synthetic bundle in the layout of AMD-030 §3.</summary>
        public string WriteBundle(IReadOnlyList<(string Name, byte[] Script)> assets)
        {
            string directory = Path.Combine(
                InstallRoot, "3dSen_Data", "StreamingAssets", "aa", "StandaloneWindows");
            Directory.CreateDirectory(directory);

            string bundle = Path.Combine(
                directory,
                ThreeDSenInstallation.BundlePrefix + "0123456789abcdef"
                + ThreeDSenInstallation.BundleExtension);
            File.WriteAllBytes(
                bundle,
                SyntheticUnityBundle.Build(
                    assets,
                    new SyntheticBundleOptions
                    {
                        BlockCompression = UnityCompression.Lz4,
                        BlockSize = 128 * 1024,
                    }));
            return bundle;
        }

        public void Clear()
        {
            _output.GetStringBuilder().Clear();
            _errors.GetStringBuilder().Clear();
        }

        public int Run(string source)
            => ThreeDSenImportCommand.Run(source, ProfilesDirectory, _output, _errors);

        public void Dispose()
        {
            _output.Dispose();
            _errors.Dispose();
            try
            {
                Directory.Delete(Root, recursive: true);
            }
            catch (IOException)
            {
                // A temporary folder that will not go away is not a test failure.
            }
        }
    }
}
