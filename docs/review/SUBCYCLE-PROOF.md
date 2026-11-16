# Zaxnes et le « sous-cycle » : dossier de preuve

*Version du 2026-11-15, branche `feat/fidplus-2` (commit `bf1c75f5` et suivants). Toutes les mesures
de ce document se refont avec les fichiers listés en partie 12 ; les traces brutes sont dans
[`subcycle-proof/traces.zip`](subcycle-proof/traces.zip).*

---

## 0. Résumé

On entend régulièrement qu'un émulateur NES « sous-cycle » (qui découpe chaque cycle du processeur
en demi-cycles, ou qui avance au rythme de l'horloge maître de la console) serait plus fidèle qu'un
émulateur « au cycle » comme Zaxnes. Ce document répond à cette affirmation **par la mesure, pas par
l'opinion**.

L'arbitre est le plus fort qui existe : **la simulation au transistor près des puces de la NES**
(le « netlist » du 2A03, qui contient le processeur 6502 et l'audio, et celui du 2C02, le processeur
graphique). Ces netlists ont été relevés sur des photographies de la puce réelle. On fait tourner les
mêmes programmes sur ce modèle physique et sur Zaxnes, et l'on compare **chaque cycle**.

Résultats :

| Ce qui est comparé | Volume | Écarts de timing | Écarts d'octet |
|---|---|---|---|
| Bus du processeur (adresse, lecture/écriture, donnée), 18 programmes ciblés sur les cas les plus durs connus | **421 901 cycles** | **0** | 3 426 cycles, tous sur des registres que le simulateur ne sait pas juger (parties 5.3 et 9) |
| dont cycles volés par les DMA (OAM et DMC, y compris leurs collisions) | **32 131 cycles** | **0** | **0** |
| PPU (adresses internes `v`/`t` point par point), corpus FIDPLUS-A, mode Fidélité+ | 3 trames par programme, 4 alignements | **0 à 2 points** par programme hors artefacts de démarrage du banc (partie 6) | — |
| AccuracyCoin (147 tests de précision vérifiés sur console réelle) | 4 alignements CPU/PPU | 147 / 147 dans les 4 | — |
| Campagne de ROM de test (blargg, holy-mapperel, etc.) avec le modèle de console de chaque ROM | 219 ROM jugeables | 0 échec | — |

**Conclusion.** Sur tout ce que le modèle physique de la puce permet de juger, Zaxnes produit
**exactement** le même comportement observable, cycle par cycle. Un émulateur sous-cycle ne peut pas
faire mieux qu'« identique » sur ces observations : un découpage plus fin à l'intérieur de
l'émulateur ne change rien à ce que les autres puces voient, et c'est ce qu'elles voient qui fait le
jeu. Là où le netlist a montré qu'un effet tombe *au milieu* d'un cycle (les écritures du processeur
dans les registres du PPU), Zaxnes le modélise déjà, au demi-cycle d'horloge maître, dans son option
Fidélité+ (partie 6). Les seuls écarts restants viennent soit **d'états de mise sous tension**
(aléatoires sur une vraie console, partie 7), soit **des limites du simulateur lui-même** (partie 9),
et aucun d'eux ne dépend de la granularité de l'émulation.

---

## 1. De quoi parle-t-on ? (version pour non-spécialiste)

### 1.1 Les horloges de la NES

La NES (version américaine et japonaise, NTSC) a un seul quartz à **21,477 MHz** : l'**horloge
maître**. Tout le reste en dérive :

* le **processeur** (CPU, dans la puce 2A03) divise par 12 : **1,79 MHz**, un « cycle CPU » dure
  12 tops d'horloge maître ;
* le **processeur graphique** (PPU, puce 2C02) divise par 4 : un « point » (*dot*) dure 4 tops, il y
  a donc **3 points par cycle CPU** ;
* le son (APU) est dans la même puce que le CPU et suit son horloge.

Pendant un cycle CPU, le processeur fait **exactement un accès au bus** : il lit ou écrit un octet à
une adresse. Le bus est le seul moyen pour le CPU de parler au reste de la machine (mémoire,
cartouche, PPU, manettes).

À l'intérieur d'un cycle, le 6502 a deux phases (φ1 et φ2) : il présente l'adresse pendant la
première, la donnée circule pendant la seconde. Le signal **M2** que voit la cartouche suit ces
phases.

### 1.2 « Au cycle » contre « sous-cycle »

* Un émulateur **au cycle** (Zaxnes en mode standard) fait avancer la machine d'un cycle CPU à la
  fois : un accès au bus, puis le PPU rattrape ses 3 points, puis l'APU son cycle.
* Un émulateur **sous-cycle** découpe ce cycle : φ1 puis φ2, ou carrément chaque top d'horloge
  maître, en entrelaçant le CPU et le PPU à cette échelle. C'est par exemple l'architecture de
  BeesNES (lue dans ses sources, licence MIT, `ext-tools/bees-src`) : le CPU y est coupé en deux
  mises à jour, `Tick` (φ1) et `TickPhi2` (φ2), sur un anneau d'horloges.

L'argument habituel est : « plus fin = plus fidèle ». Il est **faux en général** : un modèle plus fin
n'est plus fidèle que s'il produit un comportement **observable** différent, et plus proche du réel.
Si deux émulateurs produisent exactement la même suite d'accès au bus, les mêmes pixels et les mêmes
échantillons sonores, ils sont aussi fidèles l'un que l'autre, quelle que soit leur cuisine interne.

