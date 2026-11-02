namespace Zax.Scripting;

/// <summary>
/// One element of <c>Setting.VisualStyleParameters</c> (AMD-059 REQ-LUA-116): a style parameter
/// a script has written, and the normalised value it was given.
/// </summary>
/// <param name="Style">
/// The style identifier as REQ-SCN-078 reads it back, so 4 for Enhanced even when the parameter
/// was written through the alias 1.
/// </param>
/// <param name="ParameterId">
/// The parameter identifier the script passed to <c>SetVisualStyleParameter</c>, such as
/// <c>"spl.bend_zx"</c>.
/// </param>
/// <param name="Value">The value after the normalisation of AMD-040 §3.</param>
public readonly record struct VisualStyleParameter(int Style, string ParameterId, double Value);
