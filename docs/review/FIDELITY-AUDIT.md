# Audit de fidélité de Zaxnes face au matériel

> Rapport d'un audit indépendant fait par un agent IA (Fable 5.2) le 17/11/2026, copié tel quel
> depuis le dossier de travail du laboratoire (`ext-tools/fidelity-audit/`, hors dépôt : ses
> programmes, bancs et traces n'y sont pas). Les corrections de la section « Suites données » sont
> dans `dev` depuis la fusion de `feat/c3c-collision`, et dans Zaxnes 0.6.0.

Date : 2026-11-17. Tout est dans `ext-tools\fidelity-audit\` (chemins relatifs à `G:\3DSen`). Aucun worktree n'a été modifié, aucun commit.

Version auditée : `wt-trial`, branche `feat/c3c-collision`, tête `79aeba0b`. Binaires utilisés tels quels : `wt-trial\rust\target\release\zax-harness.exe` et `ext-tools\visual2c07-tools\zax-harness-c3c-fin.exe` (même taille et même date que `wt-trial\rust\target-dot\release\zax-harness.exe`, construit sur `f1b67088` ; `79aeba0b` n'ajoute que de la documentation).

## Conclusion

**Accords (confiance haute)**

- **CPU 2A03, cycle par cycle.** Sur 36 000 cycles couvrant 230 opcodes (officiels et non officiels) avec trois jeux de valeurs, le bus de Zaxnes (adresse, sens, donnée de chaque cycle, lectures et écritures factices comprises) est celui de la netlist transistor, sauf sur 7 opcodes non officiels où c'est la simulation qui est en cause (voir plus bas).
- **Branches, `JMP ($xxFF)`, `JSR`/`RTS`, `BRK`/`RTI`, retards d'IRQ de `CLI`/`SEI`/`PLP`, `RTI` sans retard** : identiques à la netlist.
- **DMA du DMC et de l'OAM.** 21 906 cycles, 66 DMA : chaque DMA tombe au même cycle et dure autant (3, 4, 513, 514, 516, 517, 518 cycles), y compris un DMA du DMC au milieu d'un DMA d'OAM et pendant des lectures de `$2002`, `$2007`, `$4016`. Zéro écart.
- **Compteur de trame de l'APU.** 742 000 cycles en quatre programmes : top de demi-trame du mode 5 temps, du mode 4 temps, drapeau d'IRQ de trame lu à `$4015` (y compris son apparition fugitive avec l'IRQ inhibée), prise de l'IRQ, pour les deux parités d'écriture de `$4017`. Zéro écart.
- **ROMs de test publiques.** 215 passent sur 219 verdicts lisibles ; les 4 échecs sont attendus (3 ROMs MMC3 révision A, `power_up_palette`). `AccuracyCoin` : 147/147. `holy-mapperel` : 40/40. Le portage de NesHawk lancé par le même harness en passe 199 et en échoue 20.
- **Registres `$2000`, `$2001`, `$2005`, `$2006`, lecture `$2007` en plein rendu** (`v` et `t` point par point, alignement netlist e4, mode Fidélité+) : identiques à la netlist sur 4 trames et environ 1 750 accès.

**Écarts trouvés, par gravité**

1. **IRQ du DMC en avance (gravité moyenne, confiance haute sur la mesure).** La netlist abaisse l'entrée IRQ du cœur 6502 trois cycles après la lecture DMA du dernier octet (78 cas sur 78). Zaxnes lève le drapeau à la lecture. Résultat : 36 arrivées d'IRQ sur 79 sont prises une instruction plus tôt dans Zaxnes.
2. **Écriture `$2007` pendant le rendu (gravité faible à moyenne, confiance haute).** La netlist applique l'incrément double de `v` au point D+6 ; Zaxnes le montre dès l'accès, en mode standard comme en Fidélité+. Sur environ 150 écritures, 136 diffèrent, dont 14 sur plus de 12 points (jusqu'à une ligne entière) ; les autres pendant 5 points.
3. **Course `$2002` / début de vblank dépendante de l'alignement (gravité faible, confiance moyenne).** Zaxnes reproduit exactement un alignement de la netlist sur quatre (e2 : 14 trames sur 14, pour la levée comme pour la retombée du drapeau et pour l'entrée du NMI). Dans les trois autres, une position d'un point diffère. Zaxnes n'a pas de dépendance à l'alignement pour cette course.
4. **`$2001` basculé en cours de ligne, famille d'alignement e0 (gravité faible, confiance moyenne).** 54 écritures sur environ 300 laissent `v` différent de la netlist (incrément parasite de Y grossier quand le rendu est coupé ou remis sur un point d'incrément). Aucun écart dans la famille e4.
5. **Mode standard seulement : lecture `$2007` pendant le rendu** : 147 lectures sur environ 150 diffèrent quelques points (le mode standard n'a pas la latence de 5 à 6 points). Le mode Fidélité+ les a toutes.
6. **Option `netlist_get_put` (remarque).** Sa phase DMC (431, 8 bits, parité get/put inversée) ne recale pas Zaxnes sur mon banc : il faut 1014, 8 bits, parité non inversée.

**Non concluant**

- **`$2004` pendant le rendu, sprite 0, débordement de sprites sur la netlist.** Dans mon banc les écritures `$2003`/`$2004` n'atteignent pas les cellules d'OAM et le sprite 0 ne touche jamais. Avec l'OAM posé directement, les points 1-64 et 321-340 concordent (170 lectures sur 170, avec un retard uniforme d'un point), mais 40 % des lectures de la phase d'évaluation diffèrent d'une façon que je n'ai pas su attribuer. Les ROMs de test (18 ROMs, toutes réussies) restent la seule référence pour ce domaine.
- **7 opcodes non officiels** (`ANC` ×2, `ALR`, `ARR`, `XAA`, `LXA`, `LAS`) : la netlist contredit Zaxnes, mais `instr_test-v5` et `AccuracyCoin`, issus du matériel, donnent raison à Zaxnes. Je classe l'écart comme limite de la simulation à interrupteurs.
- **Comparaison de jeux avec le portage NesHawk** : instrument trop sensible à l'état de mise sous tension pour conclure.

**Non couvert par cet audit** : PAL (2C07) sur netlist, sorties audio des canaux, MMC5, MC-ACC et les autres mappers au-delà des ROMs de test, comparaison avec FCEUX (le harness fourni n'a pas ces sous-commandes), sources de Mesen (le dossier local ne contient que des binaires).

## Méthode

Trois sources indépendantes, par ordre de poids :

1. **Netlist transistor** (VisualNES, 2A03 + 2C02). J'ai écrit pour l'audit mes propres programmes 6502 et mon propre banc, sans reprendre le corpus existant. Le banc écrit une ligne par cycle CPU ; Zaxnes fait de même avec `cpu-bus-trace`. Les deux traces sont comparées cycle par cycle.
2. **ROMs de test publiques**, lues par les lecteurs du harness (protocole `$6000` de blargg, écran, octets de résultat d'AccuracyCoin).
3. **Autre émulateur** : le portage Rust de NesHawk intégré au harness, sur les mêmes ROMs.

Paramètres libres de la comparaison netlist, à connaître pour juger les résultats :

| Paramètre | Valeur retenue | Fixé sur | Vérifié sur |
|---|---|---|---|
| Phase du DMC à la mise sous tension (`--dmc`) | 1014, 8 bits | `dmcdma` | `cpuflow`, lecture directe du compteur netlist (premier top au cycle 1014) |
| Parité get/put | celle par défaut | `dmcdma` | `apuL5`, `apuL4`, `apuF`, `apuG` (zéro écart) |
| Phase CPU/PPU (`--phase`) | -9 pour e0 et e2, -7 pour e4 et e6 : le point d'accès de Zaxnes égale le point où /CS tombe | `scr` | `vblA`, `vblC` en e2 (zéro écart) |
| Alignement (`--alignment`) | 1 pour e0, 3 pour e4 | corpus FIDPLUS | `scr` : les quatre valeurs essayées |

`--alignment` n'agit pas sur `$2002` ni sur `$2004` (résultats identiques pour 0 à 3) ; il agit sur `$2001`, `$2006`, `$2007`.

## Programmes et bancs

Sources : `asm6.py` (assembleur), `mkprogs.py` (tous les programmes), `benchaud.cpp` + `buildaud.bat` (banc), `progs\` (binaires).

| Programme | Contenu | Cycles |
|---|---|---|
| `cpuall` | 230 opcodes × 3 jeux (A, X, Y, P, M) ; après chacun `PHP`, `PHA`, `STX`, `STY` sortent l'état sur le bus. Modes indexés avec et sans franchissement de page. | 36 000 |
| `cpuflow` | 32 branches, branches à cheval sur une page, `JMP ($02FF)`, `JSR`/`RTS`, `BRK`, `RTI`, puis IRQ du DMC (échantillon d'un octet relancé par le gestionnaire) sur `CLI`, `SEI`, `PLP`, `RTI`, branches prises, instructions de 2 à 7 cycles | 42 000 |
| `dmcdma` | DMC au débit 15 sur une boucle de 65 cycles (premier avec 432) lisant `$4016`, `$2002`, `$2007`, `$4017`, puis 24 DMA d'OAM pendant que le DMC joue, puis `$4014` aux deux parités | 36 000 |
| `apuL5`, `apuL4` | Longueur 2 sur le canal carré 1, 48 lectures `$4015` espacées de 4 cycles autour du top attendu, 8 essais (2 parités × 4 décalages) | 126 000, 246 000 |
| `apuF` | Même fenêtre sur le drapeau d'IRQ de trame, I = 1 | 246 000 |
| `apuG` | IRQ de trame prise dans une suite de `NOP`, 4 décalages | 124 000 |
| `vblA`, `vblC` | Rendu coupé, NMI actif, une sonde de deux lectures `$2002` par trame ; tour de 29 781 cycles, donc un point de glissement par trame. A : autour de (241,1). C : autour de (261,1). | 450 000 chacun |
| `oam` | Deux sprites par ligne, 10 sprites sur la ligne 100 ; une lecture `$2004` et une lecture `$2002` par ligne, glissant d'un point par ligne | 130 000 |
| `hit` | Sprite 0 opaque en (128, 99) sur fond opaque, lecture `$2002` glissant de 1,5 point par trame | 460 000 |
| `scr` | Rendu actif ; sur 7 lignes : paire `$2006`, paire `$2005`, deux `$2000`, lecture `$2007`, `$2001` coupé puis remis, `$2006` puis `$2002`, écriture `$2007` ; un point de glissement par ligne | 4,4 M demi-pas |

Banc `benchaud` : variante « hold » des bancs existants (`visualnes-bench\macros_hold.cpp`, `datadefs_hold.cpp`). Il échantillonne adresse, R/W et donnée pendant phi2 et écrit la ligne au front descendant. Trois ajouts :

- pour une lecture de `$4015`, la donnée est prise sur le bus interne du cœur (`cpu_c_db`), le 2A03 ne la sortant pas sur ses broches ;
- colonne `q=` : entrée IRQ du cœur (`cpu_c_irq`) ;
- argument 7 : OAM posé directement dans les cellules (`sprite_write`), voir « Limites ».

Vitesse : environ 1 100 cycles CPU par seconde et par processus.

`scr` passe par `visualnes-bench\benchdot2.exe` et les comparateurs `dotcmp.py` et `buscmp.py` existants.

## Résultats : CPU

### `cpuall`

```
cycles compares : 35991 ; desaccords de donnee : 39 ; d adresse/sens : 6 ; glissements : 0
```

Les 45 désaccords viennent de 7 opcodes (détail dans `results\cpuall.txt`) :

| Opcode | Jeu | Netlist | Zaxnes |
|---|---|---|---|
| `ANC #` (`$0B`, `$2B`) | A=`$80`, opérande `$43` | A=`$80`, P=`$F5` | A=`$00`, P=`$76` |
| `ALR #` (`$4B`) | idem | A=`$40` | A=`$00` |
| `ARR #` (`$6B`) | idem | A=`$C0` | A=`$80` |
| `XAA #` (`$8B`) | idem | A=`$00`, P=`$77` | A=`$40`, P=`$75` |
| `LXA #` (`$AB`) | A=`$5A`, opérande `$FF` | A=X=`$5A` | A=X=`$FF` |
| `LAS abs,Y` (`$BB`) | M=`$C3`, S=`$FF` | S=`$FF`, A=`$D3`, X=`$FF` | S=A=X=`$C3` |

Dans la netlist, le `ET` avec l'opérande n'a pas lieu pour les immédiats de la colonne `xB`. Ces opcodes reposent sur un conflit de bus interne, que la simulation à interrupteurs ne résout pas comme le silicium. `instr_test-v5` (`02-immediate`, sommes de contrôle relevées sur console) et `AccuracyCoin` passent dans Zaxnes : je retiens Zaxnes. `SHA`, `SHX`, `SHY`, `TAS`, `SAX`, `LAX`, `DCP`, `ISC`, `SLO`, `RLA`, `SRE`, `RRA`, `AXS`, `SBC $EB` et les `NOP` non officiels concordent avec la netlist, cycles factices compris.

### `dmcdma`

```
cycles compares : 21906 ; desaccords de donnee : 0 ; d adresse/sens : 0 ; glissements : 0
DMA cote Zaxnes : 66 evenements, 14085 cycles, longueurs {3: 4, 4: 35, 513: 2, 514: 1, 516: 19, 517: 1, 518: 4}
```

Obtenu avec `--dmc 1014,8`. Les données lues à `$4016`/`$4017` sont exclues (la netlist n'a pas de manette). Avec la phase par défaut ou avec `--dmc 431,8 --flip-get-put`, les DMA tombent 156 à 166 cycles trop tôt.

### `cpuflow` : écart 1, IRQ du DMC

Jusqu'au cycle 5 720, tout concorde : les 32 branches, les branches à cheval, `JMP ($02FF)` qui lit `$0200`, `BRK`, `RTI`, et les six séquences d'IRQ en attente (`CLI NOP`, `CLI SEI`, `PLP`, `RTI`, `CLI SEI CLI`, `CLI LDA`).

Ensuite, quand l'IRQ arrive pendant une instruction :

```
cycle  netlist            Zaxnes
5716   C000 R 00          (DMA)              lecture DMA du dernier octet
5717   965D R D0          965D R D0          BNE
5718   965E R EA          965E R EA
5719   965F R A9          965F R A9          branche prise
5720   9649 R 50          9649 R 50          BVC
5721   964A R 00          9649 R 50          <- Zaxnes entre en IRQ
5722   964B R 50          01FF W 96
5725   01FF W 96          FFFE R 00          <- la netlist entre 3 cycles plus tard
```

- Netlist : `cpu_c_irq` est haut au cycle R+2 et bas au cycle R+3, R étant la lecture DMA. 78 cas sur 78 (`results\cpuflow.txt`).
- Zaxnes : `apu\dmc_channel.rs` ligne 327 lève `irq_flag` dans la fonction qui reçoit l'octet.
- Latences de la lecture DMA au vecteur IRQ, (netlist, Zaxnes) : 43 égales ; (12, 9) × 11 ; (13, 10) × 11 ; (14, 8) × 14.
- Les 401 cycles en désaccord du programme découlent tous de ces 36 arrivées (adresse de retour empilée, puis octets de pile relus).

L'IRQ de trame, elle, est prise au même cycle dans les deux (`apuG`). L'écart est donc propre au chemin du drapeau DMC.

## Résultats : APU

```
== apuL5   cycles compares : 125991 ; desaccords : 0
== apuL4   cycles compares : 245991 ; desaccords : 0
== apuF    cycles compares : 245991 ; desaccords : 0
== apuG    cycles compares : 123991 ; desaccords : 0
```

Transitions vues par la netlist, en cycles après l'écriture `$4017` (`results\apu.txt`) :

- mode 5 temps : la longueur tombe à zéro entre +14 917 et +14 920 selon le décalage de lecture ;
- drapeau d'IRQ de trame : premier `$40` entre +29 831 et +29 835.

Zaxnes rend les mêmes octets aux mêmes cycles, y compris le `$40` lu une fois avec l'IRQ inhibée (`apuL4`).

Non mesuré : séquenceurs et sorties des canaux, balayage, enveloppes, mélangeur. Seules les ROMs les couvrent (`apu_test` 9/9, `blargg_apu_2005` 11/11, `apu_mixer` 4/4, `apu_reset` 6/6, `pal_apu_tests` 10/10).

## Résultats : PPU

### `vblA` : levée du drapeau, écart 3

Position = point où /CS tombe, ligne 241. « 0, sans NMI » : la lecture rend 0 et le drapeau ne se lève pas.

| Point | e0 | e2 | e4, e6 | Zaxnes |
|---|---|---|---|---|
| 340 (ligne 240) | normal | normal | **0, sans NMI** | normal |
| 0 | 0, sans NMI | 0, sans NMI | 0, sans NMI | 0, sans NMI |
| 1 | 1, sans NMI | 1, sans NMI | 1, sans NMI | 1, sans NMI |
| 2 | **1, NMI** | 1, sans NMI | **1, NMI** | 1, sans NMI |
| 3 et après | 1, NMI | 1, NMI | 1, NMI | 1, NMI |

- e2 contre Zaxnes `--phase -9` : 14 trames sur 14.
- e0 : au mieux 1 trame fausse (point 2). e4 et e6 : au mieux 1 trame fausse (point 340 ou point 0 selon la phase).
- Le motif de Zaxnes (1 point puis 2 points) est celui que documente nesdev.

### `vblC` : retombée du drapeau et entrée du NMI

- e2 contre `--phase -9` : 14 sur 14, pour la retombée du drapeau comme pour le cycle d'entrée du NMI.
- e0 : avec `-9` la retombée concorde mais l'entrée du NMI change d'instruction une trame plus tôt dans la netlist (trames 4 et 10) ; avec `-8` c'est l'inverse.
- e4, e6 : avec `-6` le NMI concorde et la retombée est vue un point plus tard dans la netlist ; avec `-7` c'est l'inverse.

Lecture commune aux deux programmes : entre la lecture de `$2002` et la détection du NMI par le CPU, le décalage vaut un point de moins en e0 et un point de plus en e4/e6 qu'en e2. Zaxnes a le décalage de e2.

Réserve : ces positions dépendent du modèle de décodage de /CS du banc (74LS139 à un demi-pas), qui n'est pas dans la netlist. La différence entre alignements, elle, vient des puces.

### `scr` : `v` et `t` point par point

Accès sur toute la trace netlist (un peu plus de 5 trames, dont 4 appariées) : `W2006` 578, `W2000` 385, `W2001` 385, `W2005` 384, `R2007` 192, `R2002` 192, `W2007` 192. Quatre trames appariées, tous les accès sur le même point (`offsets {0: n}`).

Suites de points où `v` ou `t` diffèrent, classées par l'accès voisin (`results\scr.txt`) :

| Comparaison | `W2007` | `R2007` | `W2001` | `W2006` | `W2000` | sans accès proche |
|---|---|---|---|---|---|---|
| e4 contre alignement 3, Fidélité+ | 136 | 0 | 0 | 0 | 1 | 16 |
| e4 contre alignement 3, standard | 147 | 147 | 0 | 12 (2 points) | 1 | 156 |
| e0 contre alignement 1, Fidélité+ | 77 | 0 | 54 | 4 | 0 | 204 |
| e0 contre alignement 1, standard | 86 | 148 | 65 | 14 | 0 | 269 |

**Écart 2, `W2007`.** Exemple (e4, ligne 6, écriture au point 31) : aux points 32 à 36 la netlist a `v` = `$1535`, Zaxnes `$2536` ; ils se rejoignent au point 37. Quand l'incrément retardé tombe sur un incrément normal, la ligne entière diffère (ligne 41 : `$1539` contre `$253A` jusqu'au point 256).

**Écart 4, `W2001` en e0.** Exemple (ligne 88, `$2001`=`$00` au point 111) : la netlist passe de `$711F` au point 112 à `$713F` au point 113 ; Zaxnes reste à `$711F`. Les alignements 0, 2 et 3 de Zaxnes donnent le même compte (54) ou pire.

S'y ajoute une suite `R2002` (339 points) dans les deux lignes e0. Les suites « sans accès proche » sont les lignes qui héritent d'un `v` déjà différent. Le bus d'adresse (`results\scr-bus.txt`) ne dit rien de plus : ses écarts découlent de `v`, et les adresses de motifs de sprites diffèrent parce que `scr` n'initialise pas l'OAM.

### `oam` : `$2004` pendant le rendu (non concluant)

Méthode : carte Zaxnes (ligne, point) → valeur, bâtie sur 19 phases, puis pour chaque lecture netlist recherche du décalage k tel que la carte en (ligne, point + k) donne la valeur netlist (`latmap.py`, `results\oam2004.txt`). Mêmes chiffres en e0 et e4.

| Points | Lectures | Concordent à k = +1 |
|---|---|---|
| 1-64 | 128 | 128 |
| 65-256 | 469 | 282 |
| 257-320 | 116 | 107 |
| 321-340, 0 | 42 | 42 |

Deux motifs dans les désaccords :

- **Évaluation (65 à environ 190).** Sur des groupes de lignes, la netlist lit le sprite suivant : ligne 80, point 86, netlist `$30`, Zaxnes `$2C` au point 87 et `$30` au point 89.
- **Après le 64e sprite.** Zaxnes alterne la valeur d'OAM et `$FC` ; la netlist alterne la valeur d'OAM et `$00` : ligne 201, point 207, netlist `$00`, Zaxnes `$FC`.
- **Lignes 0 à 7, points 269-276** : netlist `$00`, Zaxnes `$FF`.

Je ne peux pas dire qui a raison : voir « Limites ». Les drapeaux de `$2002` lus par le même programme (bits 7 à 5, 820 lectures) sont compatibles, mais une lecture par ligne ne situe pas le point de bascule.

### `hit` : sprite 0 (non concluant)

La netlist ne lève jamais le bit 6 : 30 lectures, toutes `$1E` (`results\hit.txt`). Le banc ne rend pas le sprite, je n'ai pas de mesure.

## Résultats : ROMs de test

Campagne complète : `testroms-zaxnes.md` (rapport du harness), `testroms-zaxnes.json`, `testroms-zaxnes.log`.

| Résultat | ROMs |
|---|---|
| réussies | 215 |
| échecs | 4 |
| illisibles automatiquement (« manual ») | 111 |
| image refusée | 1 (`LINUSMUS.NES`, sans signature iNES) |

Par domaine, verdicts lisibles :

| Domaine | Suites | Réussies |
|---|---|---|
| Instructions | `instr_test-v3`, `-v5`, `nes_instr_test`, `instr_misc` | 51/51 |
| Timing CPU | `instr_timing`, `cpu_timing_test6`, `branch_timing_tests` | 7/7 |
| Lectures et écritures factices, espace d'exécution, reset | `cpu_dummy_reads`, `cpu_dummy_writes`, `cpu_exec_space`, `cpu_reset` | 7/7 |
| Interruptions | `cpu_interrupts_v2` | 6/6 |
| Vblank et NMI | `ppu_vbl_nmi`, `vbl_nmi_timing` | 18/18 |
| Sprites | `sprite_hit_tests`, `sprite_overflow_tests`, `oam_read`, `oam_stress` | 18/18 |
| PPU divers | `blargg_ppu_tests`, `ppu_open_bus`, `ppu_read_buffer` | 6/7 |
| APU | `apu_test`, `blargg_apu_2005`, `apu_mixer`, `apu_reset`, `pal_apu_tests` | 40/40 |
| DMA | `dmc_dma_during_read4`, `sprdma_and_dmc_dma` | 5/5 lisibles |
| MMC3 | `mmc3_test`, `mmc3_test_2`, `mmc3_irq_tests` | 15/18 |
| Cartes | `holy-mapperel` | 40/40 |
| Tout | `AccuracyCoin` | 147/147 tests |

Les 4 échecs :

- `mmc3_irq_tests/5.MMC3_rev_A`, `mmc3_test/6-MMC6`, `mmc3_test_2/6-MMC3_alt` : comportement de la révision A ; Zaxnes fixe le mapper 4 à la révision B et la ROM jumelle passe dans chaque suite.
- `blargg_ppu_tests/power_up_palette` : code `$02`, la palette à la mise sous tension n'est pas celle de la console de l'auteur (le readme la dit propre à sa console).

Deux ROMs « manual » dont j'ai vérifié la sortie contre le source :

- `dma_2007_read` : `44 55`, somme `5E3DF9C4`, l'une des deux sorties admises ;
- `double_2007_read` : `22 44 55 66 77`, somme `85CFD627`, l'une des quatre admises.

Les 109 autres « manual » sont des démos, des tests visuels ou sonores (`dmc_tests`, `nmi_sync`, `scanline`, `mmc5test`, `mmc5exram`, `MMC1_A12`, `read_joy3`…) : je ne les ai pas jugées, aucune capture n'a été faite.

### Même campagne sur le portage NesHawk

`testroms-neshawk.md`. 199 réussies, 20 échecs. Différences de verdict :

- Zaxnes passe, NesHawk échoue (18) : `AccuracyCoin` (130/147), 12 ROMs MMC1 de `holy-mapperel`, `apu_test` et `7-dmc_basics`, `dma_4016_read`, les deux `sprdma_and_dmc_dma`.
- NesHawk passe, Zaxnes échoue (2) : `power_up_palette`, `5.MMC3_rev_A`.

### Jeux, tuiles et pixels contre NesHawk (non concluant)

`results\games-neshawk.txt`, 18 jeux de `demo\roms`, trames 60, 300, 1200, 3000, sans entrée. 11 jeux ont des pixels identiques aux quatre trames. 7 divergent (Blaster Master, Excitebike, Final Fantasy, Mega Man 2, Metroid, SMB3, Zelda II Europe). Les images diffèrent dès la trame 2 pour tous : les deux moteurs ne partent pas du même état, et une démo sans entrée amplifie ce décalage. Je n'en tire rien.

## Limites des outils

- **Netlist à interrupteurs.** Niveaux logiques seulement : ni conflits de bus analogiques, ni décharge des nœuds dynamiques (bus ouvert, OAM). C'est la cause probable des 7 opcodes.
- **État de mise sous tension.** Tous les nœuds partent de zéro : phase du DMC, parité get/put, drapeau de débordement levé, `$2001` non nul (le rendu est actif tant que le programme n'écrit pas `$2001`). Ma première version de `vbl` s'est fait piéger ; `vblA` et `vblC` écrivent `$2001`.
- **OAM du banc.** Les écritures `$2003`/`$2004` n'adressent que la cellule 0 et un DMA d'OAM abîme les autres cellules (`net\o4t.txt`, `progs\o4t.bin`). Pour `oam` et `hit`, l'OAM est posé par `sprite_write` et la variante netlist du programme (`oamn`, `hitn`) envoie ses écritures à `$2002`, à cycles égaux. `OAMADDR` n'est donc jamais écrit côté netlist. Les résultats `$2004` sont à prendre avec cette réserve.
- **Décodage de /CS et temps de maintien** : modèle du banc « hold », hors netlist.
- **Lecteurs du harness.** Les verdicts des ROMs sont ceux du harness de Zaxnes ; je n'ai recoupé à la main que les deux ROMs citées.
- **`--fidelity-plus` et `--alignment`** sont sans effet sur `cpu-bus-trace` pour `$2002` et `$2004`.

## Écarts qui demandent une correction

| # | Écart | Où | Mesure à refaire après correction |
|---|---|---|---|
| 1 | Drapeau d'IRQ du DMC levé à la lecture DMA, la netlist n'abaisse la ligne que 3 cycles après | `apu\dmc_channel.rs`, levée de `irq_flag` | `cpuflow` : 0 désaccord attendu |
| 2 | Incrément de `v` d'une écriture `$2007` en plein rendu : au point D+6 | PPU, écriture `$2007`, modes standard et Fidélité+ | `scr` e4 : classe `W2007` à 0 |
| 3 | Course `$2002` / vblank identique pour les quatre alignements | PPU, lecture `$2002` | `vblA`, `vblC` sur e0, e4, e6 |
| 4 | Incrément parasite de Y grossier sur `$2001` en famille e0 | PPU, `$2001`, alignements 0 et 1 | `scr` e0 : classe `W2001` à 0 |
| 5 | Phase de l'option `netlist_get_put` | `nes_core.rs` ligne 1050 | `dmcdma` sans `--dmc` |

Avant de corriger 3 et 4, je conseille de rejouer les programmes sur la variante de banc `macros_m2.cpp` : si les tableaux changent, l'écart dépend du modèle de /CS et pas des puces. Pour 1, une ROM de test sur console trancherait ; la netlist est ici de la logique synchrone, je la crois.

## Reproductions

Depuis `ext-tools\fidelity-audit\`. `H` = `..\..\wt-trial\rust\target\release\zax-harness.exe`, `D` = `..\visual2c07-tools\zax-harness-c3c-fin.exe`, `CORE` = chemin absolu de `ext-tools\visualnes-src\Core` (le banc y fait un `chdir`, donner aussi des chemins absolus pour le programme et la sortie).

```
:: programmes et banc
set VBL_DELAY=27128& set VBLC_DELAY=29384& set HIT_DELAY=8845
python -I mkprogs.py progs
set NETOAM=1& python -I mkprogs.py progs      (ajoute oamn et hitn ; refaire ensuite sans NETOAM)
buildaud.bat                                   (MSVC 2022, produit benchaud.exe)

:: netlist : benchaud <CORE> <prog.bin> <sortie> <cycles> <extra reset> [tuile] [oam]
benchaud CORE progs\cpuall.bin  net\cpuall-e0.txt  36000  0
benchaud CORE progs\cpuflow.bin net\cpuflow-e0.txt 42000  0
benchaud CORE progs\dmcdma.bin  net\dmcdma-e0.txt  36000  0
benchaud CORE progs\apuL5.bin   net\apuL5-e0.txt   126000 0      (apuL4, apuF : 246000 ; apuG : 124000)
benchaud CORE progs\vblA.bin    net\vblA-e0.txt    450000 0      (puis extra 2, 4, 6 ; idem vblC)
benchaud CORE progs\oamn.bin    net\oam-e0.txt     130000 0 0 1  (puis extra 4)
benchaud CORE progs\hitn.bin    net\hit-e0.txt     460000 0 1 2
..\visualnes-bench\benchdot2 CORE progs\scr.bin net\scr-e0.txt 4400000 0 700000   (puis extra 4)

:: Zaxnes
H cpu-bus-trace progs\cpuall.bin  --cycles 36000 --out zax\cpuall.txt
H cpu-bus-trace progs\cpuflow.bin --cycles 42000 --dmc 1014,8 --out zax\cpuflow-n.txt
H cpu-bus-trace progs\dmcdma.bin  --cycles 36000 --dmc 1014,8 --out zax\dmcdma-n.txt
H cpu-bus-trace progs\apuL5.bin   --cycles 126000 --dmc 1014,8 --out zax\apuL5-n.txt
H cpu-bus-trace progs\vblA.bin    --cycles 449000 --phase -9 --out zax\vblA-p-9.txt
D dot-trace progs\scr.nes --frames 1-8 --alignment 3 --phase -7 --fidelity-plus --out zax\scr-a3f.txt
D dot-trace progs\scr.nes --frames 1-8 --alignment 1 --phase -9 --fidelity-plus --out zax\scr-a1f.txt

:: comparaisons
python -I cmpcpu.py zax\cpuall.txt net\cpuall-e0.txt
python -I whichop.py zax\cpuall.txt net\cpuall-e0.txt progs\cpuall.map
python -I cmpcpu.py zax\cpuflow-n.txt net\cpuflow-e0.txt
python -I irqlat.py zax\cpuflow-n.txt net\cpuflow-e0.txt
python -I irqchk.py zax\cpuflow-n.txt net\cpuflow-e0.txt
python -I side.py   zax\cpuflow-n.txt net\cpuflow-e0.txt 5700 5730
python -I cmpcpu.py zax\dmcdma-n.txt net\dmcdma-e0.txt --nodata 4016,4017
python -I cmpcpu.py zax\apuL5-n.txt net\apuL5-e0.txt
python -I aputrans.py
python -I tours.py 29781 net\vblA-e2.txt zax\vblA-p-9.txt
python -I latmap.py oam 2004 net\oam-e0.txt 129000 5        (LM_DETAIL=65-256 pour le détail)
python -I scrclass.py zax\scr-a3f.txt net\scr-e4.txt
python -I ..\visualnes-bench\dotcmp.py zax\scr-a3f.txt net\scr-e4.txt 45

:: ROMs de test
H test-roms ..\..\cleanroom\assets-free\test-roms testroms-zaxnes.json --no-shots --markdown testroms-zaxnes.md
set ZAXNES_BIZHAWK_GAMEDB=<chemin absolu de ext-tools\bizhawk-src\Assets\gamedb>
H test-roms ..\..\cleanroom\assets-free\test-roms testroms-neshawk.json --no-shots --engine neshawk --markdown testroms-neshawk.md
H neshawk-tiles ..\..\demo\roms games-nh.jsonl
```

## Fichiers

- Outils : `asm6.py`, `mkprogs.py`, `mko4t.py`, `benchaud.cpp`, `buildaud.bat`, `benchaud.exe`, `cmpcpu.py`, `whichop.py`, `irqlat.py`, `irqchk.py`, `side.py`, `dmalist.py`, `pcmwatch.py`, `aputrans.py`, `tours.py`, `tourscount.py`, `vblev.py`, `vbltab.py`, `latmap.py`, `scrclass.py`, `dotside.py`.
- Programmes : `progs\*.bin`, `progs\*.nes`, `progs\cpuall.map`.
- Sorties des comparaisons : `results\*.txt`.
- Traces gardées (moins de 6 Mo) : `net\cpuall-e0.txt`, `cpuflow-e0.txt`, `dmcdma-e0.txt`, `apuL5-e0.txt`, `apuG-e0.txt`, `oam-e0.txt`, `oam-e4.txt`, `o4t.txt`, `dmc-probe.txt`, et les traces Zaxnes de `zax\`. Les traces `vbl*`, `hit*`, `scr*`, `apuL4`, `apuF` (10 à 20 Mo chacune) ont été supprimées ; les commandes ci-dessus les refont en 3 à 8 minutes chacune.
- Campagnes : `testroms-zaxnes.*`, `testroms-neshawk.*`.
- Journal : `PROGRESS.md`.

## Suites données (ajout du 17/11, après l'audit)

Ce rapport décrit la version auditée (`79aeba0b`). Les écarts ont ensuite été repris un par un sur `feat/c3c-collision` ; le détail est dans `wt-trial\docs\review\FIDELITY-LEADS.md`.

| Écart | Suite | Commit / entrée |
|---|---|---|
| 1. IRQ du DMC | Modèle netlist codé puis annulé : il fait échouer le test B d'`Interrupt flag latency` d'AccuracyCoin, vérifié sur console | I5 |
| 2. Écriture `$2007` en plein rendu | **Corrigé** : incrément au point D+6, comme la lecture. `scr` e4 : classe `W2007` 136 → 0 ; e0 : 77 → 0 | `e17c2df8`, I6 |
| (trouvé en route) `$2007` sur le point 257, `$2000` sur le point 261:304 | **Corrigé** : `v` et `t` gardent le ET de leurs bits horizontaux ; `$2000` atteint la dernière copie verticale. **`scr` e4, Fidélité+, alignement 3 : 0 point différent sur 4 trames** | `4b81177c`, I7 |
| 3. Course `$2002` / vblank | Rejouée sur un second banc (/CS tiré de M2, `benchaudm2`) : même tableau. Le modèle netlist fait échouer `ppu_vbl_nmi`, `vbl_nmi_timing`, `cpu_interrupts` et AccuracyCoin en alignements 1 et 2 : rejeté | I8 |
| 4. `$2001` en alignements 0/1 | Banc m2 identique ; course de simulation déjà identifiée (étape 33). Modèle « ET de `v` avec l'adresse du pipeline » consigné, non porté | I8 |
| 5. Lecture `$2007` en standard | Option C1 éteinte par défaut par décision du propriétaire ; Fidélité+ exact | C1 |
| 6. `netlist_get_put` | La phase d'allumage du DMC dépend de la durée du reset du banc, pas d'une valeur fixe de la puce | G5 |
| `oam` / `$2004` | Pointeur d'évaluation tracé (`benchspr`) : le banc sélectionne les sprites d'une ligne plus tôt ; question de banc, pas de Zaxnes | I9 |

Contrôles après chaque correction : test-ROMs (AccuracyCoin 147/147), références (seule Duelito change, attendu), 18 jeux et MMC5 identiques, corpus netlist 64 cas propres, callgrind sous +0,02 %.
