# R2 — Relecture d'`AMD-054` (mixage APU non linéaire et audio du MMC5, v1.0)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-31 | Version du rapport : 1.0
Objet : relire `spec/changes/AMD-054-mixage-audio-3dsen-2.md` **en entier** et y chercher toute
fuite (code, pseudo-code, nom interne, algorithme recopié du programme d'origine plutôt que repris
de la documentation publique nesdev), toute incohérence avec `SPEC-02` (`REQ-CORE-070` à `-075`,
`REQ-CORE-080` à `-082`, `REQ-CORE-100` ligne « 5 | MMC5 »), `SPEC-11`, `Q-084` et les amendements
antérieurs, tout critère intestable et toute référence cassée. **Recalcul** de toutes les valeurs
numériques des critères (`K`, `Kₘ`, `P[15]`, `P[30]`, `T[45]`, `T[202]`, `31 × 128`).

Entrées relues : `spec/changes/AMD-054-mixage-audio-3dsen-2.md` (v1.0 du 2026-10-31, texte sous
revue, **non modifié**) ; `spec/SPEC-GUIDE.md` (en entier) ; `spec/SPEC-02` §3.7 (`REQ-CORE-070` à
`-075`), §3.8 (`REQ-CORE-080` à `-082`), §3.11 (`REQ-CORE-100`), §3.6 (`REQ-CORE-061`/`-062`), §5.3,
§6 (`Q5`), Historique ; `spec/SPEC-11` (**en entier** : §1 périmètre, §2, `REQ-AUD-001` à `-007`,
`-010` à `-013`, `-020` à `-023`, §4, §5, §6, Historique) ; `spec/SPEC-12` §2 (catégories auto /
[ROM] / [R2] / revue), `REQ-ACC-001`, `-020`, `-021`, `-030`, `-031`, `-050` ; `spec/DECISIONS-R0.md`
(`DEC-02-Q5`) ; `spec/SPEC-v1.2-INDEX.md` (tables des amendements, ligne « audio d'extension »,
ligne `REQ-CORE-073`) ; `spec/changes/Q-084.md` (en entier), `Q-039.md` (en entier, protocole de
mesure et réponse R0), `Q-007.md` (MMC5), `Q-006.md`, `AMD-018-q-tickets-p4a.md` (ligne `Q-039`),
`AMD-005-q-tickets-l1d.md` (`Q-007`), `AMD-049`/`R2-review-AMD-049.md` et
`R2-review-AMD-050-053.md` (forme attendue d'un amendement et d'un rapport R2) ;
`assets-neutral/profiles/` (inventaire des 101 profils embarqués, recherche d'un titre MMC5 à audio
d'extension) ; `impl/src/Zax.Nes/Apu/Apu2A03.cs` et `impl/tests/Zax.Nes.Tests/Apu*Tests.cs`
(périmètre des tests que l'amendement invalide) ; `journal/JOURNAL.md`.
Documentation publique consultée : wiki nesdev, pages **« APU Mixer »** (formules analytiques et
section « Lookup Table ») et **« MMC5 audio »** (carte des registres `$5000`–`$5015`, écarts avec
les pulses de l'APU, `$5010`/`$5011`/`$5015`, polarité, niveau du PCM, titres concernés), page
**« Myths »** et fils publics nesdev sur Castlevania III.
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement.
Le serveur d'outils de rétro-ingénierie présent dans la session n'a **pas** été utilisé.

Méthode : (a) réécriture **indépendante** des deux tables depuis le texte de l'amendement seul, en
double précision, et recalcul entrée par entrée des six valeurs chiffrées des critères, de `K`, de
`Kₘ`, de la somme des arrondis à pleine échelle et du rapport « deux pulses / un pulse » ;
(b) recalcul de la **variante** avec le coefficient réellement publié par nesdev pour la table de
consultation, pour mesurer si le choix est testable ; (c) balayage des 31 + 203 + 31 entrées
mises à l'échelle à la recherche d'une entrée posée sur un demi-entier (stabilité de l'arrondi) ;
(d) confrontation ligne à ligne de `REQ-CORE-075` amendée à la page publique « MMC5 audio » ;
(e) relecture de chaque critère comme le ferait un tiers qui ne connaîtrait que la spec, en
vérifiant qu'il porte sur une grandeur **observable** à la sortie que `REQ-CORE-071` définit.

Catégories utilisées, comme dans les rapports précédents : **LEAK**, **INCONSISTENT**,
**UNTESTABLE**, **UNCLEAR**, plus les points **rédactionnels**.

---

## 0. Verdict

| Document | Verdict | Points | Réserves bloquantes |
|---|---|---|---|
| **AMD-054** — mixage APU non linéaire et audio du MMC5 (v1.0) | **Accepté avec réserves — gate R2 NON franchi** | 18 (54-1 à 54-18) | **aucune pour l'étanchéité** ; **8 avant mise en œuvre** : 54-1, 54-2, 54-4, 54-5, 54-6, 54-7, 54-8, 54-9 |

- **Étanchéité : rien à signaler.** Aucun code, aucun pseudo-code, aucun nom interne de classe, de
  méthode, de champ ou de variable du programme d'origine, aucune référence à ses fichiers source,
  aucun numéro de ligne. Les deux tables, la carte des registres `$5000`–`$5015`, le comportement de
  l'écriture de `$00` en `$5011` et l'indice `3t + 2n + d` viennent **tous** de la documentation
  publique nesdev et s'y vérifient mot pour mot ; `SPEC-GUIDE` § « Ce que la spec PEUT contenir »
  les autorise explicitement (documentation publique, description mathématique d'un résultat,
  faits d'interopérabilité). Les seules grandeurs qui ne sont pas publiques — la normalisation à
  32 767, `Kₘ = 8000 / P[30]` et le pas PCM de 31 — sont des **constantes de résultat observable**,
  présentées comme telles ; elles manquent leur justification (54-10), ce qui est un défaut de
  rédaction, pas une fuite.
- **Les six valeurs chiffrées sont exactes.** Recalcul indépendant, double précision :

  | Grandeur | Recalcul R2 | AMD-054 | Verdict |
  |---|---|---|---|
  | `P[15]` | 0,149 376 82 | — | — |
  | `P[30]` | 0,258 483 11 | — | — |
  | `T[45]` | 0,255 477 12 | — | — |
  | `T[202]` | 0,742 467 61 | — | — |
  | `P[30] + T[202]` | 1,000 950 71 | — | — |
  | `K` | 32 735,877 6 | — | — |
  | `round(K · P[15])` | 4 889,98 → **4 890** | 4 890 ± 1 | **exact** |
  | `round(K · P[30])` | 8 461,67 → **8 462** | 8 462 ± 1 | **exact** |
  | `round(K · T[45])` | 8 363,27 → **8 363** | 8 363 ± 1 | **exact** |
  | `round(K · T[202])` | 24 305,33 → **24 305** | — | — |
  | somme à pleine échelle | 8 462 + 24 305 = **32 767** | 32 767 | **exact** (même après arrondi) |
  | rapport 2 pulses / 1 pulse | 8 462 / 4 890 = **1,730 5** | 1,73 | **exact** |
  | `Kₘ` | 30 949,798 | — | — |
  | `round(Kₘ · P[15])` | 4 623,18 → **4 623** | 4 623 ± 1 | **exact** |
  | `round(Kₘ · P[30])` | **8 000,00** | 8 000 | **exact** (par construction) |
  | `31 × 128` | **3 968** | 3 968 | **exact** |
  | `A + M` maximal | 32 767 + 8 000 + 31 × 255 = **48 672** (+48,5 %) | « peut dépasser 32 767 » | **exact** |

  Aucune erreur arithmétique. L'arithmétique de l'amendement est donc fiable : les réserves
  ci-dessous portent sur **ce que les formules ne disent pas** et sur **une valeur de table qui
  n'est pas celle de la source citée**, pas sur ses calculs.
- **Cohérence interne : moyenne.** Les deux exigences se tiennent entre elles (`P` partagée, même
  échelle entière, écrêtage cohérent avec le dépassement de 48,5 % que je retrouve). Mais la chaîne
  entre l'entier de 16 bits et le flottant de `[−1, 1]` que `REQ-CORE-071` exige n'est pas écrite
  (54-5), le filtre passe-haut que l'amendement dit « inchangé » n'existe **que** dans le corps de
  `REQ-CORE-073` qu'il remplace (54-4), et les six critères « auto » portent sur une grandeur que
  la spec n'expose nulle part (54-6). **Un implémenteur ne peut pas appliquer `REQ-CORE-073`
  amendée sans deviner trois choses, ni `REQ-CORE-075` amendée sans cinq.** C'est le cœur de ce
  rapport.
- **Un point est une erreur de fait, pas de rédaction** : le critère `[ROM]` « Castlevania III (US),
  écran titre : la mélodie jouée par le MMC5 est audible » **ne peut pas être satisfait** (54-9).

---

## 1. Points bloquants avant mise en œuvre

Les points sont numérotés dans l'ordre de lecture de l'amendement ; **54-3** est non bloquant et se
trouve donc au §2, en tête.

### 54-1 — INCONSISTENT (bloquant) — `P[i]` n'utilise pas le coefficient de la table publiée

`REQ-CORE-073` amendée pose `P[i] = 95,88 / (8128 / i + 100)` et le §1 attribue les deux tables aux
« **tables non linéaires** publiées par la documentation publique de la NES (nesdev, « APU
Mixer », **forme par tables de consultation** ) ». Or la page publique donne **deux** jeux de
constantes, et 95,88 n'est pas celui de la forme par tables :

| Forme nesdev | pulses | triangle + bruit + DMC |
|---|---|---|
| formules analytiques | `95,88 / (8128 / (pulse1 + pulse2) + 100)` | `159,79 / (1 / (t/8227 + n/12241 + d/22638) + 100)` |
| **tables de consultation** | `pulse_table[n] = ` **`95,52`** ` / (8128 / n + 100)` | `tnd_table[n] = ` **`163,67`** ` / (24329 / n + 100)` |

L'amendement prend donc **`163,67 / 24329` de la forme par tables** (correct, avec l'indice
`3t + 2n + d` et son maximum 202, que je confirme) mais **`95,88` de la forme analytique**. Les deux
ne se mélangent pas : les coefficients 95,52 et 163,67 ont été ajustés **ensemble** pour que la
table approche les formules.

L'écart est **testable**, donc il tranche. Recalcul avec `95,52` :

| Critère | AMD-054 (95,88) | Variante nesdev (95,52) | Dans le ± 1 ? |
|---|---|---|---|
| un pulse au volume 15 | **4 890** | 4 876,35 → 4 876 | **non** (−14) |
| deux pulses au volume 15 | **8 462** | 8 438,08 → 8 438 | **non** (−24) |
| triangle seul | **8 363** | 8 371,38 → 8 371 | **non** (+8) |
| MMC5, `round(Kₘ · P[15])` | **4 623** | 4 623,18 → 4 623 | oui (insensible) |

Les trois critères de `REQ-CORE-073` sortent tous de leur tolérance ; les critères du MMC5, eux,
sont **insensibles** au choix, parce que `Kₘ = 8000 / P[30]` élimine le numérateur de `P`. Un
implémenteur qui ouvre la page nesdev à la section citée écrira 95,52 et échouera aux trois
critères de l'APU sans comprendre pourquoi.

**Correction proposée** — au choix du propriétaire, mais il faut choisir :
(a) garder 95,88 et l'écrire : « le numérateur 95,88 est celui de la forme **analytique** de
nesdev, appliqué à la forme par tables (c'est la valeur de 3dSen 2.0) ; la forme par tables de
nesdev utilise 95,52, qui donnerait 4 876 / 8 438 / 8 371 » — et retirer « forme par tables de
consultation » du §1, ou le qualifier ; ou (b) retenir 95,52 et **remplacer les trois valeurs** par
4 876 ± 1, 8 438 ± 1 et 8 371 ± 1 (le rapport reste 1,73 : 8 438 / 4 876 = 1,7306).
**Bloquant** : contradiction entre l'exigence et la source qu'elle se donne, avec effet mesurable.

### 54-2 — UNCLEAR (bloquant) — « chaque table étant arrondie entrée par entrée à l'entier le plus proche » rend la sortie identiquement nulle

`REQ-CORE-073` amendée dit : « `A = round(K · P[p1 + p2]) + round(K · T[3t + 2n + d])` avec
`K = 32767 / (P[30] + T[202])`, **chaque table étant arrondie entrée par entrée à l'entier le plus
proche** ». Lu littéralement, « chaque table » désigne `P` et `T`, dont les valeurs sont dans
[0 ; 0,2585] et [0 ; 0,7425] : arrondies à l'entier le plus proche, **toutes** les entrées valent 0
sauf `T[i]` pour `T[i] ≥ 0,5` (`i ≥ 90`), et `K` lui-même devient `32767 / (0 + 1) = 32767`. La
sortie est alors nulle pour tout l'APU sauf un créneau binaire sur le triangle. `REQ-CORE-075`
reprend la même formule entre parenthèses : « `Kₘ = 8000 / P[30]` (table arrondie entrée par
entrée) » — avec `P[30]` arrondi à 0, `Kₘ` est une division par zéro.

L'intention est évidemment **les tables mises à l'échelle**, c'est-à-dire les deux tables d'entiers
`round(K · P[i])` (31 entrées) et `round(K · T[i])` (203 entrées), précalculées ; c'est ce
qu'expriment déjà les `round(…)` de la formule, la clause est donc à la fois redondante et fausse.

**Correction proposée** : remplacer la clause par « les deux tables d'entiers `round(K · P[i])`,
`i = 0..30`, et `round(K · T[i])`, `i = 0..202`, PEUVENT être précalculées ; `K` et `Kₘ` sont
calculés sur les valeurs réelles de `P` et `T`, jamais sur des valeurs arrondies », et supprimer la
parenthèse « (table arrondie entrée par entrée) » de `REQ-CORE-075`.
**Bloquant** : la lecture littérale donne le silence et une division par zéro.

### 54-4 — INCONSISTENT (bloquant) — le filtre de composante continue disparaît avec le corps qu'il amende

`REQ-CORE-073` amendée renvoie les traitements en aval à l'existant : « Les traitements en aval
(**filtre de composante continue**, rééchantillonnage, volume) restent ceux de `SPEC-02` et
`SPEC-11` ». Or, dans `SPEC-02`, le filtre de composante continue n'est spécifié **nulle part
ailleurs que dans le corps de `REQ-CORE-073`** que l'amendement remplace :

> La sortie vaut 1,526 × mix, bornée à [−1, 1], suivie d'un filtre passe-haut coupant la composante
> continue (**fréquence de coupure de 10 à 40 Hz**). (`SPEC-02` §3.7, `REQ-CORE-073`)

`REQ-CORE-070` ne parle que du rééchantillonnage à bande limitée ; `SPEC-11` §1 range
explicitement le sujet **hors de son périmètre** ; `Q-039` cite « le filtre passe-haut exigé par
`REQ-CORE-073` **lui-même** ». Le renvoi est donc circulaire et, après amendement, la plage
10–40 Hz et la seconde moitié du critère d'acceptation (« un silence prolongé — tous canaux à
niveau constant — tend vers 0 en moins de 0,2 s ») ne sont plus portées par aucune exigence.
L'amendement ne dit pas non plus si ce second critère est retiré : ses Notes ne retirent que la
note « ±10 % » et le « 0,172 ± 0,02 ».

**Correction proposée** : réénoncer dans `REQ-CORE-073` amendée « la sortie est ensuite filtrée par
un passe-haut coupant la composante continue (fréquence de coupure de 10 à 40 Hz) », conserver
explicitement le critère « un silence prolongé tend vers 0 en moins de 0,2 s », et dire que ce sont
les seuls éléments du corps précédent qui survivent. (Variante : porter le filtre dans une exigence
propre `REQ-CORE-076`, libre — voir 54-16.)
**Bloquant** : une exigence P2 gelée perd son contenu sans remplacement.

### 54-5 — UNCLEAR (bloquant) — le passage de l'entier de 16 bits au flottant de `REQ-CORE-071` n'est pas spécifié

`REQ-CORE-073` amendée produit « la sortie de l'APU, en échelle entière signée de 16 bits », et
`REQ-CORE-075` amendée écrête `A + M` à 32 767. Mais `REQ-CORE-071`, **inchangée et P1**, exige que
l'hôte lise des **valeurs flottantes dans [−1, 1]**. L'amendement ne dit pas comment on passe de
l'une à l'autre : division par 32 767 ou par 32 768, avant ou après le filtre de continu, et quel
gain remplace le `1,526 ≈ 50 000 / 32 768` que le corps précédent portait et qui disparaît sans
mention.

Ce n'est pas une subtilité : le niveau audible change, et `SPEC-11` applique ensuite `V`
(`REQ-AUD-006`) et les fondus (`REQ-AUD-020`) à ce signal.

| Grandeur | Ancienne chaîne (linéaire × 1,526) | Nouvelle chaîne (/32 767) | Écart |
|---|---|---|---|
| un pulse au volume 15, crête à crête | 0,1721 | 4 890 / 32 767 = **0,149 24** | **−13,3 %** |
| pleine échelle APU | 0,8528 × 1,526 = 1,301 → **écrêté** | 32 767 / 32 767 = **1,000 00** | plus d'écrêtage |

**Correction proposée** : ajouter une phrase — « la valeur entière `A + M`, écrêtée à 32 767, est
divisée par 32 767 pour donner l'échelle [−1, 1] de `REQ-CORE-071` ; le filtre de composante
continue s'applique ensuite ; le gain 1,526 de la version précédente est retiré, la normalisation
par `K` le remplace » — et une note sur la baisse de niveau de 13 % d'un pulse seul, qui est
audible et qui est le vrai effet observable de l'amendement.
**Bloquant** : l'exigence ne dit pas ce qu'elle produit à la sortie que la spec définit.

### 54-6 — UNTESTABLE (bloquant) — les six critères « auto » portent sur une grandeur non observable, et abandonnent le protocole normatif de `Q-039`

Les six critères chiffrés (4 890, 8 462, 8 363, 4 623, 8 000, 3 968) sont exprimés en **unités
entières du mixeur**. Or le cœur n'expose pas cette grandeur : `REQ-CORE-071` ne donne que des
flottants **après** le bloqueur de continu et **après** le rééchantillonnage à bande limitée de
`REQ-CORE-070`. Deux conséquences :

1. `SPEC-GUIDE` exige un critère « vérifiable par un tiers qui ne connaît que la spec ». En l'état,
   le tiers ne sait pas où mesurer.
2. Surtout, **l'amendement efface une réponse R0 normative**. `Q-039` (ticket R4 du 2026-10-21) a
   établi que la mesure de ce niveau exact dépend de deux paramètres que le critère doit fixer :
   le filtre passe-haut fait varier la crête à crête de 0,181 à 440 Hz jusqu'à **0,214 à 48 Hz**
   alors que le mixeur est exact, et `max − min` sur le flux inclut le dépassement de Gibbs du
   rééchantillonneur. La réponse R0 du 2026-10-21 a rendu **normatif** le protocole proposé :
   **440 Hz (timer 253 en NTSC), mesure sur les deux plateaux (différence des médianes au-dessus et
   au-dessous de la moyenne), première moitié des échantillons écartée** ; `AMD-018` l'a inscrit au
   titre de `Q-039`. `AMD-054` retire le « 0,172 ± 0,02 » auquel ce protocole était attaché **sans
   reprendre le protocole**, et ne cite ni `Q-039` ni `AMD-018`. La tension que `Q-039` a résolue
   revient intacte sur les nouveaux chiffres.

**Correction proposée** — l'une ou l'autre, explicitement :
(a) énoncer les critères à la sortie de `REQ-CORE-071`, avec le protocole de `Q-039` repris mot pour
mot et les équivalents que j'ai recalculés : 0,149 24 (un pulse), 0,258 25 (deux pulses), 0,255 23
(triangle), 0,141 09 (une carrée MMC5), 0,244 15 (deux carrées MMC5), 0,121 10 (PCM à 128), avec la
tolérance de ± 0,002 que `Q-039` a mesurée comme atteignable ; ou
(b) déclarer la sortie entière du mixeur **point de test exposé par le harnais** (catégorie « auto »
de `SPEC-12` §2), garder les six entiers, et dire d'une phrase que les critères portent sur ce point
de test et non sur la sortie de `REQ-CORE-071`.
Dans les deux cas, ajouter « (protocole de `Q-039`, confirmé par `AMD-018`) ».
**Bloquant** : six critères sur six ne sont pas rattachables à une grandeur de la spec, et une
réponse R0 est perdue en silence.

### 54-7 — INCONSISTENT (bloquant) — « identiques aux pulses de l'APU » contredit la documentation citée sur deux points, dont la cadence

`REQ-CORE-075` amendée décrit les voies carrées du MMC5 comme « **identiques aux pulses de l'APU
sans balayage** (registres `$5000`–`$5007`, leur compteur de longueur et leur enveloppe **cadencés
comme dans la documentation**) ». La page publique « MMC5 audio » énonce deux écarts de plus, et le
second est précisément celui qu'un implémenteur qui réutilise les voies de l'APU manque :

1. **Pas de séquenceur de trame.** Le MMC5 n'a pas d'équivalent du frame counter : enveloppe et
   compteur de longueur sont à une cadence **fixe d'environ 240 Hz**, indépendante de `$4017` et du
   mode 4 pas / 5 pas — là où l'APU cadence ses compteurs de longueur à 120 Hz et dépend de
   `$4017`. La formule « cadencés comme dans la documentation » renvoie donc à la documentation
   exactement là où elle contredit le « identiques aux pulses de l'APU » de la même phrase.
2. **Périodes basses non coupées.** Une période inférieure à 8 ne rend pas la voie MMC5 silencieuse
   (sortie ultrasonique), alors que le pulse de l'APU est coupé dans ce cas.

**Correction proposée** : remplacer la parenthèse par un énoncé explicite — « deux voies carrées
reprenant la forme d'onde, le rapport cyclique, le timer, l'enveloppe et le compteur de longueur
des pulses de l'APU, **sans unité de balayage**, **sans séquenceur de trame** (enveloppe et
compteur de longueur à une cadence fixe de 240 Hz, indépendante de `$4017`), et **sans coupure des
périodes inférieures à 8** » ; et ajouter un critère « auto » qui exerce le point 1 : « écrire
`$80` puis `$C0` en `$4017` ne change pas la vitesse de décroissance de l'enveloppe d'une voie
MMC5 ; une voie MMC5 à enveloppe décroissante passe de 15 à 0 en 15 × 1/240 s ± 5 % ».
**Bloquant** : contradiction interne d'une phrase à l'autre, et la seule clause explicite
(« sans balayage ») désigne le moins surprenant des trois écarts.

### 54-8 — INCONSISTENT (bloquant) — l'IRQ PCM est exigée sans voie d'acquittement

`REQ-CORE-075` amendée exige qu'une écriture de `$00` en `$5011` « signale l'IRQ PCM si elle est
activée par `$5010` », et s'arrête là. Elle ne dit ni comment le drapeau se remet à zéro, ni ce que
rend une lecture de `$5010`. La documentation publique est explicite : **`$5010` est en
lecture/écriture ; une lecture renvoie l'état de l'IRQ PCM, l'acquitte et efface le drapeau**
(bit 0 = mode 0 = écriture / 1 = lecture, bit 7 = autorisation d'IRQ).

Tel qu'écrit, un jeu qui autorise l'IRQ puis écrit `$00` obtient une ligne d'IRQ que rien ne relâche
jamais : le CPU reboucle indéfiniment dans son gestionnaire. L'exigence rend donc le jeu
injouable **en suivant la lettre de la spec**.

**Correction proposée** : ajouter « une lecture de `$5010` renvoie l'état de l'IRQ PCM et
l'**acquitte** (drapeau remis à zéro, ligne d'IRQ relâchée) ; l'IRQ n'est signalée que si le bit 7
de `$5010` est à 1 », avec un critère « auto » : « `$5010` ← `$80`, `$5011` ← `$00` → la ligne
d'IRQ est active ; une lecture de `$5010` la relâche ; une seconde lecture renvoie le drapeau à 0 ».
Préciser aussi, en une phrase, que la lecture **hôte/Lua** de `REQ-CORE-080`/`-081` (zone `$4020` à
`$5FFF`, « DEVRAIENT être sans effet de bord ») **n'acquitte pas** cette IRQ — sans quoi l'exigence
nouvelle et `REQ-CORE-081` se contredisent sur la même adresse.
**Bloquant** : une exigence P2 qui, appliquée telle quelle, bloque le CPU émulé.

### 54-9 — INCONSISTENT (bloquant) — le critère `[ROM]` ne peut pas être satisfait : Castlevania III n'utilise pas l'audio du MMC5

Le §1 affirme que l'audio du MMC5 est « utilisé notamment par **Castlevania III (versions US et
européenne)**, Just Breed et Metal Slader Glory », et le critère `[ROM]` exige : « « Castlevania
III » (US), écran titre : **la mélodie jouée par le MMC5 est audible** ».

C'est faux, et la documentation publique le dit de deux façons :

- **Matériellement impossible.** L'audio d'extension d'une cartouche n'atteint le mélangeur que par
  la broche d'entrée audio du **connecteur de cartouche de la Famicom** ; le connecteur 72 broches
  de la NES n'en a pas. Aucun jeu NES (72 broches), MMC5 ou non, ne peut faire sonner un canal
  d'extension. Les versions US et européenne de Castlevania III sont des cartouches 72 broches.
- **Historiquement constaté.** La version japonaise (Famicom, VRC6) utilise l'audio d'extension du
  VRC6 ; pour la sortie nord-américaine, le mapper a été remplacé par le MMC5 et **la musique a été
  réécrite pour les cinq canaux standard**. La page publique « Myths » de nesdev traite
  précisément la rumeur inverse et rappelle que Castlevania III est très loin d'exploiter les
  capacités du MMC5 — le MMC5 y a été retenu pour ses modes de banques, pas pour son son.

Conséquences : (i) une implémentation **parfaitement conforme** échouerait au critère `[ROM]`, qui
est donc à retirer ou à remplacer ; (ii) la liste du §1 est à corriger. La documentation publique
ne cite qu'**un** titre pour la voie PCM (Shin 4 Nin Uchi Mahjong) ; **Just Breed** est l'exemple
usuel pour les voies carrées ; l'usage de l'audio par **Metal Slader Glory** n'est pas établi par
la source citée et doit être vérifié ou retiré. Tous les titres concernés sont des jeux **Famicom**.

Troisième conséquence, de périmètre : **aucun titre à audio MMC5 n'est dans le corpus**. Les 101
profils d'`assets-neutral/profiles/` ne contiennent ni Castlevania III, ni Just Breed, ni Uchuu
Keibitai SDF, ni Shin 4 Nin Uchi Mahjong, ni Metal Slader Glory (seuls « Castlevania (U) (PRG 1) »
et « Castlevania II »), et aucun jeu de référence de `SPEC-12` `REQ-ACC-020` n'en fait partie.
C'est exactement ce que `SPEC-02` §6 `Q5` relevait (« aucun titre embarqué ne semble en dépendre »).

**Correction proposée** : retirer « Castlevania III (versions US et européenne) » du §1 ;
remplacer le critère `[ROM]` par un titre Famicom documenté — proposition : « « Just Breed »
(Famicom), carte du monde : avec l'audio MMC5 désactivé puis activé, l'enregistrement de 10 s
diffère ; au moins une composante spectrale absente du premier enregistrement dépasse −40 dBFS dans
le second » (critère différentiel, mesurable, qui ne dépend pas d'un jugement « est audible ») ;
ajouter la ROM retenue à la liste des ROMs attendues de l'utilisateur (`SPEC-12` §2 et
`REQ-ACC-020`) ; et reconnaître que la partie `[ROM]` comptera « en attente de ROM » au gate P4
(`REQ-ACC-050`), la partie « auto » restant exigible.
**Bloquant** : critère d'acceptation faux, et énoncé de fait faux au §1.

---

## 2. Points non bloquants, par ordre d'importance

### 54-3 — UNTESTABLE — le premier critère de `REQ-CORE-073` est une identité, pas un test

« Critère d'acceptation (auto) : `K · P[30] + K · T[202] = 32767` ». Avec
`K = 32767 / (P[30] + T[202])`, cette égalité est vraie **par construction**, pour n'importe quelles
tables `P` et `T` : elle ne distingue aucune implémentation. Écrite sur des réels sans tolérance,
elle n'est de plus pas vérifiable en virgule flottante.

Ce qui mérite d'être affirmé, et que j'ai vérifié, c'est la propriété non triviale **sur les tables
d'entiers** : `round(K · P[30]) = 8 462`, `round(K · T[202]) = 24 305`, et
`8 462 + 24 305 = 32 767` **exactement** — la pleine échelle est atteinte sans débordement, malgré
les deux arrondis indépendants de la formule.

**Correction proposée** : « Critère d'acceptation (auto) : les deux termes à pleine échelle valent
`round(K · P[30]) = 8 462` et `round(K · T[202]) = 24 305`, de somme **exactement** 32 767 : la
pleine échelle est atteinte et jamais dépassée par `A` seul. » Non bloquant.

### 54-10 — UNCLEAR — `Kₘ = 8000 / P[30]` et le pas PCM de 31 ne sont pas justifiés, et le niveau PCM s'écarte d'un facteur 4,7 de la documentation citée

`SPEC-GUIDE` demande que « les constantes qui déterminent un résultat observable » soient données
« avec leur justification ». `REQ-CORE-075` amendée en introduit deux, sans un mot : `8000` et `31`.
Ce que montre le recalcul :

- `31 × 255 = 7 905 ≈ 8 000` : l'intention paraît être « pleine échelle du PCM ≈ pleine échelle de
  la paire de carrées ». Le pas exact serait `8000 / 255 = 31,37` ; `31` donne 7 905, soit −1,2 %.
- `8 000` place la paire de carrées MMC5 **5,5 % sous** celle de l'APU (8 000 contre 8 462), alors
  que la documentation publique dit les carrées du MMC5 « equivalent in volume to the corresponding
  APU channels ».
- Surtout, le **PCM** se retrouve très bas. À entrée égale (valeur 127), le DMC de l'APU seul
  contribue `round(K · T[127]) = 18 376`, tandis que le PCM du MMC5 donne `31 × 127 = 3 937` :
  un facteur **4,67**. La documentation publique dit l'inverse (« equivalent in volume to the APU
  with equivalent input … but using the extra high bit of input it can become twice as loud »,
  donc jusqu'à ≈ 2 × le DMC à pleine échelle). Le modèle de niveau de `REQ-CORE-075` n'est donc
  **pas** celui de la source qu'elle invoque.
- Modèle hétérogène, enfin : le terme PCM est **strictement linéaire** alors que le DMC de l'APU
  passe par la table non linéaire `T` qu'un canal partagé impose. Ce n'est pas incohérent (le MMC5
  a son propre convertisseur), mais c'est un choix qui doit être dit.

**Correction proposée** : ajouter une note — « `Kₘ = 8000 / P[30]` et le pas de 31 par unité de PCM
sont les niveaux **relevés sur 3dSen 2.0** (pleine échelle du PCM 7 905 ≈ pleine échelle de la paire
de carrées 8 000) ; ils placent la paire de carrées 5,5 % sous celle de l'APU et le PCM 4,7 fois
sous le DMC de l'APU à entrée égale, là où la documentation publique les donne équivalents : écart
assumé **(D)**. Le terme PCM est linéaire, le DMC de l'APU reste dans la table `T`. » Non bloquant
(l'exigence est chiffrée et testable), mais à arbitrer : c'est la fidélité au son d'origine qui est
en jeu, et le §1 annonce « selon la documentation publique » sans réserve.

### 54-11 — INCONSISTENT — la polarité inversée des canaux MMC5 n'est pas traitée

La documentation publique est nette : « the polarity of **all** MMC5 channels is reversed compared
to the APU ». `REQ-CORE-075` amendée ajoute `M` à `A` **avec le signe +**, et écrête la somme vers
le haut. Sur une voie MMC5 seule, le bloqueur de continu rend le signe inaudible ; mais dès que
l'APU et le MMC5 jouent ensemble, le signe décide s'ils s'additionnent ou se compensent
partiellement — et il décide aussi du sens de l'écrêtage.

**Correction proposée** : soit suivre la source (`A − M`, avec un écrêtage symétrique à préciser),
soit garder `A + M` et l'écrire comme un écart assumé : « la sortie du MMC5 est **ajoutée** à celle
de l'APU ; la documentation publique donne la polarité des canaux MMC5 inversée par rapport à
l'APU, écart assumé **(D)** (3dSen 2.0 les additionne) ». Non bloquant (le `+` est bien défini),
mais la phrase « DOIT être émulé selon la documentation publique » devient inexacte sans cette
réserve.

### 54-12 — UNCLEAR — la règle d'arrondi et la précision de `K` ne sont pas fixées, et une entrée est posée sur un demi-entier

L'amendement écrit `round(…)` sans dire comment on arrondit un demi-entier, ni en quelle précision
`K` et `Kₘ` sont calculés. J'ai balayé les 31 + 203 + 31 entrées mises à l'échelle : une entrée
tombe à **3,5 × 10⁻⁴** d'un demi-entier — `K · T[196] = 23 905,499 65`. Selon la convention
(au plus proche pair, ou à l'écart de zéro) ou simplement selon que `K` est calculé en simple ou en
double précision, deux implémentations conformes diffèrent de 1 LSB sur cette entrée. Le ± 1 des
critères le masque, mais pas les exigences de déterminisme.

**Correction proposée** : ajouter « les tables sont calculées en **double précision** et arrondies à
l'entier le plus proche, un demi-entier exact s'arrondissant vers le haut ; l'entrée
`K · T[196] = 23 905,4997` est à 3 × 10⁻⁴ d'un demi-entier et vaut donc 23 905 ». Non bloquant.

### 54-13 — UNCLEAR — `$5015`, le mode lecture de `$5010` et le compteur PCM restent à deviner

Trois lacunes dans `REQ-CORE-075` amendée :

1. « **et l'état `$5015`** » n'énonce rien. La documentation publique : seuls les 2 bits de poids
   faible servent, un par voie carrée, et le registre est **lisible**. Il faut dire ce que fait la
   mise à 0 d'un bit (voie coupée / compteur de longueur forcé à 0, comme `$4015` pour l'APU) et ce
   que rend une lecture.
2. L'exigence ne demande que « une voie PCM **en mode écriture** ». Le **mode lecture**
   (`$5010` bit 0 = 1, où les lectures de `$8000`–`$BFFF` alimentent le convertisseur) n'est ni
   exigé ni exclu : il faut l'exclure explicitement **et** dire ce que la voie produit alors (niveau
   gelé ? silence ?), sans quoi le comportement d'un jeu en mode lecture est indéfini.
3. La documentation ajoute qu'une écriture de `$00` ne modifie pas non plus **le compteur PCM** ;
   l'amendement ne dit que « ne change pas le niveau ». À reprendre si le compteur est observable.

**Correction proposée** : trois clauses, une par point ; pour le point 2, « le mode lecture
(`$5010` bit 0 = 1) n'est pas exigé **(D)** : le niveau PCM reste alors à sa dernière valeur
écrite ». Non bloquant mais chacune des trois obligerait sinon à un ticket `Q`.

### 54-14 — UNTESTABLE — deux des six critères n'énoncent pas leurs conditions de mesure

- « les **deux pulses au volume 15** donnent `round(K · P[30])` = 8 462 » : vrai seulement si les
  deux pulses ont la **même période et la même phase**. Deux créneaux de rapport cyclique 50 % en
  opposition de phase donnent une somme **constante** à 15, donc une crête à crête **nulle** après
  le bloqueur de continu. Le critère est donc satisfait ou nul selon un paramètre qu'il ne fixe pas.
- « **triangle au maximum** seul » : le triangle n'a pas de réglage de volume ; sa sortie parcourt
  l'escalier de 32 pas 15…0…15. La valeur 8 363 est donc la **crête à crête de l'escalier**
  (indice 45 → 0), pas un niveau, et « au maximum » n'a pas de sens pour ce canal.

**Correction proposée** : « les deux pulses réglés sur la **même période** et démarrés en **phase**
au volume 15 → … » ; et « le **triangle actif seul** (bruit et DMC silencieux) : l'indice parcourt
45 à 0, excursion crête à crête `round(K · T[45])` = 8 363 ± 1 ». Même remarque de condition pour
« une voie carrée MMC5 seule au volume 15 » (rapport cyclique à préciser, 50 %). Non bloquant, mais
sans ces précisions le test sera écrit deux fois.

### 54-15 — Références cassées — il manque un §« Changements documentaires », et cinq documents contredisent l'amendement

`AMD-049` à `AMD-053` portent tous un §« Changements documentaires » qui liste les éditions à faire,
y compris dans `SPEC-v1.2-INDEX`. `AMD-054` n'en a pas : il ne contient qu'une ligne
(« `SPEC-11` §1 : la mention du mixage non linéaire y devient conforme à `REQ-CORE-073` amendée »).
Après son adoption, **cinq** textes restent en contradiction :

a. **`SPEC-02` `REQ-CORE-100`, ligne « 5 | MMC5 (ExROM) »** : « au minimum : modes de banques PRG et
   CHR, PRG-RAM, IRQ de ligne. ExRAM, attributs étendus, split vertical : P2 ». L'audio n'y figure
   pas : la table des mappers et `REQ-CORE-075` amendée disent deux choses différentes de ce que
   « mapper 5 » exige. Ajouter « audio (deux voies carrées et PCM, `REQ-CORE-075`) : P2 ».
b. **`SPEC-02` §6 `Q5`** : « *Close par `DEC-02-Q5`* (non requis, P3). **Audio d'extension.** Aucun
   titre embarqué ne semble en dépendre … À confirmer par l'inventaire des ROMs des 101 titres. »
   À réécrire : la question est rouverte puis tranchée par `AMD-054` pour le MMC5 — en notant que sa
   prémisse (aucun titre embarqué n'en dépend) reste **vraie**, et que c'est précisément ce qui rend
   la nouvelle P2 non vérifiable sur le corpus (54-9).
c. **`DECISIONS-R0.md`** : « **DEC-02-Q5** Audio d'extension non requis (P3 optionnel). »
   `AMD-054` doit déclarer cette décision **remplacée** pour le MMC5. C'est d'autant plus nécessaire
   que `Q-084` §5 s'appuie dessus pour conclure l'inverse (« il n'est pas émulé, et l'émuler
   introduirait une divergence audible avec la référence là où il n'y en a aucune aujourd'hui »).
d. **`SPEC-v1.2-INDEX.md`** : la ligne « | audio d'extension | `DEC-02-Q5` : non requis (P3) | » et
   les tables de renvoi (la ligne `REQ-CORE-073` y cite encore `AMD-018 Q-039` seul). Ajouter la
   ligne d'`AMD-054`, comme `AMD-049` le fait au point 3 de son §3.
e. **`SPEC-11`** (gelé en 1.1) : la ligne proposée est vague. En fait, `SPEC-11` §1 (« Hors
   périmètre : la synthèse des voies de l'APU, **le mixage non linéaire** et le rééchantillonnage à
   bande limitée (`SPEC-02`) ») devient **littéralement exacte** telle quelle — c'est l'objet de la
   question 2 de `Q-084`, qui se trouve ainsi résolue **sans édition**. Il faut l'écrire ainsi
   (« aucune modification : la mention devient exacte, `Q-084` question 2 close par le fait »),
   plutôt que « devient conforme », qui laisse croire à une édition non spécifiée. En revanche une
   **addition** est nécessaire : `SPEC-11` §2 et §3.1 ne parlent que du « signal de la console », qui
   inclut désormais l'audio d'extension de la cartouche — une incise suffit.

Manquent enfin une ligne **Historique** et une clause **Priorités**, présentes dans `AMD-049`
à `AMD-053`. Non bloquant (ce sont des éditions, pas des choix), mais sans ce § l'amendement laisse
`DEC-02-Q5` et la table des mappers le contredire.

### 54-16 — Rédactionnel / `SPEC-GUIDE` — une exigence ne peut pas porter deux priorités

« **`REQ-CORE-075` amendée** (**P2 pour le MMC5, P3 pour les autres**) ». `SPEC-GUIDE` attribue une
priorité par exigence, et le gate P4 de `SPEC-12` `REQ-ACC-050` compte séparément « 100 % des
exigences P1 et au moins 95 % des exigences P2 » : une exigence à double priorité n'est pas
comptable.

**Correction proposée** : scinder — `REQ-CORE-075` amendée (**P2**) pour l'audio du MMC5, et une
nouvelle **`REQ-CORE-076`** (**P3**) pour les autres audios d'extension (VRC6, VRC7, Namco 163,
Sunsoft 5B, FDS) qui « PEUVENT être émulés ; ils ne sont pas exigés ». J'ai vérifié que
`REQ-CORE-076` à `-079` sont **libres** (aucune occurrence dans `spec/` ni dans `impl/docs/`) ;
`SPEC-GUIDE` interdit de **réutiliser** un numéro, pas de réénoncer un sujet sous un numéro neuf.
Si `REQ-CORE-076` sert déjà au filtre de continu (54-4), prendre `-077`. Non bloquant.

### 54-17 — Rédactionnel — renvoi non consultable depuis le côté propre

L'en-tête se fonde sur « la **piste §8 du rapport de comparaison avec 3dSen 2.0** ». Ce document est
du côté sale : la référence est, pour le côté propre, pendante. **Ce n'est pas une fuite** (aucun
contenu ne passe, et `AMD-049` à `AMD-053` usent de la même convention), et le §1 résume déjà les
faits observés, qui suffisent. Mentionné pour mémoire seulement. Non bloquant.

### 54-18 — Rédactionnel — « échelle entière signée de 16 bits » et place de l'écrêtage

`A` est une somme de deux termes positifs : son domaine est **0 à 32 767**, unipolaire ; c'est le
bloqueur de continu qui la recentre. « Échelle entière **signée** de 16 bits » est donc trompeur.
Par ailleurs l'amendement ne dit pas où se place l'écrêtage de `REQ-CORE-075` par rapport au filtre
de continu (avant, vraisemblablement, puisqu'il porte sur `A + M`).

**Correction proposée** : « en échelle entière de 0 à 32 767 (16 bits signés une fois la composante
continue retirée) » ; et « l'écrêtage de `A + M` s'applique **avant** le filtre de composante
continue ». Non bloquant.

---

## 3. Conclusion — gate R2

**Verdict : accepté avec réserves. Gate R2 NON franchi en v1.0.**

- **Étanchéité : acquise.** Aucun LEAK. Tout ce qui est technique dans l'amendement se retrouve mot
  pour mot dans la documentation publique nesdev (« APU Mixer », « MMC5 audio ») ; les trois
  constantes qui n'en viennent pas (32 767, `8000`, `31`) sont des constantes de résultat
  observable, autorisées par `SPEC-GUIDE`. Rien n'est recopié du programme d'origine, ni dans la
  structure, ni dans le nommage. **De ce point de vue, l'amendement peut passer en l'état.**
- **Arithmétique : exacte.** Les six valeurs des critères (4 890, 8 462, 8 363, 4 623, 8 000,
  3 968), le rapport 1,73, la somme à pleine échelle 32 767 et le dépassement de `A + M` ont tous
  été recalculés indépendamment et sont **justes** — y compris la propriété non triviale
  `round(K · P[30]) + round(K · T[202]) = 32 767` exactement.
- **Huit réserves à lever avant mise en œuvre**, dans cet ordre :
  **54-9** (le critère `[ROM]` est faux : les Castlevania III US et européenne, cartouches
  72 broches, ne peuvent pas faire sonner un canal d'extension et leur musique a été réécrite pour
  les cinq canaux standard — à remplacer par un titre Famicom documenté, avec un critère
  différentiel) ; **54-1** (le numérateur 95,88 n'est pas celui de la table publiée, 95,52 : il faut
  choisir, les trois critères de l'APU en dépendent — 4 876 / 8 438 / 8 371 dans l'autre branche) ;
  **54-4** (le filtre de composante continue 10–40 Hz n'existe que dans le corps remplacé : il
  disparaît) ; **54-6** (les six critères portent sur une grandeur non exposée et abandonnent le
  protocole normatif de `Q-039` / `AMD-018`) ; **54-8** (IRQ PCM exigée sans acquittement : le CPU
  émulé reboucle) ; **54-5** (rien ne dit comment l'entier devient le flottant de `[−1, 1]` de
  `REQ-CORE-071`, ni que le niveau baisse de 13 %) ; **54-7** (« identiques aux pulses de l'APU »
  contredit la cadence fixe de 240 Hz et l'absence de coupure sous la période 8) ; **54-2** (la
  clause « chaque table arrondie entrée par entrée » donne, lue littéralement, le silence et une
  division par zéro).
  **54-2, 54-4, 54-5 et 54-8** sont des corrections d'une à trois phrases. **54-1, 54-7 et 54-9**
  demandent un arbitrage du propriétaire. **54-6** demande de reprendre les six critères.
- **Ensuite, par ordre d'importance** : **54-3** (le premier critère est une identité, à remplacer
  par la propriété sur les tables d'entiers), **54-10** (justifier `8000` et `31` ; le PCM est
  4,7 fois sous le DMC de l'APU là où la source citée les donne équivalents — à assumer en **(D)**),
  **54-11** (polarité MMC5 inversée dans la source, additionnée ici), **54-12** (règle d'arrondi :
  `K · T[196] = 23 905,4997`), **54-13** (`$5015`, mode lecture, compteur PCM), **54-14**
  (conditions de mesure : phase des deux pulses, « triangle au maximum »), **54-15** (le
  §« Changements documentaires » manquant : `REQ-CORE-100` ligne 5, `SPEC-02` §6 `Q5`,
  `DEC-02-Q5`, `SPEC-v1.2-INDEX`, `SPEC-11` §1/§2), **54-16** (double priorité ; scinder sur
  `REQ-CORE-076`, vérifiée libre), puis **54-17** et **54-18**, rédactionnels.
- **`Q-084` est bien close** par l'amendement : sa question 1 l'est par le retrait de la note
  « ±10 % » et du critère « 0,172 ± 0,02 » et par les nouveaux critères ; sa question 2 l'est **par
  le fait** (`SPEC-11` §1 devient exact sans édition, cf. 54-15e). En revanche son §5 — « l'audio
  d'extension n'est **pas** concerné … il n'est pas émulé, et l'émuler introduirait une divergence
  audible avec la référence » — est **renversé** par `REQ-CORE-075` amendée, et doit être déclaré
  dépassé en même temps que `DEC-02-Q5`.
- **Impact de mise en œuvre, pour mémoire** : l'amendement invalide la loi de mixage actuelle et les
  tests qui la verrouillent. Sont à réécrire ou retirer, dans `impl/tests/Zax.Nes.Tests/`, au moins
  `ApuOutputTests.REQ_CORE_073_each_channel_contributes_its_documented_step`,
  `…_the_triangle_and_the_dmc_contribute_their_documented_steps`,
  `…_the_mix_is_the_sum_of_the_channels_and_is_clamped_to_one`,
  `…_a_lone_pulse_at_volume_fifteen_swings_by_zero_point_one_seven_two` et le test marqué
  `// Q-084` `…_the_non_linear_formula_of_nesdev_misses_the_criterion`, devenu sans objet ; le test
  de décroissance du continu `…_a_constant_level_decays_to_zero_in_less_than_two_hundred_milliseconds`
  doit **survivre**, ce qui suppose 54-4. Les tests de canaux (`ApuChannelTests`) ne sont pas
  touchés : ils portent sur les niveaux par canal, non sur le mélange. Côté audio d'extension, tout
  est à écrire. Rien n'a été modifié côté `impl`.

Le fichier `AMD-054-mixage-audio-3dsen-2.md` n'a pas été modifié par cette relecture ; aucun autre
fichier de `spec/` que ce rapport n'a été créé ou modifié ; rien n'a été commité.
