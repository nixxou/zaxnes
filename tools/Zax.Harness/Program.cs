using System.Globalization;
using System.Reflection;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Scripting;

namespace Zax.Harness;

/// <summary>
/// Headless entry point.
/// </summary>
/// <remarks>
/// Without a command it prints its version and the modules it is linked against, as the P0
/// scaffold did. The <c>meshes</c> command exports the geometry of SPEC-04B
/// (<see cref="MeshExport"/>); replaying a ROM with an input sequence and the offscreen
/// captures of REQ-ARCH-031 come with the later lots.
/// </remarks>
internal static class Program
{
    private static int Main(string[] args)
    {
        if (args.Length > 0)
        {
            switch (args[0].ToLowerInvariant())
            {
                case "meshes":
                    return MeshExport.Run(args[1..]);

                case "render":
                    return RenderExport.Run(args[1..]);

                case "lua-api":
                    return LuaApiExport.Run(args[1..]);

                case "stress":
                    return StressRun.Run(args[1..]);

                case "help":
                case "--help":
                case "-h":
                    Console.Out.WriteLine(MeshExport.Usage);
                    Console.Out.WriteLine();
                    Console.Out.WriteLine(RenderExport.Usage);
                    Console.Out.WriteLine();
                    Console.Out.WriteLine(LuaApiExport.Usage);
                    Console.Out.WriteLine();
                    Console.Out.WriteLine(StressRun.Usage);
                    return 0;

                default:
                    Console.Error.WriteLine($"unknown command: {args[0]}");
                    Console.Error.WriteLine();
                    Console.Error.WriteLine(MeshExport.Usage);
                    Console.Error.WriteLine();
                    Console.Error.WriteLine(RenderExport.Usage);
                    Console.Error.WriteLine();
                    Console.Error.WriteLine(LuaApiExport.Usage);
                    Console.Error.WriteLine();
                    Console.Error.WriteLine(StressRun.Usage);
                    return 2;
            }
        }

        Console.Out.WriteLine(string.Create(CultureInfo.InvariantCulture, $"Zax.Harness {Version}"));

        foreach (string module in new[]
                 {
                     $"{NesModule.Name}: {NesModule.Role}",
                     $"{ProfilesModule.Name}: {ProfilesModule.Role}",
                     $"{Engine3DModule.Name}: {Engine3DModule.Role}",
                     $"{ScriptingModule.Name}: {ScriptingModule.Role}",
                 })
        {
            Console.Out.WriteLine(string.Create(CultureInfo.InvariantCulture, $"  {module}"));
        }

        Console.Out.WriteLine();
        Console.Out.WriteLine("commands: meshes, render, lua-api, stress, help");
        return 0;
    }

    /// <summary>Informational version stamped by Directory.Build.props.</summary>
    internal static string Version =>
        typeof(Program).Assembly
            .GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion
        ?? typeof(Program).Assembly.GetName().Version?.ToString()
        ?? "unknown";
}
