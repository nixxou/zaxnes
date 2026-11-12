# Amendement proposé de REQ-CORE-122 — le mapper et le bouton RESET

Proposition du rôle R3 (côté propre), lot **EVAL-SYNC-3**, 2026-11-10. **Non normatif** : ce
document n'est pas dans `spec/` et ne modifie rien ; il contient le libellé exact proposé, pour que
le propriétaire de la spec l'adopte, l'amende ou le refuse.

Décision du propriétaire du 2026-11-10 : le comportement est **corrigé dans le code** (lot
EVAL-SYNC-3 étape 4). Le présent document est la mise à jour de la spec que cette correction
demande, puisque le code implémente désormais le matériel et non plus la clause telle qu'elle est
écrite.

---

## 1. La clause en cause

**REQ-CORE-122** (AMD-044 §2, version 1.2), troisième tiret :

> - PPU, APU **et mapper** suivent leur état documenté *after reset* (par exemple : PPUCTRL et
>   PPUMASK à 0, canaux APU coupés, $4015 à 0) ;

Les trois exemples donnés sont tous du PPU ou de l'APU. Pour le mapper, la clause renvoie à un
« état documenté *after reset* » **qui n'existe pas**.

## 2. Pourquoi il n'existe pas

Le connecteur de cartouche de la NES **ne porte aucune broche `/RESET`**. La page nesdev
« Cartridge connector » énumère chaque broche des deux bords — le 60 broches du Famicom et le
72 broches de la NES : alimentation et masse, les lignes d'adresse, de données et de contrôle du
CPU, les lignes du PPU, CIRAM, l'audio, `/IRQ`, `M2`, `/ROMSEL`, `SYSTEM CLK`, les broches
d'extension et les deux du CIC. Il n'y a pas de `/RESET` parmi elles.

Un mapper ne peut donc pas voir l'appui sur le bouton RESET. Ses registres de banques, son unité
d'IRQ, son filtre de A12 et le miroitage qu'il sélectionne conservent ce que le jeu y a écrit en
dernier, à travers autant de resets que le joueur en demande. La page nesdev « CPU power up state »
le confirme par son silence : sa section *after reset* dit que la RAM interne est *« unchanged »* et
ne parle pas de la cartouche.

**Les cartes qui réagissent quand même existent**, et elles ne réagissent pas à une ligne de reset :
ce sont les **multicarts à détection de reset**, qui détectent l'**interruption du signal M2**.
nesdev, « NES 2.0 Mapper 313 » (multicarts à reset autour de l'ASIC MMC3) :

> *« When an interruption of the M2 signal is detected, the higher address lines are automatically
> incremented. »*

Et nesdev, « INES Mapper 215 », le dit de la carte que cette implémentation possède :

> *« The Outer Bank Register is reset to its power-up value if a Reset has been detected via an
> interruption of the M2 signal. This makes it difficult to dump these cartridges using a Kazzo
> device. »*

## 3. Le précédent de la spec : AMD-061 §5

La spec a déjà fait cette découverte une fois, pour le reset **dur**. AMD-061 §5 (Q-112) amende
REQ-CORE-004 :

> Le point « mapper revenu dans son état initial » de REQ-CORE-004 devient : « mapper revenu dans
> son état initial, **sauf pour une carte dont la documentation publique prescrit que son état
> survit au signal RESET** ; la liste de ces cartes est tenue par la spec ». Liste au 2026-11-04 :
> **mapper 28**.

Le libellé proposé ci-dessous est bâti sur le même moule, mais dans l'autre sens : pour le reset
**doux**, le défaut est la conservation et l'exception est la réaction. C'est la différence entre
une mise sous tension — où tout repart, carte comprise, puisque l'alimentation est coupée — et un
appui sur RESET, que la carte ne voit pas.

## 4. Libellé proposé

Remplacer le troisième tiret de REQ-CORE-122 par les deux tirets suivants :

> - PPU et APU suivent leur état documenté *after reset* (par exemple : PPUCTRL et PPUMASK à 0,
>   canaux APU coupés, $4015 à 0) ;
> - le **mapper est conservé tel que le jeu l'a laissé** — registres de banques, verrou de
>   sélection, protection de la PRG-RAM, unité d'IRQ, filtre de A12 et miroitage compris : le
>   connecteur de cartouche de la NES ne porte aucune broche `/RESET` (nesdev, « Cartridge
>   connector »), donc aucune carte ne peut voir l'appui sur le bouton. **Exception** : une carte
>   dont la documentation publique prescrit une réaction à un reset — les **multicarts à détection
>   de reset**, qui le détectent par l'interruption du signal `M2` (nesdev, « NES 2.0
>   Mapper 313 ») — suit cette documentation, et la liste de ces cartes est tenue par la spec.
>   Liste au 2026-11-10 : **mapper 215** (UNL-8237 / UNL-8237A), dont le registre de banque externe
>   `$5001` revient à sa valeur de mise sous tension (nesdev, « INES Mapper 215 », *Note*).
> - Critère d'acceptation (auto) : sur une image de mapper 4, après un changement de banque PRG et
>   l'armement de l'unité d'IRQ ($C000, $C001, $E001), un reset doux laisse la banque en place et
>   l'IRQ se déclenche à nouveau sans nouvelle écriture ; un reset **dur** remet les deux dans leur
>   état de mise sous tension. Sur une image de mapper 215, un reset doux remet `$5001` à sa valeur
>   de mise sous tension et laisse intacts les registres du MMC3 derrière lui.

