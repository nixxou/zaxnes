# R2 — Relecture d'AMD-050, AMD-051, AMD-052 et AMD-053

Rôle R2 (filtre anti-fuite, côté PROPRE) | 2026-10-31 | Gate R2

Amendements relus, en entier :

- `AMD-050-mode-2d-fond-transparent.md` (REQ-SCN-076) ;
- `AMD-051-robustesse-conversion.md` (REQ-E3D-644 à 646, REQ-LOOP-073, REQ-PERS-088) ;
- `AMD-052-cadence-file-gpu-msaa.md` (REQ-LOOP-074, 075, REQ-SCN-077, amendement d'AMD-032 §2) ;
- `AMD-053-reprise-favoris-ligne-de-commande.md` (REQ-LOOP-076, REQ-UI-210 à 213).

Référence appliquée : `spec/SPEC-GUIDE.md`.

Sources consultées : `spec/SPEC-GUIDE`, `spec/SPEC-02` (§3.5 `REQ-CORE-030` à `-048`, §4.1, §4.2),
`spec/SPEC-03` (`REQ-PROF-001` à `-004`, `-021`, `-022`, `-033`, `-041`, `-052`, `-060`, `-063`,
`-064`, §4.1 champs 7 à 11), `spec/SPEC-04` (`REQ-E3D-008`), `spec/SPEC-04B` (§3.4
`REQ-E3D-504/523/524`, §3.6 `REQ-E3D-560/564/567`, §3.7 `REQ-E3D-600/603`, §3.8 `REQ-E3D-640`),
`spec/SPEC-06` (`REQ-SCN-034`, `-046`, `-048`, `-055`, `-056`, `-057`, `-063`), `spec/SPEC-07`
(§3.2 `REQ-LOOP-010` à `-017`, §3.3 `REQ-LOOP-020` à `-024`, §3.4 `REQ-LOOP-030` à `-034`, §3.5
`REQ-LOOP-040` à `-045`, `REQ-LOOP-052`, `REQ-LOOP-063`, §5 vecteurs T5/T8, §6, Historique),
`spec/SPEC-09` (§2 portées, §3.1 `REQ-PERS-003` à `-024`, §3.3 `REQ-PERS-030` à `-037`, §3.4
`REQ-PERS-040` à `-042`, §3.5 `REQ-PERS-050` à `-057`, `REQ-PERS-067`, `-075`, `-082`, `-085`,
`-086`, §4.1 légende et §4.1.1 à §4.1.3, §4.5.1 à §4.5.3, §4.6), `spec/SPEC-10` (`REQ-UI-007`,
`-010` à `-014`, `-020`, `-030`, `-031`, `-037`, `-050`, `-051`, `-084`, `-085`, §5 V1),
`spec/SPEC-12` (`REQ-ACC-030`, `-031`), `spec/SPEC-v1.2-INDEX`, `spec/changes/` (`AMD-009`,
`AMD-010`, `AMD-026`, `AMD-029`, `AMD-030`, `AMD-032`, `AMD-035`, `AMD-036`, `AMD-037`, `AMD-043`,
`AMD-046`, `AMD-049`, `Q-022`, `Q-035`, `Q-037`, `Q-079`, `R2-review-AMD-046-v1.1`,
`R2-review-AMD-048`, `R2-review-AMD-049`), `impl/docs/` (miroirs `spec/` et `verify/`),
`journal/JOURNAL.md`.

Aucun accès au côté sale ; le serveur d'outils de rétro-ingénierie présent dans la session n'a pas
été utilisé ; aucune tentative de contournement du pare-feu. Aucun amendement modifié, aucun
fichier d'`impl` modifié, rien commité.

---

## 0. Contrôles transverses

**0.a — Numéros REQ : tous libres.** Chaque nouveau numéro a été cherché dans tout `spec/` **et**
dans `impl/docs/` : `REQ-SCN-076`, `REQ-SCN-077`, `REQ-E3D-644`, `-645`, `-646`, `REQ-LOOP-073`,
`-074`, `-075`, `-076`, `REQ-PERS-088`, `REQ-UI-210` à `-213` n'apparaissent **que** dans
l'amendement qui les crée. Les maxima attribués avant ce lot sont `REQ-SCN-075` (AMD-049),
`REQ-E3D-643`, `REQ-LOOP-072`, `REQ-PERS-087`, `REQ-UI-209` (AMD-010) : les quatre amendements
numérotent à la suite, sans trou ni collision. **Aucun point.**

**0.b — Étanchéité : acquise pour les quatre.** Aucun code, aucun pseudo-code suivant une structure
d'origine, aucun nom interne de classe, de méthode ou de champ privé, aucune référence à un
fichier source ou à un numéro de ligne. Les seuls noms propres employés sont admis par le
SPEC-GUIDE : réglages persistés et chemins Lua (`video.anti_alias`, `emu.game_speed`,
`video.max_queued_frames`, `ui.sort_option`), champs du format de profil visibles depuis les
données conservées (`Type`, `Ver`, `Id`, `Title`, `Maker`), options de ligne de commande
(utilisateur-visibles, même statut que le `-id` de `REQ-UI-007`), libellés d'interface, et
« 3dSen 2.0 » / « Zaxnes » / « Unity » déjà normalisés par AMD-040 et AMD-049. Une seule réserve
faible de formulation, non bloquante : **51-11**.

**0.c — Index non mis à jour (hors périmètre R2, rappel).** `spec/SPEC-v1.2-INDEX.md` ne porte pas
encore les lignes des quatorze nouvelles exigences ni celles des amendements AMD-050 à AMD-053. Le
même constat avait été fait pour AMD-048 et AMD-049 ; c'est du ressort de R1.

**0.d — Forme des quatre documents.** Les quatre suivent le gabarit court des amendements (titre,
version, besoin, exigences avec critères, questions) et portent chacun « ## 3. Questions — Aucune ».
AMD-050 et AMD-052 omettent la clause de priorité relative (« si l'exigence P2 n'est pas retenue,
alors … ») qu'AMD-046 et AMD-049 portaient à la demande des relectures précédentes ; signalé par
amendement là où cela change quelque chose (50-6 ne le demande pas, 52-7 et 52-8 si).

---

## 1. AMD-050 — Mode 2D : pixels de fond universel transparents (REQ-SCN-076)

### Verdict : **accepté avec réserves** — 1 point bloquant (50-1), 8 points en tout.

L'exigence est bien posée dans son principe : elle choisit la bonne discriminante (la **valeur**
des pixels de fond et de sprite, pas la couleur affichée), et la phrase « Un pixel opaque de même
couleur que le fond universel reste dessiné » ferme explicitement le piège de la comparaison de
couleurs. Le besoin est réel et correctement motivé par la courbure d'AMD-046. Le problème est
qu'elle exige une information que l'interface du cœur, telle que SPEC-02 la fixe, ne transporte
pas.

