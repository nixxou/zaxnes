using Zax.App.Persistence;

namespace Zax.App;

/// <summary>
/// What the command line asks the loop to start with: the entry of <c>-id=</c>
/// (SPEC-10 REQ-UI-007) and the slot of <c>-slot=&lt;n&gt;</c> (AMD-053 REQ-UI-213).
/// </summary>
/// <param name="GameId">The library entry to launch, or null.</param>
/// <param name="SlotId">
/// The slot to load once the launch is over — after step 7 of REQ-LOOP-021, the state having
/// gone to Running — or null. With no <see cref="GameId"/> it is a slot of another game, which
/// starts that game by itself the way a quick load across games does (REQ-PERS-052).
/// </param>
internal readonly record struct StartupRequest(string? GameId, string? SlotId);

/// <summary>
/// The rules of AMD-053 REQ-UI-213 for <c>-id</c> and <c>-slot</c>, written apart from the
/// window so that they are decided — and tested — without one.
/// </summary>
internal static class StartupLaunch
{
    /// <summary>
    /// Decides what to start. Every invalid value is ignored and journalled, never refused: the
    /// program starts normally in each of those cases.
    /// </summary>
    /// <param name="requestedGameId">The value of <c>-id=</c>, or null.</param>
    /// <param name="slotId">The value of <c>-slot=</c>, already known to be in the domain.</param>
    /// <param name="slots">The slot index, after the start-up consistency pass.</param>
    /// <param name="gameExists">Whether an identifier still names a library entry.</param>
    /// <param name="log">Where the ignored values are journalled.</param>
    public static StartupRequest Decide(
        string? requestedGameId,
        string? slotId,
        SaveSlotIndex slots,
        Func<string, bool> gameExists,
        Action<string> log)
    {
        ArgumentNullException.ThrowIfNull(slots);
        ArgumentNullException.ThrowIfNull(gameExists);
        ArgumentNullException.ThrowIfNull(log);

        string? game = null;
        if (requestedGameId is { Length: > 0 } requested)
        {
            if (gameExists(requested))
            {
                game = requested;
            }
            else
            {
                // REQ-UI-007: « Un identifiant inconnu est ignoré, et le menu s'affiche
                // normalement. »
                log($"-id={requested} matches no library entry; it is ignored (REQ-UI-007).");
            }
        }

        if (slotId is null)
        {
            return new StartupRequest(game, null);
        }

        if (slots.Find(slotId) is not { IsOccupied: true } slot)
        {
            // « Si le slot est vide, il est ignoré, le jeu de -id reste lancé et le fait est
            // journalisé. »
            log($"-slot={slotId} is empty; it is ignored and the start-up carries on (REQ-UI-213).");
            return new StartupRequest(game, null);
        }

        if (!gameExists(slot.GameId))
        {
            log($"-slot={slotId} holds a game that is no longer in the library; it is ignored.");
            return new StartupRequest(game, null);
        }

        if (game is null || !string.Equals(game, slot.GameId, StringComparison.Ordinal))
        {
            // « Si le slot appartient à un autre jeu, c'est le jeu du slot qui est lancé, comme
            // pour le slot rapide (REQ-PERS-052), et -id est alors sans effet. »
            if (game is not null)
            {
                log($"-slot={slotId} holds another game; -id={game} has no effect (REQ-UI-213).");
            }

            return new StartupRequest(null, slotId);
        }

        return new StartupRequest(game, slotId);
    }
}