Le reste de REQ-CORE-122 est inchangé : la RAM interne, la RAM de sauvegarde et la RAM de CHR
restent conservées, le CPU garde A, X, Y et ses drapeaux sauf `I`, les ports de manette sont remis
à zéro, et le compteur de frames repart de 0.

**REQ-CORE-004 et DEC-02-Q3 ne sont pas touchées** : le reset dur reste une mise sous tension, où
le mapper revient à son état initial — avec l'exception que AMD-061 §5 a déjà inscrite pour le
mapper 28. Les deux listes d'exception ne sont pas la même : celle de REQ-CORE-004 recense les
cartes dont l'état survit à une coupure du signal RESET, celle proposée ici recense les cartes qui
**réagissent** à une interruption de M2.

## 5. Ce que Q-078 devient

Q-078 enregistre la lecture des deux compagnons de la clause (la fenêtre d'inhibition du PPU, le
séquenceur de frames de l'APU) et ne parle pas du mapper. Il reste valable tel quel : les deux
points qu'il tranche sont du PPU et de l'APU, que le libellé proposé laisse mot pour mot où ils
étaient. Aucun nouveau ticket Q n'est demandé — `spec/SPEC-GUIDE.md` réserve un `Q-NNN` à une
ambiguïté qui **bloque** un implémenteur, et rien n'est bloqué : la clause a été implémentée telle
qu'écrite pendant deux lots, puis corrigée sur décision du propriétaire.

## 6. Ce que le code fait désormais

| | |
|---|---|
| `NesCore::soft_reset` (`crates/zax-nes/src/nes_core.rs`) | appelle `Cartridge::soft_reset` au lieu de `Cartridge::reset` |
| `Cartridge::soft_reset` (`crates/zax-nes/src/cartridges/mod.rs`) | appelle `Mapper::soft_reset`, puis rafraîchit les fenêtres rapides du bus (ce n'est pas un comportement : c'est la cohérence d'un cache) |
| `Mapper::soft_reset` (`crates/zax-nes/src/cartridges/mappers/mod.rs`) | **ne fait rien** par défaut, avec les références nesdev ci-dessus en commentaire |
| `Mmc3Unl8237::soft_reset` (`…/mappers/mmc3_unl8237.rs`) | la **seule** redéfinition : `$5001` revient à `$0F`, sa valeur de mise sous tension |
| `IrqBoard`, `SmallBoard`, `DecodeBoard`, `Mmc3ChrRam` | transmettent l'appel, pour que la carte 215 l'entende à travers leurs dispatches |
| le miroitage et le compteur de cycles M2 (`CartridgeMemory::cpu_cycle`) | **ne sont plus remis à zéro** par un reset doux ; `Cartridge::reset` les remet comme avant |

Mesures de l'étape 4 du lot :

| | |
|---|---|
| `zax-nes`, suite complète | **1 177 tests, 0 échec** (3 nouveaux dans `soft_reset_tests.rs`) |
| `zax-harness soft_reset_mapper_tests` | 3 tests, retournés dans l'autre sens et verts |
| `golden-core`, 98 originaux, 1 200 frames | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **identique à l'octet** à la valeur de référence de FCEUX-23, EVAL-SYNC et EVAL-SYNC-2 |

Le `golden-core` inchangé est attendu et c'est aussi la preuve que la correction est bien bornée :
`golden-core` appelle `NesCore::reset`, la mise sous tension, qui n'est pas touchée.