**50-1 — L'exigence n'est pas implémentable avec l'image de pixels telle que SPEC-02 la définit.
(BLOQUANT)**
La seule description normative de l'image de pixels est `REQ-CORE-048` : « Le cœur **PEUT**
produire en plus une image 2D de 256 × 240 pixels (**indices de couleur NES**) […] Cette image
n'est pas exigée. » `Q-022` (réponse R0) et `AMD-046` §2 fondent le rectangle du mode 2D sur cette
image. Or une image d'indices de couleur ne permet **pas** de distinguer les deux cas que
REQ-SCN-076 oppose : un pixel transparent (fond universel faute de pixel opaque) et un pixel
opaque dont la couleur est celle du fond universel portent **le même indice**. L'exigence est donc
intestable et non implémentable sans deviner l'interface.
*Correction proposée* : amender SPEC-02 dans le même lot. Au choix, (a) `REQ-CORE-048` produit, à
côté de l'image, un **masque d'opacité** de 256 × 240 bits (1 = au moins un pixel opaque : pixel de
fond ≠ 0 avec le fond activé, ou pixel de sprite ≠ 0 avec les sprites activés, hors colonnes
masquées) ; ou (b) l'image passe à un octet par pixel dont une valeur réservée (par exemple 0xFF,
hors des 64 indices NES) signifie « transparent ». Déclarer la sortie retenue dans le contrat du
rappel vidéo (`REQ-CORE-030` énumère le contenu d'un lot) et donner son critère d'acceptation côté
cœur. Sans ce choix écrit, le côté propre devra ouvrir un ticket Q et deviner.

**50-2 — Priorité incohérente entre l'exigence et sa source. (non bloquant)**
`REQ-CORE-048` est **P3** et « n'est pas exigée », alors que `REQ-SCN-055` lu avec `Q-022` rend
l'image de pixels obligatoire pour le mode 2D et que REQ-SCN-076 (P2) légifère dessus. L'écart
préexiste à AMD-050 (il date d'AMD-009) mais cet amendement est le premier à poser une exigence
normative sur le **contenu** de cette image.
*Correction proposée* : profiter de la correction 50-1 pour écrire que l'image de pixels et son
information d'opacité sont **exigées** (P1 ou P2) dès lors que le mode 2D est dans le périmètre, et
retirer de `REQ-CORE-048` la mention « n'est pas exigée » ou la restreindre au mode de débogage.

**50-3 — « colonnes masquées par PPUMASK » : risque de confusion avec maskX / maskY. (non bloquant)**
Les bits 1 et 2 de PPUMASK masquent les **8 pixels de gauche** (fond, sprites). `REQ-CORE-046`
définit par ailleurs `maskX`/`maskY`, bandes « peu fiables » qui s'appliquent aux **deux** bords et
qui ne sont pas un masquage matériel mais une indication destinée à SPEC-04. Le texte actuel peut
se lire comme rendant transparentes les bandes de `maskX`/`maskY`, ce qui creuserait un liseré
transparent à droite et en haut/bas de l'image.
*Correction proposée* : « … ou que le rendu correspondant est désactivé, y compris les 8 pixels de
gauche quand le bit 1 (fond) ou le bit 2 (sprites) de PPUMASK les masque. `maskX` et `maskY`
(`REQ-CORE-046`) ne rendent aucun pixel transparent. »

**50-4 — Cas du rendu entièrement désactivé non discuté. (non bloquant)**
Avec les bits 3 et 4 de PPUMASK à 0 (blanking forcé, fréquent pendant les écritures VRAM de
transition), la règle rend **toute** l'image transparente : en mode 2D le rectangle disparaît
complètement pendant ces frames, ce qui produit un clignotement du rectangle que la version actuelle
n'a pas. C'est probablement le comportement voulu (le programme d'origine n'a alors aucune
géométrie non plus), mais cela mérite d'être écrit.
*Correction proposée* : ajouter en Notes « quand le fond et les sprites sont tous deux désactivés,
le rectangle entier est transparent (la scène seule est visible) ; c'est le comportement attendu ».
Préciser aussi que le cas matériel du « fond de palette » (rendu désactivé avec l'adresse PPU dans
`$3F00`–`$3FFF`) est hors périmètre, pour éviter un ticket Q plus tard.

**50-5 — Le critère d'acceptation (auto) n'est pas atteignable tel quel. (non bloquant)**
Il demande « REQ-SCN-046 réglée sur une couleur personnalisée différente ». Or en mode 2D le profil
« 2D Mode » est rechargé à **chaque** lancement (`REQ-PERS-005`, `REQ-PERS-006`) et
`REQ-PERS-005` point 3 force `scene.use_nes_bg_color` (`Setting.BgColor`) à **vrai** quelle que soit
la valeur du profil : la couleur de fond de la scène est donc, par construction, égale au fond NES
juste après le chargement, et la transparence y est invisible (ce que les Notes disent justement).
*Correction proposée* : écrire la mise en condition : « après le chargement, poser
`scene.use_nes_bg_color` = faux et `Setting.CustomBgColor` = magenta (ou Skybox = `Color`) », de
sorte que le test soit reproductible par un tiers.

**50-6 — « ni ombre projetée » duplique AMD-046 §2. (non bloquant)**
`AMD-046` §2 fixe déjà, pour le rectangle du mode 2D : « Il ne projette pas d'ombre et n'en reçoit
pas. » Réénoncer la clause dans REQ-SCN-076 crée une seconde source normative pour le même fait.
*Correction proposée* : remplacer par un renvoi — « (le rectangle ne projette ni ne reçoit d'ombre,
`AMD-046` §2 ; la transparence n'y change rien) ».

**50-7 — « à l'éclairage près » n'est adossé à aucune exigence. (non bloquant)**
Ni `REQ-SCN-055`, ni `Q-022`, ni `AMD-046` §2 ne disent si le rectangle du mode 2D est **éclairé**
(diffus de `REQ-SCN-028`), non éclairé, ou éclairé par le modèle Standard désormais réservé au
backdrop (`AMD-049` `REQ-SCN-075`). Le §1 l'affirme en passant (« devient un écran bleu opaque,
éclairé et incurvé ») et les Notes s'en servent pour qualifier l'écart résiduel ; un tiers ne peut
pas vérifier la note.
*Correction proposée* : trancher en une phrase (« le rectangle est éclairé comme une tile,
`REQ-SCN-028` », ou « non éclairé : albédo restitué tel quel ») avec son critère, ou retirer les
mots « à l'éclairage près » et ouvrir un ticket Q.

**50-8 — Interaction avec le mélange et l'anticrénelage non fixée. (non bloquant)**
« Transparent (non dessiné) » peut s'implémenter par rejet de fragment ou par mélange alpha. Les
deux ne donnent pas le même résultat : par mélange, le rectangle tomberait dans la classe
transparente de `REQ-SCN-040` (tri, passage de profondeur) ; et avec le multi-échantillonnage
d'`AMD-032` §2 — dont AMD-052 fait désormais le défaut — un alpha-to-coverage donnerait des bords
de sprite partiellement transparents, donc un résultat **dépendant de `video.anti_alias`**.
*Correction proposée* : « le pixel transparent DOIT être obtenu par rejet du fragment, dans le
passage opaque ; le résultat NE DOIT PAS dépendre de `video.anti_alias` ».

**50-9 — Le critère [ROM] n'est pas mesurable. (non bloquant)**
« le ciel bleu n'est pas dessiné sur le rectangle ; on voit la scène derrière » suppose un jugement
visuel et une ROM commerciale. Les relectures précédentes (46-x, 49-2) ont demandé des critères
[ROM] chiffrés.
*Correction proposée* : « Skybox = `Color`, `Setting.CustomBgColor` = magenta : au pixel central
d'une zone de ciel, la couleur lue vaut la couleur personnalisée à ±2 niveaux sRGB ; les pixels de
la barre de statut (noir opaque, indice `$0F`) restent dessinés. » Garder la phrase actuelle en
Notes comme contrôle visuel.

### Conclusion AMD-050 (gate R2)

Exigence juste et utile, étanchéité acquise, numéro libre, aucune contradiction avec SPEC-06,
AMD-046 ou AMD-049. **Un seul bloquant : 50-1** — l'amendement doit emporter avec lui la
modification de SPEC-02 (information d'opacité par pixel), sans quoi REQ-SCN-076 est intestable et
non implémentable. 50-2 à 50-9 sont des précisions de rédaction et de critères, à traiter dans la
même version 1.1. Gate R2 franchi **sous réserve de 50-1**.

---

## 2. AMD-051 — Robustesse de la conversion, du chargement et des écritures (REQ-E3D-644 à 646, REQ-LOOP-073, REQ-PERS-088)

### Verdict : **accepté avec réserves — REQ-E3D-645 à retirer** ; 5 points bloquants, 12 points en tout.

L'intention (remplacer des pannes franches par des dégradations contrôlées) est bonne et conforme à
l'esprit de la spec, qui demande déjà en plusieurs endroits de ne pas interrompre l'émulation. Mais
quatre des cinq exigences entrent en concurrence avec des exigences en vigueur — dont une
contradiction frontale avec une P1 — ou s'appuient sur des notions que la spec ne définit pas.

**51-1 — REQ-E3D-645 contredit frontalement REQ-E3D-524 (P1). (BLOQUANT)**
`REQ-E3D-524` (P1) : « Le moteur **DOIT accepter** des maillages de tuile de **plus de 65 535
sommets** (index 32 bits ou découpage) », critère d'acceptation : damier 3D de 8 × 8 × 96, « 18 432
rectangles (**73 728 sommets**), **tous présents** » ; Notes : « l'original utilise des index
16 bits ; le dépassement (rarissime) y produit des faces erronées, **qu'il n'est pas demandé de
reproduire (classe D)** ». REQ-E3D-645 (P2) exige l'inverse : un maillage de plus de 65 536 sommets
« DOIT être abandonné (la tile n'est pas dessinée) », critère : « un maillage synthétique de
70 000 sommets **n'est pas dessiné** ». Les deux critères portent sur le même objet et s'excluent.
De plus la clause de sortie (« Une implémentation qui utilise des indices de 32 bits n'a pas cette
limite et satisfait l'exigence ») rend l'exigence **vide** pour toute implémentation conforme à
REQ-E3D-524 : un critère que l'implémentation de référence ne peut pas exécuter n'est pas un critère.
*Correction proposée* : **retirer REQ-E3D-645** (le numéro reste attribué avec la mention
« retirée », SPEC-GUIDE). Si l'on veut conserver une garantie, l'écrire sans plafond et sans
contredire REQ-E3D-524 : « aucun maillage de tuile, quelle que soit sa taille, NE DOIT lever
d'exception ni produire de géométrie corrompue ; une implémentation à indices 16 bits DOIT découper
le maillage (REQ-E3D-524) », avec le critère du damier de REQ-E3D-524 pour seul vecteur.
*Remarque de cadrage* : le texte dit « maillage de **tile** ». Dans SPEC-04B un maillage est celui
d'un **objet 3D d'un pattern** et les 65 536 sommets ne sont atteints que par des champs de voxels
profonds (le damier 8 × 8 × 96 de REQ-E3D-524) ; si l'intention était le maillage **agrégé** d'un
lot ou d'un tampon de rendu, il faut le dire, car ce n'est pas le même objet ni la même exigence.

**51-2 — « file d'envoi au GPU » : architecture interne, interdite et non définie. (BLOQUANT)**
L'expression « la file d'envoi au GPU est pleine » (REQ-E3D-644) et « au moment de l'envoi au GPU »
(REQ-E3D-646) n'apparaissent **nulle part** dans `spec/` en dehors d'AMD-051 : ce sont des notions
introduites par l'amendement. Le SPEC-GUIDE interdit explicitement « l'architecture interne de
l'original (découpage en classes, héritage, **threads internes**), sauf effet observable » ; une
file d'envoi de capacité finie est un mécanisme interne, pas un comportement observable, et c'est
aussi la seule formulation du lot qui laisse deviner un rouage du programme d'origine (cf. 51-11).
*Correction proposée* : énoncer les deux exigences par leur **effet**. REQ-E3D-644 : « quand la
texture d'un objet d'un pattern ne peut pas être allouée (capacités de `REQ-E3D-564` dépassées,
échec d'allocation côté pilote), le moteur NE DOIT PAS lever d'exception ni interrompre la frame ;
les tuiles de ce pattern ne sont pas dessinées tant que la texture n'est pas disponible ; la frame
suivante est produite. » REQ-E3D-646 : « un objet dont les maillages ou la texture ne sont pas
prêts n'est pas dessiné (`REQ-E3D-601`) et **DOIT** être dessiné à la première frame rendue qui
suit leur disponibilité, sans être oublié. »

**51-3 — Le critère d'acceptation de REQ-E3D-644 n'est pas applicable. (BLOQUANT)**
Il postule une « capacité d'atlas […] à **N patterns** ». La spec n'exprime nulle part la capacité
d'atlas en patterns : `REQ-E3D-564` la donne en **texels** (« jusqu'à 512 × 512 texels » par objet,
« au moins 64 × 512 × 512 texels » au total pour le profil chargé), et un pattern porte un nombre
variable d'objets de tailles variables — « N patterns » n'est donc pas une grandeur mesurable.
Le critère suppose en outre un moyen de réduire la capacité, qu'aucune exigence ne fournit.
*Correction proposée* : « avec un budget total d'atlas réduit à C texels par le harnais
(`Sen.Harness`, option dédiée), un profil dont les objets demandent plus de C texels rend les
objets logés, journalise une erreur par objet non logé, et la frame suivante est produite » ;
et ajouter l'option du harnais à la liste des moyens de vérification.

**51-4 — REQ-LOOP-073 : « n'avance plus » n'est pas défini, donc le critère est intestable. (BLOQUANT)**
Rien ne dit ce qui doit avancer. La spec dispose pourtant d'une mesure d'avancement observable :
`REQ-E3D-600` définit l'avancement exposé à l'indicateur de chargement
(`(objets dont le champ et les maillages sont calculés + objets dont les maillages sont transmis) /
(2 × objets à construire)`), et `REQ-LOOP-021` point 6 impose de le consulter « au moins toutes les
100 ms ». Sans renvoi, « un chargement artificiellement bloqué » ne décrit pas une entrée de test.
*Correction proposée* : « si l'avancement exposé à l'indicateur de chargement (`REQ-E3D-600`) ne
change pas pendant 10 s, ou si une étape de `REQ-LOOP-021` ne se termine pas dans les 10 s … » ;
critère : « le harnais bloque l'étape 6 de `REQ-LOOP-021` → notification à 10 s ± 1 s ».

**51-5 — REQ-LOOP-073 : ni texte exact de notification, ni état cible nommé. (BLOQUANT)**
Deux manques, chacun en contradiction avec une règle en vigueur.
(a) `REQ-LOOP-023` fixe **mot pour mot** les messages d'échec de lancement (`Corrupted zip file`,
`Invalid rom file.`, …) et `AMD-036` exige que tout texte fixé par SPEC-10/SPEC-09 devienne une
entrée anglaise de la table des textes **avec** sa traduction française. AMD-051 demande « une
notification d'erreur (SPEC-10) » sans texte : le texte serait inventé côté implémentation et la
table des deux langues (`AMD-035` §3, test « mêmes clés, aucune valeur vide ») serait incomplète.
(b) « revenir à un état utilisable (**bibliothèque**) » n'est pas un état de SPEC-07 et n'existe
plus comme écran : `REQ-LOOP-023` nomme l'**état Arrêté**, et `AMD-010` `REQ-UI-207` remplace la
bibliothèque en plein écran par l'écran d'accueil quand aucune ROM n'est chargée.
*Correction proposée* : ajouter le texte exact et sa traduction (p. ex. `Loading timed out.` /
« Délai de chargement dépassé. », gravité normale, 4 s, à inscrire dans la table d'`AMD-035`), et
écrire « le programme DOIT effacer le jeu courant, remettre l'indicateur Chargement à faux et
passer à l'état **Arrêté** (`REQ-LOOP-023`), l'écran d'accueil de `REQ-UI-207` étant alors
affiché ».

**51-6 — REQ-PERS-088 duplique REQ-PERS-021 pour le fichier de réglages. (non bloquant)**
`REQ-PERS-021` (P1) dit déjà : « Toute écriture DOIT être **atomique** : écriture d'un fichier
temporaire dans le même dossier, puis remplacement », avec le critère « tuer le processus pendant
une écriture → le fichier reste lisible (ancienne ou nouvelle version complète) ». REQ-PERS-088
réénonce la même règle et le même critère : deux sources normatives pour un comportement, qui
divergeront à la première reformulation.
*Correction proposée* : « L'écriture atomique de `REQ-PERS-021` DOIT s'appliquer de la même façon
à : le fichier de bibliothèque, l'index des slots, le fichier de bindings et les fichiers de
savestate. » Et ne garder qu'un critère, celui des fichiers nouvellement couverts.

**51-7 — La liste de fichiers de REQ-PERS-088 est incomplète. (non bloquant)**
`REQ-PERS-024` énumère les fichiers persistants : « réglages, bibliothèque, **index des slots**,
**bindings** ». AMD-051 cite les réglages, la bibliothèque et les savestates, et oublie précisément
`saves/index.json` (§4.5.3, « métadonnées des 37 slots ») et les bindings — alors que
`REQ-PERS-055` montre que l'index des slots est le fichier dont la corruption coûte le plus cher
(« à défaut, tous les slots sont vides »). À signaler aussi : la copie `<nom>.v<N>.bak` de
`REQ-PERS-024` et le renommage `*.corrupt-<horodatage>` de `REQ-PERS-022`/`REQ-PERS-037` doivent
rester cohérents avec le remplacement atomique (ordre des opérations).
*Correction proposée* : « tout fichier persistant au sens de `REQ-PERS-024`, plus les fichiers de
savestate (`REQ-PERS-056`) ».

**51-8 — REQ-E3D-646 est pour l'essentiel déjà exigé, et son critère contredit une recommandation.
(non bloquant)**
`REQ-E3D-601` (P1) dit déjà qu'un objet dont les maillages ne sont pas prêts n'est pas dessiné, que
sa shape reste reconnue, et que « pour un pattern créé pendant le jeu, les maillages **DEVRAIENT**
être prêts au plus tard **2 frames émulées** après sa création », avec le critère « une shape
nouvelle apparaît en 3D dans les 2 frames ». Le critère de REQ-E3D-646 met en scène un retard de
**10 frames**, c'est-à-dire un cas que REQ-E3D-601 déconseille : pas de contradiction formelle
(DEVRAIT), mais un vecteur qui verrouille un comportement dégradé sans dire qu'il l'est. Le seul
contenu réellement nouveau est le diagnostic à 600 frames.
*Correction proposée* : réduire REQ-E3D-646 au diagnostic, renvoyer à `REQ-E3D-601` pour la mise en
attente, et justifier la constante (« 600 frames ≈ 10 s en NTSC, même ordre que les 10 s de
`REQ-LOOP-073` », le SPEC-GUIDE demandant la justification des constantes). Ajouter dans le critère
que le retard de 10 frames est un cas de test, non un comportement visé.

**51-9 — « une erreur journalisée une fois par pattern » n'est pas bornée. (non bloquant)**
Les patterns sont créés **pendant le jeu** (patterns temporaires, `REQ-E3D-008`,
`REQ-E3D-453`) et rien ne borne leur nombre sur une partie : avec une ROM pathologique, la règle
« une fois par pattern » peut produire des milliers de lignes de journal, ce qui est en soi une
dégradation.
*Correction proposée* : « au plus une fois par pattern et au plus 100 fois par partie, puis une
ligne de résumé périodique ».

**51-10 — Tension non traitée avec REQ-E3D-603 et REQ-E3D-564. (non bloquant)**
`REQ-E3D-603` (P2) : « Le nombre d'objets et de tuiles dessinés par frame NE DOIT PAS être limité
par une capacité fixe » (critère : 1 500 shapes sur 3 000 tuiles « se dessine **entièrement** »).
`REQ-E3D-564` (P2) : « Le comportement au-delà n'est pas spécifié mais NE DOIT PAS interrompre
l'émulation. » REQ-E3D-644 est en réalité le **raffinement** de cette dernière phrase et ne doit pas
se lire comme une autorisation à plafonner la frame. Aucune des deux n'est citée.
*Correction proposée* : ouvrir REQ-E3D-644 par « (précise la phrase “le comportement au-delà n'est
pas spécifié” de `REQ-E3D-564`, sans rouvrir `REQ-E3D-603`) ».

**51-11 — §1 décrit des mécanismes plutôt que des effets (LEAK faible). (non bloquant)**
« un pattern qui ne trouve plus de place n'est simplement pas dessiné, un maillage trop grand est
abandonné au lieu de corrompre la géométrie, un pattern pas encore prêt est **envoyé plus tard au
lieu d'être oublié** » : les deux premières clauses sont des effets observables (admis), la
troisième — avec la « file d'envoi » de 51-2 — décrit un rouage. Pas de fuite au sens du plan
(aucun nom interne, aucune structure de code), mais la formulation sort du périmètre « ce que le
programme fait ».
*Correction proposée* : reformuler en effets (« la tuile apparaît dès que son maillage et sa
texture sont disponibles, et n'est jamais perdue »).

**51-12 — Le second critère de REQ-LOOP-073 n'est pas une assertion. (non bloquant)**
« puis la bibliothèque répond » n'est pas mesurable.
*Correction proposée* : « puis l'état est Arrêté et une commande du modèle de commandes
(`REQ-UI-205`) s'exécute dans la seconde qui suit ».

### Conclusion AMD-051 (gate R2)

Lot utile mais à retravailler. **REQ-E3D-645 doit être retirée** : elle contredit une P1 en vigueur
et son critère est inexécutable pour l'implémentation de référence (51-1). **REQ-E3D-644** et
**REQ-E3D-646** doivent être réécrites en termes observables, sans la « file d'envoi au GPU »
(51-2), et le critère de REQ-E3D-644 reformulé en texels (51-3). **REQ-LOOP-073** doit définir
l'absence d'avancement (51-4) et porter le texte exact de sa notification ainsi que l'état cible
(51-5). **REQ-PERS-088** est acceptable telle quelle mais fait doublon avec REQ-PERS-021 et oublie
deux fichiers (51-6, 51-7). Étanchéité acquise, un LEAK faible de formulation (51-11), numéros
libres. Gate R2 franchi **sous réserve des cinq bloquants 51-1 à 51-5**.

---

## 3. AMD-052 — Cadence, file de frames GPU, anticrénelage par défaut (REQ-LOOP-074, 075, REQ-SCN-077, AMD-032 §2)

### Verdict : **accepté avec réserves** — 6 points bloquants, 13 points en tout.

Les trois sujets sont indépendants et de qualité inégale. REQ-LOOP-074 est juste sur le fond et son
arithmétique se vérifie (cadence nominale 60 images/s par DEC-R2-T3 → 60 frames dues sur 1 s, 2
exécutées, **58 perdues** : le vecteur T5 amendé est exact), mais l'amendement ne va pas au bout de
ce qu'il annonce amender. REQ-LOOP-075 déclenche un effet de bord sur l'avance rapide. REQ-SCN-077
est placée dans le mauvais document et contredit une P1 de portabilité. L'amendement d'AMD-032 §2
vise un texte qui ne contient pas la valeur qu'il prétend changer.

**52-1 — REQ-LOOP-012 n'est amendée qu'à moitié : son critère d'acceptation reste faux. (BLOQUANT)**
L'en-tête annonce « **Amende** SPEC-07 REQ-LOOP-012/013 et le vecteur T5 », mais seul T5 est
réécrit. Le critère de `REQ-LOOP-012` reste : « bloquer la boucle 500 ms à 60 Hz → à la reprise, **au
plus 5 frames** sont exécutées en rafale […] ; le compteur de frames accuse **≈ 25 frames** de
moins ». Avec la nouvelle limite ce sont « au plus 2 » et « ≈ 28 ». Une P1 conserverait donc un
critère que l'implémentation conforme échouerait.
*Correction proposée* : donner le texte amendé de REQ-LOOP-012 (« dans la limite de **2** frames de
retard », « Le retard au-delà de 2 frames DOIT être abandonné ») **et** de son critère (« au plus
2 frames en rafale ; ≈ 28 frames de moins »), comme AMD-049 a donné le texte amendé de REQ-SCN-048.

**52-2 — La grandeur bornée n'est pas la même dans les deux rédactions. (BLOQUANT)**
`REQ-LOOP-012` borne le **retard accumulé** : « le nombre de frames dues et non encore exécutées ne
dépasse jamais 5 ». REQ-LOOP-074 borne « au plus 2 frames de **rattrapage d'affilée** », sans dire
si la frame normalement due compte parmi les 2. Les deux lectures diffèrent d'une frame (rafale de
2 ou de 3) et seule la lecture « retard accumulé ≤ 2 » redonne les « ≈ 58 frames perdues » du
vecteur T5. Un tiers ne peut pas choisir.
*Correction proposée* : reprendre mot pour mot la grandeur de REQ-LOOP-012 : « le nombre de frames
dues et non encore exécutées NE DOIT jamais dépasser **2** ; le surplus est abandonné et l'horloge
de cadence réinitialisée à l'instant présent ».

**52-3 — REQ-LOOP-075 casse l'avance rapide de REQ-LOOP-016. (BLOQUANT)**
`REQ-LOOP-016` arme l'avance rapide par salves de **30 frames émulées**, réarmées « dès qu'elle
s'achève » tant que la hotkey est maintenue (une demande à **chaque image affichée**,
`REQ-INP-103`). Un maintien de la touche produit donc un changement d'état d'avance rapide toutes
les 30 frames émulées (≈ 20 ms × 30 ≈ 0,5 s, soit deux changements — fin puis réarmement — par
salve). Réinitialiser l'horloge de cadence « sans rattrapage » à chaque changement abandonne le
reliquat de retard 2 à 4 fois par seconde pendant tout le maintien, ce qui rend inverifiables le
critère de `REQ-LOOP-016` (« maintenir 5 s → ≈ 450 frames émulées ±5 % ») et l'histogramme
d'intervalles de `REQ-LOOP-011`.
*Correction proposée* : exclure explicitement le cycle de `REQ-LOOP-016` — « une modification du
réglage `emu.game_speed` (y compris par hotkey ±1) DOIT réinitialiser l'horloge ; l'armement,
l'expiration et le réarmement d'une avance rapide (`REQ-LOOP-016`) NE DOIVENT PAS la
réinitialiser » — ou, si l'on veut couvrir aussi l'entrée/sortie d'avance rapide, limiter à
« l'entrée en avance rapide depuis un état sans avance rapide **et** la sortie durable (aucune
demande depuis 1 s) », avec le critère correspondant.

**52-4 — REQ-SCN-077 est dans le mauvais document et contredit REQ-SCN-057 (P1). (BLOQUANT)**
`REQ-SCN-057` (P1) : « Le moteur de rendu DOIT dessiner dans une **cible fournie par l'hôte** à
chaque frame […] sans supposer le framebuffer par défaut, sans état global de processus » ; la même
contrainte figure dans les contraintes transverses du SPEC-GUIDE (« un rendu dans une cible fournie
par l'hôte »). Borner le nombre de frames **présentées** que le GPU peut garder en attente est une
affaire de présentation, donc du frontend desktop : le moteur, dans un hôte libretro, ne contrôle
ni la chaîne d'échange ni la présentation (`REQ-PERS-086`). Classer l'exigence en `REQ-SCN-` la
place dans le périmètre portable, où elle n'est pas tenable.
*Correction proposée* : déplacer l'exigence dans SPEC-07 (numéro `REQ-LOOP-`) ou SPEC-09 comme
réglage du frontend desktop, écrire « sans objet en libretro, où la présentation est celle de
l'hôte », et ajouter `video.max_queued_frames` à la liste « Non exposés en libretro » de SPEC-09
§4.6, aux côtés de `video.vsync` et `video.frame_rate` (voir 52-10).

**52-5 — Le critère d'acceptation de REQ-SCN-077 n'est pas vérifiable. (BLOQUANT)**
« la mesure du temps entre la fin du rendu et la fin de l'exécution GPU d'une frame montre
qu'aucune frame n'est en attente au début du rendu suivant » : ni seuil, ni définition de « frame en
attente », ni verdict binaire. Deux implémenteurs mesureront deux choses différentes.
*Correction proposée* : un critère observable et conforme à `REQ-SCN-056` (GL 3.3 core) : « un objet
de synchronisation créé après la dernière commande de la frame *n* (`glFenceSync` +
`glClientWaitSync` à délai nul) DOIT être signalé **avant** la première commande de la frame
*n + 1* quand `video.max_queued_frames` = 1 ; à 2, ce n'est pas exigé. » Plus, si l'on veut
l'effet utile : « la latence mesurée entre la fin de l'émulation d'une frame et sa présentation est
plus courte à 1 qu'à 2, sur le banc de `REQ-ACC-030` ».

**52-6 — Le défaut d'anticrénelage est amendé au mauvais endroit : l'amendement est inopérant.
(BLOQUANT)**
La valeur par défaut `0` de `video.anti_alias` est fixée par la **ligne de SPEC-09 §4.1.1**
(colonne « Défaut »), pas par AMD-032. Et `AMD-032` §2 dit explicitement l'inverse de ce qu'AMD-052
veut lui faire dire : « Les libellés **et le domaine** de SPEC-09 §4.1.1 sont **inchangés** : la
persistance, l'aller-retour des profils (champ 32), la table libretro et l'API Lua ne bougent
pas. » « AMD-032 §2 amendé » ne change donc **rien** de normatif, et un implémenteur lisant SPEC-09
garde le défaut 0.
*Correction proposée* : amender la **ligne `video.anti_alias` de SPEC-09 §4.1.1** (Défaut : `0` →
`1`), en reproduisant la ligne complète comme AMD-046 et AMD-043 le font pour leurs réglages ;
mentionner que `REQ-PERS-017` (« sans effet dans l'original ») reste inchangée ; et vérifier la
table §4.6 (la ligne §4.1 porte une discrétisation `None/FFAA/SMAA`, donc le réglage est exposé en
libretro par la règle du §4.6 : son défaut y passe aussi à `FFAA`).

**52-7 — L'amendement d'AMD-032 §2 n'a pas d'identifiant REQ. (non bloquant)**
« **AMD-032 §2 amendé** (P3) » est un énoncé normatif muni de son propre critère d'acceptation, mais
sans `REQ-<MOD>-NNN`, alors que le SPEC-GUIDE impose l'identification de chaque exigence et sa
traçabilité. La priorité est par ailleurs incohérente : P3 pour un changement qui modifie l'image
produite à l'installation, alors que l'exigence porteuse (`REQ-PERS-017`) est P2.
*Correction proposée* : soit le présenter comme un simple amendement de table (ligne SPEC-09
réécrite, sans priorité propre, voir 52-6), soit lui attribuer `REQ-PERS-089` en P2 ou P3 assumé,
avec une clause de priorité relative (« si elle n'est pas retenue, le défaut reste 0 »).

**52-8 — Divergence non étiquetée (D). (non bloquant)**
Le programme d'origine force son anticrénelage à « désactivé » (`REQ-PERS-017`) et son défaut est 0.
Livrer MSAA 4× activé est une divergence assumée supplémentaire, et la seule des trois clauses
d'AMD-052 qui change l'image à la première ouverture. `AMD-032` §2 porte, lui, la mention
« (divergence D) ».
*Correction proposée* : ajouter **(D)** à la clause et une phrase de justification (le
multi-échantillonnage ne touche que les arêtes géométriques, argument déjà écrit dans AMD-032 §2).

**52-9 — « groupe Rendu » : référence cassée. (non bloquant)**
La ligne SPEC-09 de `video.max_queued_frames` place le réglage dans un « (groupe Rendu) » de la
fenêtre d'options. Ce groupe n'existe pas : `AMD-010` `REQ-UI-202` fixe les groupes de la fenêtre
d'options (« Vidéo, Caméra, Lumière et ombres, Audio, Émulation, Interface ») et `AMD-043` §6 y a
ajouté « Géométrie ». « Rendu » n'est qu'une **catégorie libretro** (SPEC-09 §4.6 :
`3dsen_render_mesh_algo`, `3dsen_render_shadow`).
*Correction proposée* : colonne UI → « groupe **Vidéo** » ; si la catégorie libretro « Rendu » était
visée, l'écrire dans la ligne §4.6 (voir 52-10).

**52-10 — Ligne libretro manquante et préfixe non traité. (non bloquant)**
La colonne Libretro porte « option 1/2 » sans clé, sans valeurs, sans catégorie, et aucune ligne
n'est ajoutée au §4.6 — alors qu'`AMD-046` §3 avait fait exactement cela, en relevant que le
préfixe est `zaxnes_` (`AMD-029`) et non le `3dsen_` encore écrit dans SPEC-09 §4.6.
*Correction proposée* : cohérent avec 52-4, mettre « — » dans la colonne Libretro et ajouter
`video.max_queued_frames` à la liste des non exposés ; si l'on veut malgré tout l'exposer, écrire la
ligne `zaxnes_video_max_queued_frames | 1, 2 | 2 | Rendu`.

**52-11 — Interaction de REQ-SCN-077 avec la cadence d'émulation non fixée. (non bloquant)**
À 1, « le programme attend, avant de commencer le rendu d'une frame, que la frame précédente soit
terminée par le GPU ». `REQ-LOOP-010` (P1) exige que les frames émulées tournent « indépendamment de
la cadence de rendu et de la synchronisation verticale » ; si l'attente se fait dans la même boucle,
elle produit du retard, donc — avec la nouvelle limite de 2 — des abandons de frames systématiques
sur machine lente. `REQ-LOOP-014` (saut de rendu) n'est pas discutée non plus.
*Correction proposée* : ajouter « l'attente NE DOIT PAS décaler la cadence des frames émulées
(`REQ-LOOP-010`) ; elle ne constitue pas un retard au sens de `REQ-LOOP-074` » (ou l'inverse,
explicitement), et « à 1, le saut de rendu de `REQ-LOOP-014` reste applicable ».

**52-12 — Le vecteur T5 amendé n'est pas donné sous sa forme de table. (non bloquant)**
SPEC-07 §5 est une table (`| T5 | blocage de 1 s de la boucle, NTSC | au plus 5 frames rattrapées ;
≈ 55 frames perdues |`). AMD-052 le reformule dans le critère de REQ-LOOP-074 au lieu de réécrire
la ligne, et ne justifie pas le « ≈ 58 ».
*Correction proposée* : donner la ligne (`| T5 | blocage de 1 s de la boucle, NTSC | au plus
2 frames rattrapées ; ≈ 58 frames perdues |`) et, en Notes, l'arithmétique (60 images/s nominales,
DEC-R2-T3 → 60 dues, 2 exécutées, 58 abandonnées). *Vérifié : le chiffre est exact.*

**52-13 — Étanchéité : aucune fuite. (non bloquant, pour mémoire)**
Le §1 décrit des comportements observables de 3dSen 2.0 (limite de rattrapage, recalage à un
changement de vitesse, réglage de file de frames, anticrénelage activé) et un réglage
utilisateur-visible ; c'est admis, et la limite précédente de 5 était déjà écrite et justifiée dans
`REQ-LOOP-012`. Aucun nom interne, aucun code.

### Conclusion AMD-052 (gate R2)

Les trois sujets devraient être traités séparément, car ils ne sont pas au même niveau de maturité.
**REQ-LOOP-074** est acceptable sur le fond (arithmétique vérifiée) mais doit emporter le texte
**et** le critère amendés de REQ-LOOP-012 (52-1) et fixer la grandeur bornée (52-2).
**REQ-LOOP-075** doit exclure le cycle de l'avance rapide, sans quoi elle rend deux exigences P1/P2
invérifiables (52-3). **REQ-SCN-077** doit changer de document et de portée (52-4) et recevoir un
critère exécutable (52-5). **L'amendement du défaut d'anticrénelage est inopérant tel qu'écrit** et
doit viser la ligne de SPEC-09 §4.1.1 (52-6). Étanchéité acquise, numéros libres. Gate R2 franchi
**sous réserve des six bloquants 52-1 à 52-6**.

---

## 4. AMD-053 — Reprise, favoris, recherche, inspection des imports, ligne de commande (REQ-LOOP-076, REQ-UI-210 à 213)

### Verdict : **accepté avec réserves** — 7 points bloquants, 16 points en tout.

Les cinq fonctions sont légitimes et bien choisies, et la précaution d'en-tête (« **sans** reprendre
la présentation de la nouvelle interface de 3dSen ») est exactement la bonne. Mais l'amendement se
pose « en complément » de SPEC-07, SPEC-09, SPEC-10 et AMD-010 là où il les **contredit**, et il
laisse hors spec tout le modèle de persistance qu'il suppose — alors que SPEC-09 est exhaustive par
construction.

**53-1 — REQ-LOOP-076 contredit REQ-LOOP-052 (P1). (BLOQUANT)**
`REQ-LOOP-052` : « une demande de sortie […] DOIT être exécutée **sans confirmation** et **sans
sauvegarde automatique de la partie** ». REQ-LOOP-076 exige d'écrire, à cette même sortie, « un
savestate complet (`REQ-LOOP-040`) avec sa vignette » : c'est précisément une sauvegarde automatique
de la partie. L'en-tête se contente d'un « complète SPEC-07 ».
*Correction proposée* : donner le texte amendé de REQ-LOOP-052 — « sans sauvegarde automatique dans
les slots numérotés ni dans le slot rapide ; l'emplacement “reprise” de `REQ-LOOP-076` est écrit
**(D)** » — et fixer l'ordre des opérations, que REQ-LOOP-076 ne donne pas : le hook Lua de
sauvegarde **avant** la capture (`REQ-LOOP-041`, `REQ-LOOP-063`), puis le hook Lua de fin, puis
l'écriture des réglages et des métadonnées de slots (`REQ-LOOP-052`). Préciser aussi comment la
condition de `REQ-LOOP-041` (« l'état n'est pas Arrêté et il y a un jeu courant ») et sa clause
« écrire le savestate **entre deux frames émulées** » s'appliquent quand la boucle est déjà arrêtée.

**53-2 — L'emplacement « reprise » n'a aucun modèle de persistance. (BLOQUANT)**
SPEC-09 fixe exhaustivement les emplacements : `REQ-PERS-050` (P1) « **36 slots numérotés** `1` à
`36` et **un slot rapide** `quick`, **communs à tous les jeux** », §4.5.3 `saves/index.json` =
« métadonnées des **37** slots ». AMD-053 ajoute un 38ᵉ emplacement sans amender aucune de ces
lignes, et sans traiter : l'exclusion des listes Load/Save (le slot rapide a sa clause explicite,
`REQ-PERS-052`), la cohérence au démarrage (`REQ-PERS-055` : slot d'un jeu absent → vidé), la
suppression d'un jeu (`REQ-PERS-042` : « effacer **tous** les slots associés à ce jeu »), ni la
**vignette** — `REQ-PERS-051` exige « la capture prise à la pause, 256 × 192 px RGB », capture qui
n'existe pas nécessairement si l'on quitte depuis l'état En cours (`REQ-LOOP-006`).
*Correction proposée* : donner les lignes SPEC-09 : `REQ-PERS-050` amendée (« 36 slots numérotés, un
slot rapide `quick` et un emplacement `resume` »), l'entrée correspondante de `saves/index.json`
(identifiant, `gameId`, vignette, date), le nom de fichier du savestate, l'exclusion des listes
Load/Save, l'effacement avec le jeu (`REQ-PERS-042`) et le vidage à la réconciliation
(`REQ-PERS-055`), ainsi que la source de la vignette à la sortie depuis En cours.

**53-3 — REQ-UI-210 et REQ-UI-211 contredisent REQ-UI-030 (P1). (BLOQUANT)**
`REQ-UI-030` se termine par : « **Il n'y a ni champ de recherche ni filtre.** » AMD-053 exige un
filtre « Favoris seulement » et un champ de recherche, en se déclarant seulement « complète […]
REQ-UI-030 ». Par ailleurs `AMD-010` `REQ-UI-201` fournit **déjà** une « recherche par titre » dans
la boîte de sélection du profil : REQ-UI-211 crée une seconde recherche sans le dire.
*Correction proposée* : amender explicitement `REQ-UI-030` (« la phrase “Il n'y a ni champ de
recherche ni filtre” est remplacée par… »), marquer l'ajout **(D)** (l'original n'a ni l'un ni
l'autre), et rattacher REQ-UI-211 à la recherche existante de `REQ-UI-201` plutôt que d'en exiger
une nouvelle.

**53-4 — REQ-UI-210 s'appuie sur une présentation que AMD-010 a neutralisée. (BLOQUANT)**
« commande de la **tuile** sélectionnée », « une **tuile** favorite porte un marqueur visible » et
« le tri `Sort by` (**REQ-UI-014**) » désignent la grille de tuiles paginée et la liste déroulante
de SPEC-10, qu'`AMD-010` §4 rend « informatives, **P3** » (« La présentation à onglets, les grilles
paginées et les jaquettes en tuiles de SPEC-10 : informatives, P3 »). Une exigence **P2** ne peut
pas reposer sur des éléments de présentation P3-informatifs, d'autant que l'amendement affirme
lui-même « Zaxnes garde son interface (AMD-010) ».
*Correction proposée* : exprimer les favoris sur le **modèle de commandes** d'`AMD-010`
`REQ-UI-205` (`favorite.toggle`, `library.filter_favorites`, ordre de tri) et sur la boîte de
sélection de jeu/profil de `REQ-UI-201`, en laissant libre la forme du marqueur : « l'entrée
favorite DOIT être distinguée visuellement, forme libre ». Les critères d'acceptation peuvent alors
être automatiques, comme le prévoit AMD-010 §5.

**53-5 — Les favoris et le nouveau tri n'ont ni représentation persistée ni ordre défini. (BLOQUANT)**
Trois manques :
(a) `library.json` est entièrement fixé par SPEC-09 §4.5.2 (`id`, `title`, `romPath`, `lastPlayed`,
`cover`) et `REQ-PERS-024` impose, pour tout nouveau champ, l'incrément de `schemaVersion`, la
migration à la lecture et la copie `<nom>.v<N>.bak`. AMD-053 dit seulement « persisté par entrée
(SPEC-09, fichier de bibliothèque) ».
(b) L'option « Favoris d'abord » suppose d'élargir le domaine de `ui.sort_option`
(`| ui.sort_option | SortOption | 36 | entier | 0 | 0 A-Z, 1 Recent | U | … |`). Or ce réglage porte
le **champ de profil 36** : une valeur 2 voyagerait dans les profils réécrits, hors du domaine que
le format d'origine connaît — point d'interopérabilité à trancher explicitement.
(c) `REQ-PERS-033` fixe les deux ordres (« Recent » : `lastPlayed` décroissant puis titre ; « A-Z » :
titre croissant) et `REQ-UI-014` précise que le choix « s'applique aux **trois** grilles » — donc
aussi aux listes de slots, dont `REQ-PERS-057` donne l'ordre. « Favoris d'abord » n'a aucun sens
pour des slots, et l'ordre **à l'intérieur** des favoris n'est pas défini.
*Correction proposée* : ajouter le champ `favorite` (booléen, défaut faux) au §4.5.2 avec
`schemaVersion` 2 et la migration ; amender la ligne `ui.sort_option` (domaine « 0 A-Z, 1 Recent,
2 Favorites first », et dire ce qu'on écrit dans le champ 36 — par exemple « la valeur 2 n'est jamais
écrite dans un profil, elle retombe sur 0 ») ; amender `REQ-PERS-033` (« Favoris d'abord » = favoris
puis non-favoris, chaque groupe ordonné par le critère précédent — préciser lequel) ; et écrire que
l'option est **sans effet sur les listes de slots** (`REQ-PERS-057`). Ajouter enfin « en libretro,
la bibliothèque est gérée par l'hôte (`REQ-PERS-086`) : favoris et recherche sont sans objet ».

**53-6 — REQ-UI-212 invente un format que SPEC-03 n'a pas. (BLOQUANT)**
Un profil est un **message protobuf nu** : `REQ-PROF-001` (« le lecteur DOIT accepter les champs
dans **n'importe quel ordre** et DOIT ignorer un champ inconnu »). Il n'y a donc **ni en-tête, ni
signature**, et « en lire l'en-tête » n'a pas de sens. Les discriminantes qui existent réellement
sont : le champ `Type` (doit valoir `"3dn"`), le champ `Ver` (`REQ-PROF-021` : `"2"` → V2, `"1"`
refusée, autre → format courant) et le champ `Id` (`REQ-PROF-063`, `REQ-PERS-041` point 1 : absent
ou vide → refus) ; `REQ-PROF-033` énumère les cas d'invalidité et impose l'échec **en bloc**.
« signature absente » ne correspond à rien, et « données tronquées » se confond avec « protobuf mal
formé ».
*Correction proposée* : nommer les causes réelles — protobuf mal formé ou fichier illisible ;
`Type` ≠ `"3dn"` ; `Ver` = `"1"` ou non analysable ; `Id` absent ou vide — et renvoyer à
`REQ-PROF-021`, `REQ-PROF-033` et `REQ-PERS-041`. Remplacer « en lire l'en-tête » par « le lire
selon SPEC-03 sans charger patterns ni scripts ».

**53-7 — Les quatre messages d'import contredisent les textes déclarés normatifs. (BLOQUANT)**
`REQ-PERS-041` point 1 désigne « le texte exact défini par **SPEC-10 REQ-UI-084, seule source
normative** », et `REQ-UI-084` fixe trois textes au caractère près : `This file is not a 3dn file`,
`3dn format version <v>  is not supported` (avec ses **deux espaces**), `<chemin>has an obsolate or
wrong profile format. This profile is not compatible with the current version of 3dSen.` (sans
espace après le chemin). `AMD-036` en a fait les entrées **anglaises** de la table des textes, qui
doivent rester identiques caractère par caractère (« y compris les particularités de l'original,
comme les deux espaces de REQ-UI-084 »). AMD-053 les remplace par `Unreadable file`, `Not a profile
(missing signature)`, `Unknown profile version`, `Truncated profile`, sans amender ni REQ-UI-084 ni
REQ-PERS-041 : deux jeux de textes normatifs pour le même événement.
*Correction proposée* : au choix, (a) mapper les causes de 53-6 sur les **trois textes existants**
(`Type` ≠ `"3dn"` → `This file is not a 3dn file` ; `Ver` refusée → `3dn format version <v>  is not
supported` ; protobuf mal formé ou `Id` absent → le texte `has an obsolate…`), auquel cas les
libellés nouveaux disparaissent ; ou (b) amender explicitement `REQ-UI-084` et `REQ-PERS-041`
point 1, marquer **(D)** (REQ-UI-084 dit « DEVRAIENT être conservés », donc c'est permis mais doit
être écrit), et inscrire les quatre couples anglais/français dans la table d'`AMD-035`.

**53-8 — La justification de REQ-UI-212 est inexacte. (non bloquant)**
« au lieu d'échouer en silence » : `REQ-PERS-041` point 1 exige déjà une notification de 4 s à
l'import, et `REQ-PERS-040` exige déjà d'ignorer **et de journaliser** un `.3dn` illisible trouvé
dans le dossier. Ce qui est réellement nouveau, c'est de **nommer la cause** et d'afficher titre et
auteur en cas de succès.
*Correction proposée* : reformuler le §1 et l'exigence autour de « préciser la cause du refus »,
pour ne pas donner l'impression d'une lacune à combler là où une règle existe.

**53-9 — « son titre et son auteur » : champs et cas d'absence non précisés. (non bloquant)**
`Title` (champ 8) et `Maker` (champ 9, « auteur du profil ») sont **facultatifs** (`REQ-PROF-022`),
et pour un profil **embarqué** les métadonnées affichées viennent du catalogue (`REQ-PROF-060`), pas
des champs. `REQ-PERS-034` donne la règle de repli pour le titre (nom du fichier sans extension).
*Correction proposée* : « titre = `Title`, à défaut le nom du fichier sans extension
(`REQ-PERS-034`) ; auteur = `Maker`, omis de la notification s'il est vide ».

**53-10 — `-slot=<n>` « 1 à 36 » contre REQ-UI-208. (non bloquant)**
`AMD-010` `REQ-UI-208` expose « **10** emplacements numérotés plus l'état rapide […] les
emplacements au-delà de 10 ne sont **pas exposés** **(D)** ». La ligne de commande en exposerait 36 :
pas une contradiction logique (le modèle de persistance en gère bien 36, `REQ-PERS-050`), mais une
incohérence d'exposition qu'il faut assumer par écrit.
*Correction proposée* : soit aligner sur `1` à `10` plus `quick`, soit écrire « la ligne de commande
n'est pas tenue par la limite d'exposition de `REQ-UI-208` et accepte 1 à 36 ».

**53-11 — Convention de préfixe des options non énoncée. (non bloquant)**
`REQ-UI-007` emploie `-id=` (tiret simple, repris de l'original) ; les options propres à Zaxnes
emploient le double tiret (`--help`, `--list-profiles`, `--import-3dsen`, `AMD-032` §5 ; `--fov`,
`--vsync`, `--skybox`, … `AMD-026`). Les trois nouvelles options prennent un tiret simple sans que
l'on sache si c'est par fidélité (options existantes du programme d'origine) ou par inadvertance.
*Correction proposée* : dire laquelle des deux familles elles rejoignent — « reprises de l'original,
donc à tiret simple » ou « propres à Zaxnes, donc `--slot`, `--no-resume`, `--emu-stats`, les formes
à tiret simple étant acceptées en alias ».

**53-12 — REQ-UI-213 : effets sous-spécifiés. (non bloquant)**
Quatre trous : (a) « après le lancement du jeu » ne dit pas à quel point de `REQ-LOOP-021` (avant ou
après le passage à En cours de l'étape 7) le slot est chargé ; (b) rien n'est dit si le slot visé est
**vide**, ni s'il appartient à **un autre jeu** — cas que `REQ-PERS-052` autorise pour le slot
rapide (« le jeu du slot est alors lancé ») ; (c) l'interaction entre `-slot` et la reprise de
`REQ-LOOP-076` n'est pas fixée (`-slot` implique-t-il `-noresume` ?) ; (d) `-emustats` n'a ni
critère ni destination, alors qu'`AMD-032` §5 interdit d'emprunter la console parente pour un
lancement normal — la sortie doit donc aller dans le fichier de journal.
*Correction proposée* : ajouter les quatre clauses ; pour `-emustats`, « écrit une ligne par frame
émulée dans le journal, en anglais (`AMD-035` §2) ; critère : 600 lignes après 10 s en NTSC ».

**53-13 — Conditions de REQ-LOOP-076 exprimées hors du vocabulaire de SPEC-07. (non bloquant)**
« si un jeu est en cours ou en pause et n'est **ni en chargement ni en fin de partie** » : « en
chargement » est l'**indicateur** Chargement (`REQ-LOOP-021`), et « fin de partie » est défini par
`REQ-PERS-012` comme « arrêt du jeu, changement de jeu, **sortie** » — soit, au pied de la lettre,
l'événement même qu'on est en train de spécifier. La condition est donc circulaire.
*Correction proposée* : « si l'état est **En cours** ou **En pause** et que l'indicateur
**Chargement** est faux ».

**53-14 — Règles d'effacement et d'échec de la reprise incomplètes. (non bloquant)**
Deux cas non traités : (a) « l'ignorer laisse l'emplacement en place jusqu'à la prochaine sortie »
se combine mal avec « si aucun jeu ne tourne, l'emplacement est effacé » — ignorer la reprise puis
quitter sans jouer efface donc le point de reprise ; c'est peut-être voulu, mais la priorité des
deux phrases doit être écrite ; (b) rien ne dit ce qui se passe si le savestate de reprise existe
mais est **illisible ou incompatible** (profil modifié, version de savestate antérieure), alors que
`REQ-PERS-053` et `REQ-PERS-055` traitent ces cas pour les slots (`The game profile doesn't exist
anymore.`).
*Correction proposée* : énoncer la priorité des deux règles, et ajouter « un emplacement “reprise”
illisible ou incompatible est effacé, l'entrée n'est pas proposée, et le démarrage est normal »
(avec notification ou non, à trancher).

**53-15 — REQ-UI-211 : normalisation non définie et critère incomplet. (non bloquant)**
L'exigence demande d'ignorer « la casse **ni les accents** », mais le critère ne teste que la casse
(« mario » / « Mario » / « MARIO ») et aucune règle de normalisation n'est donnée — deux
implémentations peuvent différer sur « œ », « ß », « Pokémon ».
*Correction proposée* : « comparaison après décomposition NFD, retrait des marques diacritiques et
mise en minuscules invariante de culture » ; ajouter au critère « la saisie “pokemon” garde
“Pokémon” » et « la saisie “bros” garde “Mario Bros.” ».

**53-16 — Étanchéité : aucune fuite. (non bloquant, pour mémoire)**
L'en-tête écarte explicitement la présentation de la nouvelle interface de 3dSen. Les noms cités
(`-slot`, `-noresume`, `-emustats`, `Resume`, `Favorites only`, …) sont des chaînes
utilisateur-visibles, admises au même titre que le `-id` de `REQ-UI-007` ; les champs `Title` et
`Maker` sont des faits d'interopérabilité du format conservé. Aucun nom interne, aucun code, aucun
algorithme recopié.

### Conclusion AMD-053 (gate R2)

Fonctions bien choisies et bien délimitées, étanchéité acquise, numéros libres. Mais l'amendement se
déclare « en complément » de quatre exigences qu'il contredit — `REQ-LOOP-052` (53-1), `REQ-UI-030`
(53-3), la présentation neutralisée par `AMD-010` (53-4), `REQ-UI-084`/`REQ-PERS-041` (53-7) — et
laisse hors spec le modèle de persistance de l'emplacement « reprise » (53-2) et des favoris
(53-5), alors que SPEC-09 est exhaustive par construction ; enfin `REQ-UI-212` s'appuie sur une
signature et un en-tête que le format de profil n'a pas (53-6). Les sept bloquants sont tous
réparables par ajout de texte, sans changer la fonction visée. Gate R2 franchi **sous réserve des
sept bloquants 53-1 à 53-7**.

---

## 5. Récapitulatif

| Amendement | Verdict | Points | Bloquants |
|---|---|---|---|
| AMD-050 (REQ-SCN-076) | **accepté avec réserves** | 50-1 à 50-9 | **50-1** |
| AMD-051 (REQ-E3D-644/645/646, REQ-LOOP-073, REQ-PERS-088) | **accepté avec réserves — REQ-E3D-645 à retirer** | 51-1 à 51-12 | **51-1, 51-2, 51-3, 51-4, 51-5** |
| AMD-052 (REQ-LOOP-074/075, REQ-SCN-077, AMD-032 §2) | **accepté avec réserves** | 52-1 à 52-13 | **52-1 à 52-6** |
| AMD-053 (REQ-LOOP-076, REQ-UI-210 à 213) | **accepté avec réserves** | 53-1 à 53-16 | **53-1 à 53-7** |

Aucun des quatre n'est rejeté ; une seule exigence est à retirer (`REQ-E3D-645`, point 51-1).
Étanchéité acquise pour les quatre, avec un unique LEAK faible de formulation (51-11). Les quatorze
numéros REQ créés sont libres dans tout `spec/` et dans `impl/docs/`. Les 19 points bloquants
portent, pour la quasi-totalité, sur des **amendements manquants** d'exigences en vigueur (SPEC-02
`REQ-CORE-048` ; `REQ-E3D-524` ; `REQ-LOOP-012`, `-052` ; `REQ-SCN-057` ; `REQ-UI-030`, `-084` ;
`REQ-PERS-041`, `-050` ; la ligne SPEC-09 de `video.anti_alias`) et sur des critères d'acceptation
non exécutables : ils sont tous réparables par ajout de texte, sans remettre en cause les
comportements visés.