### 1.3 Une image

Deux horlogers reproduisent une montre. Le premier modélise chaque engrenage, le second seulement ce
que fait chaque aiguille à chaque seconde. Si, posées côte à côte avec l'original pendant des
heures, les trois montres affichent la même heure à chaque seconde, aucune n'est « plus juste » :
la seule question qui compte est « affiche-t-elle la même chose que l'original ? ». Ce document pose
cette question, seconde par seconde (cycle par cycle), à l'original lui-même.

---

## 2. Ce qu'il faut prouver

Le critère est le suivant : **pour tout programme, la suite des événements observables est-elle
identique à celle de la vraie puce ?** Les événements observables sont :

1. **le bus du CPU, à chaque cycle** : adresse, sens (lecture ou écriture), octet. C'est tout ce que
   le CPU montre au monde : la cartouche, la mémoire, le PPU et les manettes ne voient que ça ;
2. **ce que le PPU fait de ce qu'il reçoit** : ses registres internes et ses pixels, point par point ;
3. les interruptions (NMI, IRQ) et les DMA, qui se voient eux aussi sur le bus (cycles volés,
   adresses lues).

Si (1) est identique à chaque cycle, alors tout ce qui dépend du CPU est identique : un jeu ne peut
rien faire qui ne passe par le bus. Un modèle interne plus fin ne peut changer le résultat que s'il
change au moins **un** cycle de (1) ou **un** point de (2). La preuve consiste donc à comparer (1) et
(2), exhaustivement, sur les cas où une différence aurait le plus de chances d'apparaître.

---

## 3. L'arbitre : le netlist des puces

### 3.1 Ce que c'est

L'équipe Visual 6502 a photographié au microscope les puces de la NES, couche par couche, et en a
relevé chaque transistor et chaque connexion : c'est le **netlist**. Un simulateur de commutation
(« switch-level ») fait ensuite tourner ce circuit, demi-cycle d'horloge par demi-cycle d'horloge.
**Ce n'est pas un émulateur écrit par quelqu'un qui a interprété une documentation : c'est le circuit
lui-même.** Ici on utilise VisualNes (2A03 et 2C02 réunis), dont le code ne sert qu'à mesurer : il
reste hors du dépôt (`ext-tools/visualnes-bench`).

Le netlist simule **plus fin que n'importe quel émulateur sous-cycle** : il ne découpe pas le cycle
en deux, il simule chaque nœud électrique à chaque demi-période d'horloge maître. Si Zaxnes
correspond au netlist, il correspond *a fortiori* à tout émulateur qui correspond lui-même au netlist.

### 3.2 Ses limites (et pourquoi elles ne gênent pas la démonstration)

* Il ne contient **que** les puces : pas de cartouche réelle, pas de manettes, pas de circuit
  analogique. Le banc fournit une mémoire programme ; tout le reste de l'espace `$2000`-`$7FFF` qui
  n'est pas une puce est laissé **flottant** (partie 9.1). C'est la source de presque tous les écarts
  d'octet du tableau de la partie 0.
* L'état de mise sous tension est celui que la simulation fixe à la fin de son *reset* ; sur une
  vraie console, plusieurs de ces états sont aléatoires (partie 7).
* Il simule une révision de puce (NTSC). Le PAL (2C07, 2A07) n'est pas couvert (partie 10).

---

## 4. Méthode

### 4.1 Les deux côtés

* **Netlist** : `ext-tools/visualnes-bench/benchcpu.cpp` (hors dépôt) fait tourner le 2A03 et le 2C02
  de VisualNes et écrit **une ligne par cycle CPU**, au front descendant de φ2, avec l'adresse, le
  sens et l'octet tenus pendant φ2 (c'est l'instant où la donnée est valide, celui où une mémoire la
  prend).
* **Zaxnes** : la commande `zax-harness cpu-bus-trace`, compilée avec la fonction de diagnostic
  `bus-trace`, enregistre chaque lecture et écriture du CPU **et chaque accès des unités de DMA**
  (cycle d'arrêt, cycle factice, cycle d'alignement, lecture d'échantillon DMC, lecture et écriture
  de l'OAM). Format identique : une ligne par cycle. Sans `bus-trace`, ce crochet est une fonction
  vide : il ne coûte rien au noyau normal.

### 4.2 Lire une ligne de trace

```
37 0200 R 00 D
```

Cycle 37 (compté depuis la première lecture d'instruction en `$8000`), adresse `$0200`, **R**ead
(lecture), octet `$00`, et `D` : c'est une unité de DMA qui tient le bus, pas le CPU. `I` signale un
cycle de DMA où la puce répète la lecture arrêtée sans que la manette voie une nouvelle lecture
(partie 5.2, cas `$4016`). Sans lettre : c'est le CPU.

### 4.3 Les programmes

Dix-huit programmes écrits pour ce diagnostic (`docs/progress/cpu-diag/mkprog.py`, `mkprog2.py`,
`mkdmc.py`, `mkprog3.py`), chargés au même endroit des deux côtés (32 Kio en `$8000`, RESET en
`$8000`, NMI et IRQ en `$9000`). Chacun vise une famille de cas où un émulateur au cycle serait,
selon l'argument « sous-cycle », le plus susceptible de se tromper :

| Programme | Ce qu'il exerce |
|---|---|
| `loop`, `modes`, `allops`, `branch` | toutes les instructions officielles, tous les modes d'adressage, franchissements de page, branchements pris / non pris / qui changent de page, lecture-modification-écriture, pile |
| `illegal` | toutes les instructions « illégales » stables, dans tous leurs modes |
| `nmi`, `irq` | entrée d'interruption au moment de la vblank, IRQ du compteur de trames, `RTI` |
| `intbranch` | IRQ et NMI qui tombent pendant des branchements pris (le délai d'interruption des branchements, piège classique) |
| `fc5` | compteur de trames de l'APU en mode 5 pas puis 4 pas avec IRQ, écrits sur les deux parités |
| `dma`, `oam0`, `oam1` | DMA OAM (`$4014`) démarré sur l'une et l'autre parité de cycle : 513 ou 514 cycles |
| `dmc`, `dmc0`, `dmc1` | DMA du canal DMC en boucle, sous un mélange de lectures, d'écritures et de lecture-modification-écriture ; chargement démarré sur les deux parités |
| `dmcoam` | **collisions DMC / OAM** : un DMA OAM à chaque boucle pendant que le DMC lit ses échantillons (le cas le plus dur connu du bus de la NES) |
| `dmc2007` | lectures de `$2007` (données PPU) interrompues par le DMC (la lecture est répétée : effet de bord visible) |
| `dmc4016` | lectures de manettes interrompues par le DMC (la fameuse double lecture de `$4016`) |

Plus deux programmes de contrôle pour la question des bits flottants de la manette (`g6jmp`,
`g6brk`, `mkg6.py`, partie 9.1).

### 4.4 La comparaison

`cmp3.py <netlist> <zaxnes>` compare chaque cycle et sépare :

* les **écarts de timing** : une autre adresse ou un autre sens au même cycle. C'est ce que
  changerait une meilleure ou une moins bonne gestion du temps ;
* les **écarts d'octet** : même adresse, même sens, octet différent. Ce ne sont pas des questions
  de timing : ce sont des valeurs (bus flottant, état de mise sous tension).

### 4.5 Les réglages de mise sous tension

Trois valeurs ne sont pas fixées par le circuit mais par l'instant où la console démarre. Elles sont
réglées **une seule fois, identiques pour les 18 programmes** (aucun réglage par programme) :

| Réglage | Valeur | Ce que c'est |
|---|---|---|
| `--alignment 3 --phase -8` | alignement CPU/PPU 3, phase -8 points | où en est le PPU quand le CPU démarre |
| `--flip-get-put` | parité « get/put » du netlist | la parité de cycle du CPU par rapport à l'APU après reset |
| `--dmc 431,8` | minuterie DMC 431, 8 bits | la position du compteur interne du canal DMC à la mise sous tension |

La partie 7 montre que ce sont bien des états de départ et non des différences de modèle, et que
chacun est **unique** : une seule valeur aligne tout, pas une famille de valeurs ajustées.

---

## 5. Résultats côté CPU

### 5.1 Le tableau complet

Mesuré le 2026-11-15 (`final5.sh`), harness compilé avec `bus-trace` :

| Programme | Cycles | dont CPU | dont DMA | Écarts de timing | Écarts d'octet |
|---|---|---|---|---|---|
| `loop` | 3 990 | 3 990 | 0 | 0 | 0 |
| `modes` | 3 990 | 3 990 | 0 | 0 | 0 |
| `allops` | 3 990 | 3 990 | 0 | 0 | 0 |
| `branch` | 3 990 | 3 990 | 0 | 0 | 0 |
| `dma` | 3 999 | 3 486 | 513 | 0 | 0 |
| `oam0` | 3 999 | 3 486 | 513 | 0 | 0 |
| `oam1` | 3 999 | 3 485 | 514 | 0 | 0 |
| `nmi` | 29 990 | 29 990 | 0 | 0 | 0 |
| `irq` | 59 990 | 59 990 | 0 | 0 | 0 |
| `dmc` | 29 999 | 29 727 | 272 | 0 | 0 |
| `dmc0` | 29 999 | 29 725 | 274 | 0 | 0 |
| `dmc1` | 29 999 | 29 726 | 273 | 0 | 0 |
| `dmcoam` | 29 999 | 762 | 29 237 | 0 | 0 |
| `dmc2007` | 29 999 | 29 732 | 267 | 0 | 0 |
| `illegal` | 3 990 | 3 990 | 0 | 0 | 0 |
| `dmc4016` | 29 999 | 29 731 | 268 (dont 22 « idle ») | 0 | 624 (partie 9.1) |
| `intbranch` | 59 990 | 59 990 | 0 | 0 | 10 (partie 9.2 et 9.3) |
| `fc5` | 59 990 | 59 990 | 0 | 0 | 2 792 (partie 9.3) |
| **Total** | **421 901** | **389 770** | **32 131** | **0** | **3 426** |

**Pas un seul cycle, sur 421 901, n'a une autre adresse ou un autre sens que la puce réelle.** Ni
dans les instructions, ni à l'entrée des interruptions, ni dans les 32 131 cycles volés par les DMA,
y compris quand le DMC et l'OAM se battent pour le bus.

Les 3 426 écarts d'octet portent **tous** sur des lectures de `$2002`, `$4015`, `$4016` et `$4017`
(et sur les écritures qui recopient ces valeurs) ; la partie 9 les explique un par un. Aucun ne
change le moment d'un accès.

### 5.2 Traces commentées

#### a) Entrée d'une NMI (programme `nmi`, cycles 27 380 à 27 400)

La vblank lève la NMI pendant une boucle `INX / JMP`. Le CPU finit son instruction, fait deux
lectures factices, empile PC et P, lit le vecteur `$FFFA`, saute en `$9000` où se trouve un `RTI`.

| Cycle | 2A03 (netlist) | Zaxnes | |
|---|---|---|---|
| 27380 | `800B R 4C` | `800B R 4C` | = |
| 27381 | `800C R 0A` | `800C R 0A` | = |
| 27382 | `800D R 80` | `800D R 80` | = |
| 27383 | `800A R E8` | `800A R E8` | = |
| 27384 | `800B R 4C` | `800B R 4C` | = |
| 27385 | `800B R 4C` | `800B R 4C` | = |
| 27386 | `800C R 0A` | `800C R 0A` | = |
| 27387 | `800D R 80` | `800D R 80` | = |
| 27388 | `800A R E8` | `800A R E8` | = |
| 27389 | `800A R E8` | `800A R E8` | = |
| 27390 | `01FF W 80` | `01FF W 80` | = |
| 27391 | `01FE W 0A` | `01FE W 0A` | = |
| 27392 | `01FD W 24` | `01FD W 24` | = |
| 27393 | `FFFA R 00` | `FFFA R 00` | = |
| 27394 | `FFFB R 90` | `FFFB R 90` | = |
| 27395 | `9000 R 40` | `9000 R 40` | = |
| 27396 | `9001 R 00` | `9001 R 00` | = |
| 27397 | `01FC R 00` | `01FC R 00` | = |
| 27398 | `01FD R 24` | `01FD R 24` | = |
| 27399 | `01FE R 0A` | `01FE R 0A` | = |
| 27400 | `01FF R 80` | `01FF R 80` | = |

L'instant exact où la NMI est prise est le point que les partisans du sous-cycle citent le plus :
le signal NMI est échantillonné pendant une phase précise du cycle. Zaxnes prend la NMI **au même
cycle** que la puce, empile les mêmes octets (`$24` = registre d'état avec le bit B à 0) au même
cycle et reprend au même cycle.

#### b) Arrêt du CPU par le DMC (programme `dmc0`, cycles 30 à 42)

L'écriture `$4015 = $10` (cycle 33) démarre le DMC. Quelques cycles plus tard, le DMC doit charger son
premier octet : il **arrête** le CPU sur une lecture, répète cette lecture, puis lit l'échantillon en
`$C000`.

| Cycle | 2A03 (netlist) | Zaxnes | Qui | |
|---|---|---|---|---|
| 30 | `8019 R 8D` | `8019 R 8D` | CPU | = |
| 31 | `801A R 15` | `801A R 15` | CPU | = |
| 32 | `801B R 40` | `801B R 40` | CPU | = |
| 33 | `4015 W 10` | `4015 W 10` | CPU | = |
| 34 | `801C R AD` | `801C R AD` | CPU | = |
| 35 | `801D R 00` | `801D R 00` | CPU | = |
| 36 | `801E R 02` | `801E R 02` | CPU | = |
| 37 | `0200 R 00` | `0200 R 00` | DMA (arrêt) | = |
| 38 | `0200 R 00` | `0200 R 00` | DMA (factice) | = |
| 39 | `C000 R 00` | `C000 R 00` | DMA (lecture DMC) | = |
| 40 | `0200 R 00` | `0200 R 00` | CPU | = |
| 41 | `801F R 8D` | `801F R 8D` | CPU | = |
| 42 | `8020 R 01` | `8020 R 01` | CPU | = |

Tout y est : le cycle d'arrêt tombe sur la lecture de donnée `$0200` (pas sur la lecture d'opérande
qui précède), la lecture est **répétée** deux fois sur le bus (ce qui importe quand l'adresse a un
effet de bord, comme `$2007` ou `$4016`), l'échantillon est lu au cycle 39, et le CPU refait sa
lecture au cycle 40.

#### c) Le DMC qui s'invite au milieu d'un DMA OAM (programme `dmcoam`, cycles 804 à 820)

C'est le cas le plus difficile du bus de la NES : deux unités de DMA veulent le bus en même temps.
Le DMA OAM alterne une lecture en RAM (`$02xx`) et une écriture dans `$2004` ; le DMC vole une de
ses lectures.

| Cycle | 2A03 (netlist) | Zaxnes | Qui | |
|---|---|---|---|---|
| 804 | `2004 W 00` | `2004 W 00` | DMA OAM (écriture) | = |
| 805 | `0278 R 00` | `0278 R 00` | DMA OAM (lecture) | = |
| 806 | `2004 W 00` | `2004 W 00` | DMA OAM | = |
| 807 | `0279 R 00` | `0279 R 00` | DMA OAM | = |
| 808 | `2004 W 00` | `2004 W 00` | DMA OAM | = |
| 809 | `027A R 00` | `027A R 00` | DMA OAM | = |
| 810 | `2004 W 00` | `2004 W 00` | DMA OAM | = |
| 811 | `027B R 00` | `027B R 00` | DMA OAM | = |
| 812 | `2004 W 00` | `2004 W 00` | DMA OAM | = |
| 813 | `C000 R 00` | `C000 R 00` | **DMA DMC** (vole la lecture) | = |
| 814 | `801C R E8` | `801C R E8` | cycle d'alignement : le bus répète la lecture arrêtée du CPU | = |
| 815 | `027C R 00` | `027C R 00` | DMA OAM reprend | = |
| 816 | `2004 W 00` | `2004 W 00` | DMA OAM | = |
| 817 | `027D R 00` | `027D R 00` | DMA OAM | = |
| 818 | `2004 W 00` | `2004 W 00` | DMA OAM | = |
| 819 | `027E R 00` | `027E R 00` | DMA OAM | = |
| 820 | `2004 W 00` | `2004 W 00` | DMA OAM | = |

Le DMC prend le cycle 813, le DMA OAM perd sa parité et doit attendre un cycle d'alignement
(814), pendant lequel la puce remet sur le bus l'adresse où le CPU s'était arrêté (`$801C`). Zaxnes
fait **exactement** la même chose, cycle par cycle, adresse par adresse. Ce programme est presque
entièrement fait de DMA (29 237 cycles de DMA sur 29 999) : **0 écart**.

#### d) Le DMC qui interrompt une lecture de manette (programme `dmc4016`, cycles 1 238 à 1 252)

| Cycle | 2A03 (netlist) | Zaxnes | Qui | |
|---|---|---|---|---|
| 1238 | `0301 W 40` | `0301 W 40` | CPU | = |
| 1239 | `802D R AD` | `802D R AD` | CPU | = |
| 1240 | `802E R 16` | `802E R 16` | CPU | = |
| 1241 | `802F R 40` | `802F R 40` | CPU | = |
| 1242 | `4016 R 40` | `4016 R 40` | DMA (arrêt sur la lecture de manette) | = |
| 1243 | `4016 R 40` | `4016 R --` | DMA (répétition invisible pour la manette) | = |
| 1244 | `4016 R 40` | `4016 R --` | DMA (idem) | = |
| 1245 | `C000 R 00` | `C000 R 00` | DMA DMC | = |
| 1246 | `4016 R 00` | `4016 R 40` | CPU | octet seulement |
| 1247 | `8030 R 8D` | `8030 R 8D` | CPU | = |

Le timing est identique, y compris le détail qui casse les manettes dans certains jeux : pendant
l'arrêt, `/OE` reste actif et la manette ne voit **qu'une** lecture, pas trois (Zaxnes marque les
deux répétitions `--`, l'adresse est sur le bus mais le registre à décalage n'avance pas). Seul
l'octet du cycle 1246 diffère, et c'est une question de bus flottant que le simulateur ne sait pas
trancher (partie 9.1).

---

## 6. Côté PPU : là où le demi-cycle compte, Zaxnes l'a déjà

Le différentiel du PPU a été fait avant celui du CPU (lots FIDPLUS-2 et FIDPLUS-A,
`docs/progress/FIDPLUS-2.md` et `docs/progress/FIDPLUS-A.md`). Le juge (`fidjudge.sh`, hors dépôt)
compare **point par point** les adresses internes `v` et `t` du PPU (celles qui décident de chaque
tuile affichée) entre le 2C02 du netlist et Zaxnes, sur trois trames entières, dans les alignements
CPU/PPU possibles.

Ce différentiel a trouvé **le seul endroit où le demi-cycle a un effet mesurable** : quand le CPU écrit
dans un registre du PPU (`$2001`, `$2005`, `$2006`, `$2007`), l'effet arrive dans le PPU un nombre de
demi-tops d'horloge maître après le front de M2, et selon l'alignement de la console il tombe un point
plus tôt ou plus tard. Zaxnes Fidélité+ (partie A) modélise précisément cela : chaque écriture devient
un **front** daté en demi-tops d'horloge maître (`ppu/edges.rs`, une latence par signal, mesurée sur
le netlist). C'est du sous-cycle, mais **uniquement là où la mesure l'exige**, et pour un coût nul
quand rien n'est en attente.

Résultat (corpus sans la première trame, alignements 2 et 3, points dont `v` ou `t` diffère) :

| Programme | Zaxnes standard | Zaxnes Fidélité+ |
|---|---|---|
| `g2006px` | 740 | **0** |
| `g2006x` | 8 850 | 2 (une écriture `$2006` sur la copie verticale du point 304) |
| `g2001px`, `g3e01px` | 0 | **0** |
| `g2005px`, `g2000` | 2 | 2 |
| `oam` | 1 | 1 |
| `r2007` | 168 152 | 78 806 (lignes après l'image : le PPUCTRL du netlist démarre avec le bit 2 à 1, le programme ne l'écrit jamais) |
| `g2005` | 57 399 | 57 399 (même artefact de démarrage) |

Les alignements 0 et 1 ont été jugés aussi (`g2006px` 0 point en Fidélité+, contre 740 en
standard). Les écarts des lignes `r2007` et `g2005` ne sont pas des erreurs du PPU de Zaxnes : le
netlist démarre son registre de contrôle avec un incrément de 32 au lieu de 1, ce qu'aucun des deux
programmes ne corrige ; tout le reste de la trame est identique.

Restent 1 à 2 points sur `g2006x`, `g2005px`, `g2000` et `oam` (sur trois trames de 89 342 points
chacune). Ils sont **dans** le modèle au demi-top d'horloge maître : ce ne sont donc pas des questions
de granularité, mais le détail de quelques signaux individuels, encore à l'étude
(`docs/review/FIDELITY-LEADS.md`, A5 et A7).

**Pourquoi cela compte pour le débat** : un émulateur « sous-cycle » qui découperait tout en demi-
cycles ne pourrait pas faire mieux que **0 point d'écart** sur `g2006px`, `g2001px` et `g3e01px`.
Zaxnes Fidélité+ y est déjà, et à 1 ou 2 points près sur les autres.
Et en mode standard, ces écarts de point ne changent **aucune image** du corpus de jeux (le
« golden » de Zaxnes est octet pour octet identique entre standard et Fidélité+, voir
`FIDPLUS-A.md`, « Final validation ») : ils ne se voient que dans une trace interne du PPU.

---

## 7. Les réglages de démarrage ne sont pas des « rustines »

Un sceptique dira : « vous avez réglé des paramètres jusqu'à ce que ça colle ». Voici pourquoi
c'est faux.

### 7.1 Ce sont des états que la vraie console tire au hasard

* **L'alignement et la phase CPU/PPU** : à chaque mise sous tension, le CPU et le PPU démarrent à un
  point quelconque l'un par rapport à l'autre. C'est documenté sur nesdev et
  c'est pour ça qu'AccuracyCoin se lance dans les quatre alignements. Zaxnes propose d'ailleurs
  « Aléatoire » comme choix (`emu.cpu_ppu_alignment`).
* **La parité get/put CPU/APU** : le DMA lit sur un cycle « get » et écrit sur un cycle « put »,
  selon une parité qui dépend du démarrage. D'une console à l'autre, un même DMA OAM dure 513 ou 514
  cycles selon le cycle où il est lancé : c'est cette parité.
* **La minuterie du DMC** : son compteur interne (un registre à décalage à rétroaction de 9 bits,
  `cpu_pcm_t0-8` dans le netlist) n'a pas de valeur définie à la mise sous tension.

### 7.2 Chacun ne change qu'une chose, et une seule valeur convient

Mesuré avec `sweep.sh` :

**La phase PPU** (programme `nmi`, alignement 3) :

| Phase | -20 | -16 | -13 | **-12 à -4** | -3 | 0 | 4 |
|---|---|---|---|---|---|---|---|
| Écarts de timing | 17 | 14 | 14 | **0** | 15 | 15 | 17 |

Hors de la fenêtre, **seule** l'entrée de la NMI se déplace d'une instruction (une quinzaine de
cycles décalés, puis tout redevient identique) : c'est exactement ce qui arrive sur deux consoles
allumées à des instants différents. Exemple sans aucun réglage :

| Cycle | 2A03 (netlist) | Zaxnes (démarrage par défaut) |
|---|---|---|
| 27385 | `800B R 4C` | `800B R 4C` |
| 27386 | `800C R 0A` | `800B R 4C` |
| 27387 | `800D R 80` | `01FF W 80` |
| 27388 | `800A R E8` | `01FE W 0B` |
| 27389 | `800A R E8` | `01FD W 24` |
| 27390 | `01FF W 80` | `FFFA R 00` |

Zaxnes prend la NMI une instruction plus tôt (il empile `$800B` au lieu de `$800A`) parce que
son PPU a démarré un peu plus tôt : la séquence d'entrée elle-même est identique, décalée.

**La parité get/put** (sans `--flip-get-put`) : `oam0` passe de 0 à 2 775 cycles décalés, car le DMA
OAM dure 514 cycles au lieu de 513 et tout le reste glisse d'un cycle ; `dmc0` montre l'arrêt du DMC
un cycle plus tôt :

| Cycle | 2A03 (netlist) | Zaxnes (autre parité) |
|---|---|---|
| 36 | `801E R 02` | `801E R 02` (arrêt ici) |
| 37 | `0200 R 00` (arrêt ici) | `801E R 02` |
| 38 | `0200 R 00` | `C000 R 00` |
| 39 | `C000 R 00` | `801E R 02` |
| 40 | `0200 R 00` | `0200 R 00` |

Même nombre de cycles volés, même retour au cycle 40 : seule la parité de départ change. Les deux
comportements existent sur de vraies consoles.

**La minuterie DMC** : un balayage de la minuterie (380 à 470) et du compteur de bits (1 à 8) sur
`dmc`, `dmc0` et `dmc1` ensemble trouve **une seule** valeur, 431 et 8 bits, qui rend les trois
identiques (`CPU-NETLIST-DIAG.md`, étape 4). Sans elle, les lectures d'échantillon arrivent à une
autre position dans la même période de 432 cycles.

### 7.3 Les mêmes réglages pour tout

Les trois valeurs ont été trouvées sur les premiers programmes, puis **appliquées telles quelles**
aux programmes difficiles écrits ensuite (`dmcoam`, `dmc2007`, `illegal`, `dmc4016`, `intbranch`,
`fc5`) : ceux-ci ont donné 0 écart de timing du premier coup. Un ajustement « pour faire coller »
n'aurait pas survécu à des programmes qu'il n'avait jamais vus.

Et surtout : **un émulateur sous-cycle a exactement le même problème**. Il doit lui aussi choisir
une phase CPU/PPU, une parité get/put et une minuterie DMC au démarrage. Découper le cycle en deux
ne supprime aucun de ces trois choix.

---

## 8. Les ROM de test vérifiées sur console

Le netlist juge le bus ; les ROM de test jugent les résultats, et beaucoup ont été vérifiées sur
du matériel réel.

* **AccuracyCoin** (147 tests, dont les DMA, les lectures factices, le bus ouvert, les interruptions,
  le PPU au point près) : **147 / 147 dans les quatre alignements CPU/PPU**, en mode standard comme
  en Fidélité+.
* **Campagne de ROM de test** (blargg, holy-mapperel, mmc3_test, etc.) : 215 réussies, 4 échouées en
  configuration par défaut. Ces 4 échecs sont **voulus** : trois testent la révision A du MMC3
  (incompatible avec la révision B, que la plupart des cartouches utilisent, et les auteurs des
  suites écrivent eux-mêmes qu'un émulateur ne peut pas réussir les deux à la fois), le dernier compare
  la palette de démarrage à « probablement celle de ma NES » (dixit son auteur). **En lançant chaque
  ROM avec le modèle de console pour lequel elle a été écrite (`ZAXNES_CONSOLE_MODEL=rom`) : 219
  réussies, 0 échec** (`docs/design/CONSOLE-MODEL.md`).

---

## 9. Les écarts d'octet, un par un

Les 3 426 cycles de la partie 5.1 dont seul l'octet diffère se rangent en trois familles. Aucune
n'est un écart de timing, et aucune ne serait corrigée par un découpage en demi-cycles.

### 9.1 `$4016` / `$4017` : le simulateur n'a pas de manette (630 cycles)

Dans VisualNes, toute lecture entre `$2000` et `$7FFF` qui n'est pas une puce **laisse le bus
externe flotter** (`Core/macros.cpp`, `mCpuRead` puis `floatBits("cpu_db")` ; le code porte la
mention « TODO: proper open bus implementation »). Il n'y a pas de manettes, pas de tampon 74HC368 :
les bits lus en `$4016` et `$4017` sont ce que le simulateur de commutation fait d'un bus
entièrement flottant. C'est pourquoi le netlist lit `$10` en `$4017` (le bit 4 levé sans raison
électrique) là où Zaxnes rend le bus ouvert attendu (`$40`) : 606 écritures `$0303` recopient cette
valeur, plus 4 lectures `$4017` juste après une lecture DMC.

Pour les 7 lectures `$4016` juste après une lecture DMC (et les 7 écritures qui les recopient), on a
voulu savoir qui a raison. Programme de contrôle `g6jmp` : on exécute du code **depuis `$4015`**,
l'opcode vient du registre d'état de l'APU (`$10`, un `BPL`), son opérande de `$4016`. La cible du
branchement révèle l'octet que le CPU a reçu :

| Cycle | 2A03 (netlist) | Zaxnes |
|---|---|---|
| 42 | `4015 R` (opcode `$10`) | `4015 R 10` |
| 43 | `4016 R` → opérande `$40` | `4016 R 00` → opérande `$00` |
| 45 | saute en `$4057` | saute en `$4017` |

Le netlist prend les bits ouverts sur les **broches** (`$40`, l'octet de poids fort de l'opérande du
`JMP` resté sur le bus externe), Zaxnes sur le **bus interne** (`$10` de `$4015`, dont les bits 5 à 7
sont nuls). Or AccuracyCoin, **vérifié sur console**, a un test pour exactement cette situation
(« Implied Dummy Reads » : un `JSR` dont l'opcode est lu en `$4015` et l'opérande en `$4016`) et
attend la valeur du **bus interne** : le commentaire du test (`AccuracyCoin.asm`, routine
`ImpliedDummyRead_Bit5`) donne `[Read $4016: 21]` pour toutes les instructions implicites dont le
bit 5 est à 1. Pour un `NOP` (`$EA`), les broches tiennent encore `$EA` à ce moment et donneraient
`$E1` ; le bus interne tient `$20` (lu en `$4015`) et donne `$21`. Le `JSR` saute alors en `$0021`
ou en `$00E1` : le test distingue les deux règles, et c'est `$21` qui est juste sur console.
Zaxnes réussit ce test. **Sur ce point, la console réelle
contredit le netlist, et Zaxnes suit la console.** Le cas précis « juste après une lecture DMC »
n'a pas de test matériel connu ; Zaxnes y applique la même règle, cohérente avec le test vérifié
(piste G6 de `FIDELITY-LEADS.md`).

### 9.2 `$2002` bit 5 : l'OAM de démarrage (8 cycles)

Dans `intbranch`, le gestionnaire d'interruption lit `$2002` : le netlist a le bit 5 (débordement de
sprites) à 1, Zaxnes à 0. Ce bit dépend du contenu de la mémoire des sprites (OAM) à la mise sous
tension, que le netlist initialise à sa façon et qui est aléatoire sur une console (4 lectures et
les 4 écritures qui les recopient).

### 9.3 `$4015` : le registre qui ne sort pas sur les broches (2 794 cycles)

`$4015` est lu **à l'intérieur** du 2A03 et ne pilote pas les broches de données. Le banc imprime les
broches : il montre donc le bus ouvert (`$40`), alors que Zaxnes imprime ce que le CPU a reçu
(`$00` ou `$40`). Les programmes recopient chaque valeur lue en RAM, et **ces copies sont identiques
des deux côtés** : le CPU du netlist et celui de Zaxnes ont bien reçu le même octet. C'est un écart
d'affichage du banc, pas de comportement.

---

## 10. Ce que ce dossier ne prouve pas

Une preuve honnête dit où elle s'arrête.

* **Un corpus n'est pas l'ensemble des programmes possibles.** Les 18 programmes visent les cas
  difficiles connus (DMA, interruptions, branchements, instructions illégales, registres à effet de
  bord) ; les jeux et AccuracyCoin couvrent le reste par leurs résultats. Toute nouvelle situation
  peut être ajoutée au corpus en quelques minutes (partie 12) : c'est l'invitation à faire à
  quiconque affirme le contraire.
* **PAL et Dendy** : pas de netlist du 2C07 ni du 2A07 disponible. Le timing PAL de Zaxnes repose sur
  la documentation et les ROM de test, pas sur un transistor.
* **Les mappers** qui observent M2 (MMC5, FDS, prédiviseurs d'IRQ VRC) : aucune cartouche n'a de
  netlist. Ils sont jugés par leurs ROM de test.
* **L'analogique** : la sortie audio et vidéo n'est pas sur le bus ; elle relève d'autres options
  (profil audio de console, signal composite) et d'autres mesures.
* **PPU** : 1 à 2 points résiduels sur quatre programmes du corpus (partie 6), dans le modèle au
  demi-top déjà, donc sans rapport avec la granularité.
* **G5** : si la parité get/put est tirée au hasard à chaque démarrage (comme l'alignement CPU/PPU) ou
  fixée par le reset n'est pas tranché ici ; dans les deux cas c'est un état de départ (partie 7).
* **La révision du 2A03** : le netlist est celui d'une puce photographiée ; les différences connues
  entre lettres de révision sont traitées comme des choix de modèle de console (Fidélité+ C).

Aucun de ces points ne serait réglé par un émulateur sous-cycle : ce sont des questions de
documentation, de mesure ou de matériel, pas de découpage du temps.

---

## 11. Réponses courtes aux objections

**« Un émulateur sous-cycle est forcément plus précis. »**
Seulement s'il produit un comportement observable différent et plus juste. Sur 421 901 cycles
comparés au circuit réel, Zaxnes produit le même comportement : il n'y a plus rien à rattraper.

**« Les ROM de test ne voient pas tout. »**
D'accord, c'est pour ça qu'on ne s'est pas contenté d'elles : le netlist voit **tout** le bus, à
chaque cycle, sans qu'un programme ait besoin de mesurer quoi que ce soit.

**« Vous avez ajusté des paramètres. »**
Trois états de démarrage, aléatoires sur une vraie console, chacun unique, les mêmes pour tous les
programmes, et qu'un émulateur sous-cycle doit choisir aussi (partie 7).

**« Le PPU, lui, a besoin du demi-cycle. »**
Oui, pour les écritures du CPU dans ses registres, et Zaxnes Fidélité+ le fait déjà, au demi-top
d'horloge maître, avec 0 à 2 points d'écart sur le netlist hors artefacts du banc (partie 6).

**« Et les DMA ? »**
32 131 cycles de DMA comparés, collisions DMC/OAM comprises : 0 écart, ni de timing ni d'octet.

**« Le netlist peut se tromper. »**
Oui, sur ce qui n'est pas dans la puce (manettes, bus flottant), et on l'a montré (partie 9.1) ;
dans ce cas c'est la console réelle, via un test vérifié sur matériel, qui tranche, et Zaxnes la suit.

---

## 12. Reproduire

Tout est fourni pour refaire chaque chiffre de ce document.

### 12.1 Fichiers

* Programmes et outils : `docs/progress/cpu-diag/` (`mkprog.py`, `mkprog2.py`, `mkdmc.py`,
  `mkprog3.py`, `mkg6.py` pour les programmes ; `cmp3.py` pour la comparaison ; `excerpt2.py` pour
  les extraits ; `final5.sh` pour le tableau de la partie 5.1 ; `sweep.sh` pour la partie 7 ;
  `pack.py` pour l'archive).
* Traces : [`subcycle-proof/traces.zip`](subcycle-proof/traces.zip) contient, pour les 20
  programmes, le binaire (`<p>.bin`), la trace du netlist (`n4-<p>.txt`) et celle de Zaxnes avec le
  détail des DMA (`z5-<p>.txt`), plus `SHA256SUMS`.
* Journal de l'enquête, étape par étape : `docs/progress/CPU-NETLIST-DIAG.md`, et pour le PPU
  `docs/progress/FIDPLUS-2.md` et `docs/progress/FIDPLUS-A.md`.

### 12.2 Commandes

```sh
# Zaxnes, avec le détail des DMA
cargo build --release -p zax-harness --features bus-trace

# Netlist (banc VisualNes, hors dépôt) : programme, trace, nombre de cycles, maintien du reset
benchcpu.exe <VisualNes>/Core dmcoam.bin n4-dmcoam.txt 30000 4

# Zaxnes, mêmes réglages de démarrage pour tous les programmes
zax-harness cpu-bus-trace dmcoam.bin --cycles 29990 --alignment 3 --phase -8 \
    --flip-get-put --dmc 431,8 --out z5-dmcoam.txt

# Comparaison et extrait
python cmp3.py n4-dmcoam.txt z5-dmcoam.txt
python excerpt2.py n4-dmcoam.txt z5-dmcoam.txt 804 820
```

Sans le netlist, les traces de l'archive suffisent à vérifier les comparaisons (`cmp3.py` sur
`n4-*.txt` et `z5-*.txt`) ; avec un Zaxnes compilé, `cpu-bus-trace` regénère les `z5-*.txt` et leurs
sommes SHA-256 doivent correspondre à `SHA256SUMS`.
