# SPEC-05 — Runtime Lua des profils : modèle de scripts, hooks, API exposée
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-02 (lecture mémoire, mot d'entrées), SPEC-03 (message `Script`, réglages embarqués), SPEC-04 (pipeline de conversion, instances de frame, injection de shapes, palette de frame), SPEC-04B (géométrie, déformation, texture), SPEC-06 (scène, caméra, lumière, backdrop), SPEC-07 (boucle, savestates, reset), SPEC-08 (entrées, pad de script, Zapper), SPEC-09 (réglages exposés à Lua), SPEC-10 (notifications)

## 1. Objet et périmètre

Les profils embarqués contiennent des scripts Lua (579 scripts dans 89 des 101 profils) que le programme exécute pendant le jeu. Ce document fixe :

1. le **modèle de scripts** : où ils se trouvent, comment ils sont chargés, activés et liés aux instances de pattern ;
2. l'**environnement Lua** : état partagé, durée de vie des globales, rechargement ;
3. les **hooks** appelés par le programme, leur moment d'appel, leurs arguments et l'effet de leur valeur de retour ;
4. l'**API exposée aux scripts** (globales, objets, membres, méthodes, constantes), qui est un contrat d'interopérabilité : les noms sont ceux qu'emploient les données conservées ;
5. la **liaison des valeurs** entre Lua et le programme (nombres, vecteurs, listes, références, nil) ;
6. le traitement des **erreurs**, la sortie de `print`, les bibliothèques standard disponibles ;
7. les **données persistantes** des scripts dans les savestates ;
8. les points de **compatibilité** entre le dialecte Lua de l'original et Lua 5.4, le runtime retenu (PLAN.md, décision P1).

Hors périmètre : la console de scripting et l'éditeur de scripts de l'original (UI de scripting, hors périmètre du projet) ; les entrées VR (les membres VR existent mais renvoient des valeurs neutres, 3.10) ; la sémantique détaillée de chaque réglage (SPEC-09), de la géométrie (SPEC-04B) et du rendu (SPEC-06), auxquels ce document renvoie.

## 2. Concepts et vocabulaire

Vocabulaire de SPEC-04 (§2) : **pattern**, **instance de pattern** (« variante 3D » dans SPEC-03), **shape**, **instance de frame**, **frame**, **instances scriptables**, **Hs** = 248.

- **Script** : enregistrement du profil (message `Script`, SPEC-03 REQ-PROF-038) : nom, indicateur `enable`, indicateur `error`, source Lua, chaîne de tags.
- **Chunk** : la source d'un script compilée et exécutée une fois au chargement ; elle définit en général des fonctions globales, dont les hooks.
- **Hook** : fonction globale de nom réservé, définie par un chunk, que le programme appelle à un moment précis. Les dix noms réservés sont : `Start`, `Update`, `UpdateS`, `LateUpdate`, `End`, `BuildPattern`, `Load`, `Save`, `InputProcess`, `DetectShape`.
- **Script actif** : script dont `enable` est vrai et qui n'est pas en erreur.
- **Script en erreur** : script dont la compilation, l'exécution du chunk ou un hook a levé une erreur Lua depuis son dernier chargement (3.8).
- **État Lua du profil** : l'unique machine Lua (un seul espace de globales) dans laquelle tous les chunks et hooks du profil s'exécutent.
- **Ordre des scripts** : l'ordre de tri des scripts par nom défini en REQ-LUA-004 ; c'est l'ordre d'appel des hooks.
- **Liaison** : relation entre un script et une instance de pattern qui fait recevoir à ce script, via `UpdateS` et `BuildPattern`, les instances correspondantes (REQ-LUA-006).
- **Objet hôte** : valeur Lua qui représente un objet du programme (instance, shape, réglages, caméra…). Son comportement est fixé par les tables du §4.
- **Valeur vectorielle** : valeur Lua qui représente un vecteur, un quaternion, une couleur ou un rectangle **par copie** (type « valeur », §4.3).
- **Vecteur lié** : objet hôte qui représente un vecteur **stocké** dans un objet du programme (offset, échelle, rotation…) ; le modifier modifie l'objet (type « référence », §4.3).
- **Données persistantes** : la table globale `PData`, dont les entrées 1 à 64 sont enregistrées dans les savestates (3.9).
- **Corpus** : les 579 scripts des profils embarqués (`assets-neutral/profiles`).

Notations : `L` = lecture seule, `LE` = lecture et écriture ; « 0-based » = premier indice 0 (indexation des objets hôtes), « 1-based » = premier indice 1 (tables Lua).

## 3. Exigences

### 3.1 Scripts d'un profil, ordre et liaison

**REQ-LUA-001** (P1) — Les scripts d'un profil DOIVENT être lus depuis le champ `scripts` du profil (SPEC-03 REQ-PROF-024, REQ-PROF-038) : pour chacun, le nom (champ 1), l'indicateur `enable`, la source et la chaîne de tags. L'indicateur `error` lu dans le fichier DOIT être ignoré : tout script est considéré sans erreur au début de son chargement.
- Critère d'acceptation : le chargement des 101 profils embarqués produit 579 scripts dans 89 profils ; 556 ont `enable` vrai et 23 faux ; « empty » contient le seul script `Build` (tags `*`, actif).

**REQ-LUA-002** (P1) — Les scripts sont **globaux au profil** : il n'existe pas de script attaché à un pattern particulier. Un script reçoit des instances uniquement par la liaison (REQ-LUA-006) ; tous les autres hooks sont appelés une fois par événement, indépendamment des instances.
- Critère d'acceptation : un profil de test à un script sans tag commun avec aucune instance voit `Start`, `Update`, `LateUpdate` appelés, et `UpdateS` jamais.

**REQ-LUA-003** (P2) — Deux scripts d'un même profil NE DOIVENT PAS porter le même nom ; si un fichier en contient deux, le chargement DOIT garder le dernier lu et le signaler dans le journal.
- Critère d'acceptation : profil de test avec deux scripts « a » → un seul script « a » (le second), un avertissement journalisé.
- Notes : aucun profil embarqué n'a de doublon. L'original échoue au chargement dans ce cas ; garder le dernier est un choix plus robuste (écart de classe D).

**REQ-LUA-004** (P1) — L'**ordre des scripts** DOIT être le tri croissant des noms selon la comparaison suivante, appliquée caractère par caractère sur la première différence : (1) clé primaire = classe du caractère (ponctuation, espace et symboles < chiffres < lettres), puis le caractère mis en minuscules ; (2) à clé primaire égale sur toute la longueur commune, le nom le plus court d'abord ; (3) en cas d'égalité complète des clés primaires, départage par la casse, minuscule avant majuscule à la première différence. Cet ordre est celui de tous les appels de hooks (3.3) et celui de l'exécution des chunks pour les profils écrits par le programme.
- Critère d'acceptation : « Excitebike » → `bike, camera, ground, ramp_rotation, ramp7, scene` (le `_` précède le chiffre `7`) ; noms `B`, `a`, `b` → `a, b, B` ; `bg`, `bg'`, `bg1` → `bg, bg', bg1`.
- Critère d'acceptation (complément) : pour chacun des 89 profils à scripts de `assets-neutral/profiles`, le tri redonne exactement l'ordre des scripts dans le fichier (champ 3, SPEC-03).
- Notes : c'est l'ordre d'une collation linguistique usuelle (par exemple une comparaison de chaînes sensible à la culture, non ordinale) ; un tri ordinal par code de caractère donnerait `ramp7` avant `ramp_rotation` et changerait l'ordre d'exécution d'« Excitebike ». La règle ci-dessus reproduit l'ordre de l'original pour les 89 profils embarqués (DEC-03-Q02). **Portée** : la règle est normative pour les noms composés de caractères ASCII imprimables, ce qui couvre tous les profils embarqués ; pour un nom contenant d'autres caractères (profils utilisateur), l'implémentation DEVRAIT utiliser une collation Unicode standard indépendante de la langue du système, de mêmes propriétés (ponctuation < chiffres < lettres, casse au dernier niveau), et DOIT être déterministe ; l'écart éventuel avec l'original n'est pas spécifié. Répond à la question Q-02 de SPEC-03.

**REQ-LUA-005** (P1) — **Tags d'un script** : la chaîne de tags est découpée sur `,`, chaque élément est débarrassé de ses espaces de bord. Un script dont la chaîne de tags **contient** le caractère `*` (n'importe où) est lié à toutes les instances. Les éléments vides n'ont aucun effet.
- Critère d'acceptation : chaînes `"*"`, `"ui,*"` et `"a*b"` → liaison universelle ; `"gear, gear1"` → tags `gear` et `gear1`.

**REQ-LUA-006** (P1) — **Liaison** : un script DOIT être lié à une instance de pattern si et seulement si sa chaîne de tags contient `*` ou si l'un de ses tags est **égal** (comparaison exacte, sensible à la casse) à l'un des tags de l'instance (tags d'instance : SPEC-03 REQ-PROF-036, éléments non vides). Une instance de frame est liée aux mêmes scripts que l'instance de pattern dont elle est la copie (SPEC-04 REQ-E3D-190). La liaison est établie au chargement du profil et ne change pas pendant le jeu (les tags ne sont pas modifiables par les scripts du corpus).
- Critère d'acceptation : SPEC-04 REQ-E3D-321 ; « Mega Man 2 », script `gear` (tags `gear,gear1`) : lié exactement aux instances portant le tag `gear` ou `gear1`.
- Notes : les tags spéciaux d'instance (`!cast`, `!recv`…) sont aussi des tags ordinaires pour la liaison.

**REQ-LUA-007** (P1) — **Indicateur `enable`** : un script dont `enable` est faux NE DOIT PAS recevoir `Update`, `UpdateS`, `LateUpdate`, `BuildPattern`, `Load`, `Save` ni `InputProcess`, mais DOIT recevoir `Start` et `End`. `enable` est modifiable pendant le jeu par l'objet script (`Script:Get(nom).Enable`, §4.2) ; le changement s'applique au prochain appel de hook.
- Critère d'acceptation : « KungFu » n'a qu'un script, `hud`, désactivé, dont `Start` exécute `Setting.Layer[3].CastShadow = true` → après le démarrage du jeu, `Setting.Layer[3].CastShadow` est vrai et aucun `Update` de `hud` n'est appelé. « Jackal (U) » : le script `scene` active ou désactive le script `ov` (désactivé dans le fichier) via `sOv.Enable = scene == 1` ; quand `scene` vaut 1, `ov` reçoit ses `Update`/`UpdateS`/`LateUpdate` à partir de la frame suivante.

### 3.2 État Lua du profil

**REQ-LUA-010** (P1) — Tous les scripts d'un profil DOIVENT s'exécuter dans **un seul état Lua** dont les globales sont partagées : une globale définie par un script est visible et modifiable par tous les autres. Il n'y a pas d'environnement par script.
- Critère d'acceptation : « Adventures of Lolo (U) » : le script `bg` remplit les globales `bgSlot` et `bgCount` dans `UpdateS` ; le script `bg1` les lit dans son `LateUpdate` → `bg1` voit les valeurs de la frame courante.
- Notes : de nombreux profils partagent ainsi des tables et des indicateurs entre scripts (ex. `scene` et `sceneChanged`, calculés par un script `scene` et lus par un script `cam` : « DQ3_DW3 », « Zelda II - The Adventure of Link (U) », « Blaster Master (U) »).

**REQ-LUA-011** (P1) — **Chargement d'un profil** : un nouvel état Lua DOIT être créé, l'environnement de base installé (globales du §4.1, bibliothèques de 3.7, `PData` initialisé à 64 zéros), puis chaque script DOIT être chargé **dans l'ordre du fichier** : (a) les dix globales de hooks sont mises à nil ; (b) la source est compilée puis exécutée comme chunk ; (c) le programme mémorise, **pour ce script**, la valeur des dix globales de hooks qui sont des fonctions. Les hooks d'un script restent donc distincts de ceux des autres scripts même s'ils portent le même nom.
- Critère d'acceptation : profil de test à deux scripts `A` (définit `Update`) et `B` (ne définit pas `Update`) → seul `A` reçoit `Update` ; si `B` est chargé après `A`, `B` n'hérite pas de l'`Update` de `A`.
- Notes : pour les 89 profils embarqués l'ordre du fichier coïncide avec l'ordre des scripts (REQ-LUA-004).

**REQ-LUA-012** (P1) — Les fonctions globales **non réservées** (fonctions utilitaires) définies par plusieurs scripts DOIVENT suivre la règle ordinaire de Lua : la dernière définition exécutée remplace les précédentes pour **tous** les scripts, y compris dans les hooks des scripts chargés avant.
- Critère d'acceptation : « Super Spike V Ball (U) » : `bg` et `foot_body` définissent tous deux `Connect` ; après chargement, `Connect` est la version de `foot_body` (paramètres `foot, body`) et c'est elle que le `UpdateS` de `bg` appelle. Idem « Dragon Warrior (U) (PRG1) », `Inside` (version de `dialog`), et « Jackal (U) », `Overlap` (version de `hills`).
- Notes : relevé par l'outil `tools/lua_collisions.py` : 8 collisions de définitions différentes dans 8 profils, dont 5 purement textuelles (`max`).

**REQ-LUA-013** (P1) — Les globales DOIVENT persister d'une frame à l'autre et entre hooks pendant toute la durée de vie de l'état Lua. L'état Lua NE DOIT PAS être recréé lors d'une **relance du même jeu** sans rechargement du profil (reset, relance depuis la bibliothèque, chargement d'un slot du même jeu : SPEC-07 REQ-LOOP-021, REQ-LOOP-050) : seuls `End` puis `Start` (et `Load` le cas échéant) sont appelés, les chunks ne sont pas réexécutés, les indicateurs d'erreur et `enable` sont conservés.
- Critère d'acceptation : script dont le chunk exécute `n = (n or 0) + 1` au niveau principal et dont `Start` fait `print(n)` → affiche 1 au premier lancement, 1 encore après un reset du même jeu ; après lancement d'un autre jeu puis retour, 1.

**REQ-LUA-014** (P2) — Un changement de jeu (ou le chargement de « 2D Mode ») DOIT détruire l'état Lua du profil précédent après l'appel de ses `End` : aucune globale, aucune donnée persistante et aucun hook ne survit d'un profil à l'autre.
- Critère d'acceptation : jeu A dont un script fait `leak = 1` ; lancer B → dans B, `leak` vaut nil et `PData[6]` vaut 0.
- Notes : écart volontaire (classe D) : l'original garde une seule machine Lua pour toute la session et laisse fuir globales et `PData` d'un profil à l'autre ; aucun profil embarqué ne peut en dépendre de façon fiable. Cette règle satisfait la contrainte « aucun état global » de PLAN.md.

**REQ-LUA-015** (P1) — **Ordre du chargement** relativement aux autres étapes du profil : (1) exécution de tous les chunks (REQ-LUA-011) ; (2) application des réglages embarqués du profil (SPEC-09 REQ-PERS-005) ; (3) inscription et construction des patterns, avec appels de `BuildPattern` (REQ-LUA-030) ; (4) appels de `Start` (REQ-LUA-020). Le code exécuté au niveau principal d'un chunk NE DOIT donc pas compter sur les réglages du profil : ceux-ci l'écrasent à l'étape 2.
- Critère d'acceptation : chunk de test qui exécute `Setting.EmptyTile = true` au niveau principal, dans un profil dont les réglages portent `EmptyTile` faux → au premier `Update`, `Setting.EmptyTile` vaut faux ; le même code dans `Start` → vrai.

### 3.3 Hooks de cycle de vie

**REQ-LUA-020** (P1) — `Start()` DOIT être appelé pour chaque script non en erreur (actif ou non, REQ-LUA-007), dans l'ordre des scripts, à chaque démarrage de partie : lancement d'un jeu depuis la bibliothèque, relance du même jeu (reset) et chargement d'un savestate (avant `Load`, REQ-LUA-072). Il est appelé après le chargement du profil (s'il a eu lieu) et avant la première frame émulée de la partie. Aucun argument ; la valeur de retour est ignorée.
- Critère d'acceptation : « Adventures of Lolo (U) » : après le lancement, avant la première frame, `Setting.Layer[3].CastShadow` = vrai, `Setting.EmptyTile` = vrai, `MaxSlot` = 50, `#bgSlot` = 50.

**REQ-LUA-021** (P1) — `End()` DOIT être appelé pour chaque script non en erreur (actif ou non), dans l'ordre des scripts, à chaque fin de partie : arrêt du jeu, lancement d'un autre jeu ou du même jeu, chargement d'un savestate (avant la restauration), sortie du programme. Il est appelé avant le déchargement éventuel du profil et avant les remises à zéro de fin de partie (SPEC-09 REQ-PERS-012 ; backdrop et caméra, SPEC-06).
- Critère d'acceptation : « Duck Hunt (World) » : `Start` met le port 2 en Zapper (`Setting:SetInputType(0, 1)`) et `End` restaure les types lus au démarrage → après retour au menu, les deux ports sont de type Manette.

**REQ-LUA-022** (P1) — Un hook absent (non défini par le chunk, ou globale non fonction) DOIT simplement ne pas être appelé ; ce n'est pas une erreur.
- Critère d'acceptation : script vide → aucun message d'erreur, aucun appel.

**REQ-LUA-023** (P2) — Pendant les appels de `Start`, `End`, `Load` et `Save`, les globales `frame` et `time` gardent la valeur de la dernière frame convertie (0 avant la première frame de l'état Lua).
- Critère d'acceptation : `Start` qui fait `print(frame)` au premier lancement d'un profil → `0`.
- Notes : dans l'original elles valent nil avant la toute première frame de la session ; 0 évite une erreur sans changer le résultat des scripts du corpus.

### 3.4 Hooks de frame : `Update`, `UpdateS`, `LateUpdate`

**REQ-LUA-025** (P1) — Pour chaque frame émulée, après la segmentation de la trame et avant le calcul des positions des instances (SPEC-04 REQ-E3D-001 étape d, REQ-E3D-320), le programme DOIT exécuter, de façon synchrone, sur le fil d'émulation et dans cet ordre :
1. affecter les globales `frame` (compteur de la frame, valeur entière poussée en flottant, REQ-LUA-040) et `time` (`frame / 60`, secondes) (SPEC-04 REQ-E3D-043) ;
2. pour chaque script dans l'ordre des scripts : appeler `Update()` s'il est actif ;
3. pour chaque instance scriptable de la frame, dans l'ordre de SPEC-04 REQ-E3D-220, et pour chaque script lié à cette instance qui est actif et définit `UpdateS` : ajouter l'instance à la liste de ce script (ordre d'ajout conservé) ;
4. pour chaque script dans l'ordre des scripts dont la liste n'est pas vide : pour chaque instance de sa liste, affecter les globales `shape` (l'instance) et `shape2D` (sa shape), puis appeler `UpdateS()` ;
5. pour chaque script dans l'ordre des scripts : appeler `LateUpdate()` s'il est actif.
- Critère d'acceptation : profil de test à deux scripts `a` et `b` liés à toutes les instances, trame à 3 instances i1, i2, i3, chaque hook journalisant son appel → journal : `a.Update, b.Update, a.UpdateS(i1), a.UpdateS(i2), a.UpdateS(i3), b.UpdateS(i1), b.UpdateS(i2), b.UpdateS(i3), a.LateUpdate, b.LateUpdate`.
- Notes : les appels `UpdateS` d'un script sont **groupés** (toutes ses instances, puis le script suivant), et non entrelacés instance par instance. Les listes sont constituées à l'étape 3 : un script désactivé pendant son `Update` (étape 2) ne reçoit pas d'`UpdateS` ; un script désactivé pendant l'étape 4 garde les appels restants de sa liste pour cette frame.

**REQ-LUA-026** (P1) — Les globales `shape` et `shape2D` sont des globales ordinaires : après l'étape 4 elles gardent les valeurs de la dernière instance traitée, et un script peut les réaffecter (le corpus le fait, ex. `shape = Frame:GetShape(i)`). Seules les instances **scriptables** sont transmises à `UpdateS` ; les shapes créées par script ou restaurées d'une frame épinglée ne le sont pas (SPEC-04 REQ-E3D-220).
- Critère d'acceptation : `LateUpdate` qui lit `shape` voit la dernière instance passée au dernier `UpdateS` de la frame ; une shape créée par `Frame:CreateNewShape` pendant `Update` n'apparaît dans aucun appel `UpdateS`.

**REQ-LUA-027** (P1) — Les modifications faites par les hooks de frame sur des **instances de frame** ne valent que pour la frame en cours (SPEC-04 REQ-E3D-190) ; les modifications sur des **instances de pattern** (obtenues par `Pattern.GetPatternWithTag`, `refPattern3D`…) et sur un **pattern** sont durables et s'appliquent aux instances de frame créées à partir des frames suivantes. Les shapes créées par les hooks pendant une frame sont dessinées avec cette frame.
- Critère d'acceptation : TV-E3D-13 (SPEC-04) ; un `Update` qui fait `Pattern.GetPatternWithTag("t").Offset.x = 5` → à partir de la frame suivante, toutes les instances de frame copiées de cette instance ont `Offset.x` = 5 avant tout `UpdateS`.

**REQ-LUA-028** (P1) — Une **référence d'instance ou de shape conservée** par un script au-delà de sa frame (dans une globale) NE DOIT PAS provoquer de plantage ni d'erreur Lua à la lecture : l'objet DEVRAIT rester lisible avec les valeurs de sa dernière frame, et le modifier n'a aucun effet visible.
- Critère d'acceptation : script qui garde `old = shape` à la frame N et lit `old.Offset.x` et `old.Shape2D.tStart.x` à la frame N+10 → valeurs numériques, aucune erreur.
- Notes : dans la référence, une lecture tardive de ces objets peut renvoyer les valeurs d'une autre shape (objets réutilisés d'une frame à l'autre). Le corpus remet ses listes à zéro à chaque `Update`, il ne dépend pas de ce comportement.

### 3.5 Hooks spéciaux

**REQ-LUA-030** (P1) — `BuildPattern()` DOIT être appelé, pour chaque script actif lié à une instance de pattern, avec la globale `pattern` = cette instance de pattern, à chaque construction de la géométrie de cette instance et **avant** cette construction (SPEC-04 REQ-E3D-322) : au chargement du profil (avant tout `Start`), à l'apprentissage d'un pattern (SPEC-04 REQ-E3D-161) et lors de la reconstruction de tous les patterns après un changement d'algorithme de rendu. Les modifications que le hook apporte à `pattern` (profondeur, géométrie, échelle, texture, taux de remplissage…) sont permanentes et prises en compte par la construction qui suit.
- Critère d'acceptation : « empty » (« 2D Mode ») : script `Build` (`*`) qui fait `pattern.sizeZ = 1`, `pattern.geo = 0`, `pattern.Scale.z = 0.1` → chaque pattern appris en 2D Mode a une profondeur 1, la géométrie « Default » et une échelle z de 0,1. « Mega Man 2 », script `gear` : `pattern.fillrate = 0.05` sur les instances `gear`/`gear1` avant leur construction.
- Notes : les scripts `BuildPattern` de « Legend Of Zelda » et « Hudson s Adventure Island (U) » sont désactivés (`enable` faux) et ne sont donc jamais appelés. La reconstruction peut avoir lieu hors du fil d'émulation dans l'original ; la réimplémentation DOIT sérialiser ces appels avec les autres hooks (aucun appel Lua concurrent sur le même état).

**REQ-LUA-031** (P1) — `DetectShape` NE DOIT jamais être appelé par le programme, même s'il est défini (SPEC-04 REQ-E3D-323). Sa définition est acceptée sans effet.
- Critère d'acceptation : « Super Bat Puncher Demo » (script `detect_shape`, au demeurant désactivé) : aucune différence de segmentation avec ou sans ce script.

**REQ-LUA-032** (P1) — `InputProcess(index)` : un seul script fournit le pad de script, le **dernier chargé** (ordre du fichier) parmi ceux qui définissent `InputProcess`. Quand le pad de script est activé (`ScriptPad:Enable(true)`) et que le jeu est en cours, `InputProcess` DOIT être appelé une fois par frame émulée, **avant** l'exécution de la frame par le cœur, avec `index` = indice du joueur du pad (`ScriptPad.PadIndex`, 0 par défaut), si ce script est actif ; les globales `frame` et `time` sont mises à jour juste avant. La valeur renvoyée est convertie en entier (nombre tronqué vers zéro ; nil, booléen ou valeur non numérique → 0), puis combinée par OU au masque du joueur (SPEC-08 REQ-INP-071 à 073). Si le script est inactif, en erreur ou absent, la contribution est 0. `ScriptPad.State` vaut la dernière valeur obtenue.
- Critère d'acceptation : script de test `InputProcess` qui renvoie `8` une frame sur deux, `ScriptPad:Enable(true)` dans `Start` → le cœur voit Start enfoncé une frame sur deux sur le port 1 ; un retour `8.9` → 8 ; un retour `nil` → 0.
- Notes : seuls trois profils VR du corpus (« flappy_fpv », « flappy_tpv », « punchout ») l'utilisent ; sans données VR (REQ-LUA-095) leurs fonctions renvoient 0 et n'ont pas d'effet.

**REQ-LUA-033** (P2) — Les hooks ne reçoivent aucun argument, à l'exception de `InputProcess` (un entier). Leurs valeurs de retour sont ignorées, à l'exception de `InputProcess` (REQ-LUA-032).
- Critère d'acceptation : un `Update` qui renvoie `false` ou une table n'a aucun effet.

### 3.6 Liaison des valeurs entre Lua et le programme

**REQ-LUA-040** (P1) — **Nombres** : toute valeur numérique transmise du programme vers Lua (propriété, retour de méthode, globales `frame` et `time`, entrées de `PData` restaurées, argument de hook) DOIT être poussée comme un nombre **flottant** Lua (sous-type float de Lua 5.4), y compris quand elle est entière dans le programme. Une valeur Lua transmise au programme pour un paramètre entier DOIT être convertie par troncature vers zéro (3,7 → 3 ; −3,7 → −3 ; 2,5 → 2), puis réduite à la plage du type cible par modulo (octet : modulo 256 ; adresse 16 bits : modulo 65 536) ; pour un paramètre réel, convertie en réel simple précision.
- Critère d'acceptation : `math.type(frame)` → `"float"` ; `Script:ReadMem(0x70 + 0.9)` lit l'adresse `0x70` ; `Script:ReadMem(65536 + 0x70)` lit `0x70` ; `shape.Palette = 2.9` puis `shape.customPalette` → 2.
- Notes : justification en 3.11 (le dialecte de l'original n'a qu'un type de nombre, double précision ; pousser des flottants évite les erreurs « division entière par zéro » de Lua 5.4 dans `math.fmod(x, n)` et `x % n` quand `n` vient du programme et vaut 0). Les littéraux des scripts restent des entiers Lua.

**REQ-LUA-041** (P1) — **Arguments** : un paramètre numérique DOIT recevoir un nombre Lua (une chaîne, même numérique, est refusée) ; un paramètre chaîne DOIT recevoir une chaîne (nil accepté seulement s'il est documenté) ; un paramètre booléen reçoit la valeur de vérité Lua ; un paramètre objet reçoit un objet hôte du type attendu ou nil. Les paramètres optionnels indiqués au §4 PEUVENT être omis. Quand aucune signature ne correspond au nombre et aux types des arguments, l'appel DOIT lever une erreur Lua `invalid arguments to <membre>`.
- Critère d'acceptation : `Script:ReadMem("16")` → erreur Lua ; `Palette.FrameColorIndex(3)` et `Palette.FrameColorIndex(3, 1)` → valides ; `Frame:GetShape()` → erreur.

**REQ-LUA-042** (P1) — **Membres inconnus** : lire sur un objet hôte un nom qui n'est pas un membre DOIT renvoyer nil sans erreur ; écrire un nom qui n'est pas un membre DOIT lever l'erreur Lua `cannot set <nom>, no such field` ; écrire un membre en lecture seule (`L` au §4) DOIT lever une erreur Lua.
- Critère d'acceptation : `shape.hiden` et `shape2D.Hiden` (fautes de frappe présentes dans le corpus) → nil ; `shape.hiden = true` → erreur `cannot set hiden, no such field` ; `shape.Pos = Vector3.zero` → erreur.

**REQ-LUA-043** (P1) — **Identité** : un même objet hôte DOIT toujours être représenté par une valeur Lua égale (`==`) à elle-même, d'un accès à l'autre et d'un hook à l'autre (pendant la vie de l'objet) ; deux objets distincts sont différents. Les objets hôtes PEUVENT servir de clés de table.
- Critère d'acceptation : `shape.Shape2D == shape.Shape2D` → vrai ; dans « Adventures of Lolo (U) », `shape.Shape2D ~= _bgSlot.shape.Shape2D` distingue la shape du slot des autres ; `Frame:GetShape(0) == Frame:GetShape(0)` → vrai.

**REQ-LUA-044** (P1) — **Références et copies** : les types du §4.3 marqués « référence » (vecteurs liés `Vec3R`, `Vec2R`, `Mat4R`, `Calque`, `Découpe`, objets hôtes) DOIVENT être des vues vivantes : `shape.Offset.z = 4` modifie l'instance ; une référence gardée dans une globale (ex. `camRot = Camera.Rot` dans `Start`) continue de modifier l'objet d'origine aux frames suivantes. Les types marqués « valeur » (`Vector3`, `Vector2`, `Quaternion`, `Color`, `IntVector2`, `IntVector3`, `Rect`, `XRState`) DOIVENT être des copies : modifier un champ d'une telle valeur ne modifie que cette valeur Lua.
- Critère d'acceptation : « Antarctic Adventure (J) », script `cam` : `camRot = Camera.Rot` dans `Start`, puis `camRot.x, camRot.y = -60, 0` dans un `LateUpdate` ultérieur (scène 2) → `Camera.Rot.x` vaut −60 et la caméra s'incline ; `local bl = shape.BottomLeft; bl.x = 0` → `shape.BottomLeft.x` inchangé ; `shape2D.tStart.x = 0` (copie) → `shape2D.tStart.x` inchangé.
- Notes : les membres de type valeur s'écrivent par affectation complète (`shape.Offset.content = Vector3(0, 16, -4)`, `Setting.CustomBgColor = Color(0, 0, 0)`).

**REQ-LUA-045** (P1) — **Indexation** : les objets hôtes indexables (listes, shape → instances, pattern → instances, données perso `CData`, `Setting.Layer`, `Setting.Clipping`, vecteurs liés) DOIVENT être indexés **à partir de 0**, avec un indice numérique (tronqué). Un indice hors bornes sur une liste, une shape, un pattern ou `Setting.Layer` DOIT lever une erreur Lua. Les tables Lua (dont `PData`) restent indexées comme en Lua (1-based par convention).
- Critère d'acceptation : `local l = Frame:GetShapesWithTag("pf"); for i = 0, l.Count - 1 do local s = l[i] end` parcourt toutes les instances ; `l[l.Count]` → erreur ; `Setting.Layer[3]` désigne le calque L2 (indice 3 de §4.4).

**REQ-LUA-046** (P1) — **Énumérations** : les valeurs d'énumération (couche, alignement, type de géométrie, mode caméra, type de port, gravité de notification) DOIVENT être représentées en Lua par leur **valeur entière** (§4.4) ; les tables de constantes (`TLayer`, `Align`, `CamMode`) contiennent ces entiers. Un membre de type énumération accepte en écriture un entier ou une constante.
- Critère d'acceptation : `shape.Layer = 3` et `shape.Layer = TLayer.L2` sont équivalents ; « Jackal (U) » : `obj1.Layer == TLayer.L2` vaut vrai pour une instance de couche L2 ; `pattern.geo = 0` → géométrie « Default ».
- Notes : l'original représente ces valeurs par des objets opaques comparables à leurs constantes ; le corpus ne fait que les copier, les affecter ou les comparer à une constante, usages pour lesquels l'entier donne le même résultat.

**REQ-LUA-047** (P1) — **nil** : une méthode qui ne trouve rien renvoie nil (ex. `Frame:GetShapeWithTag` sans instance correspondante) ; passer nil là où un objet est requis et qu'il est utilisé (ex. `Frame:AddNewShape(nil, pos, 0)`) DOIT lever une erreur Lua (et non planter le programme).
- Critère d'acceptation : « Chip n Dale Rescue Rangers (U) », script `jeep` : si `rjeeps[index + 1]` vaut nil, `Frame:CreateNewShape(nil, bl, pal)` lève une erreur Lua, le script passe en erreur (3.8) et le jeu continue.

**REQ-LUA-048** (P2) — **Listes partagées** : `Frame:GetShapesWithTag(tag)` DOIT renvoyer **la même liste** à chaque appel, vidée puis remplie à chaque appel ; une liste obtenue précédemment reflète donc le résultat du dernier appel. Même règle pour `Pattern.GetPatternsWithTag` (une liste propre à cette fonction) et `Frame:GetShapesFromPinnedFrame`.
- Critère d'acceptation : `local a = Frame:GetShapesWithTag("x"); local b = Frame:GetShapesWithTag("y")` → `a == b`, et `a.Count` = nombre d'instances de tag `y`.
- Notes : « Excitebike » copie le résultat dans des tables Lua pour cette raison ; « Super Spike V Ball (U) » appelle deux fois de suite sans garder le premier résultat.

### 3.7 Bibliothèques standard et environnement de base

**REQ-LUA-050** (P1) — L'état Lua DOIT offrir les bibliothèques de base (`assert`, `error`, `ipairs`, `pairs`, `next`, `pcall`, `xpcall`, `select`, `tonumber`, `tostring`, `type`, `rawget`, `rawset`, `rawequal`, `setmetatable`, `getmetatable`, `unpack`), `math`, `string` et `table`. Il DEVRAIT offrir `coroutine` et `utf8`. Il NE DOIT PAS offrir `io`, `os` (sauf `os.clock`, `os.time`, `os.date`), `debug`, `package`, `require`, `dofile`, `loadfile` ni d'accès au système de fichiers ou au réseau.
- Critère d'acceptation : `type(math.fmod)`, `type(table.sort)`, `type(select)` → `"function"` ; `io` → nil ; `require` → nil ; tous les scripts du corpus se chargent.
- Notes : le corpus n'utilise que `math` (`max`, `min`, `fmod`, `abs`, `modf`, `sin`, `cos`, `floor`, `pi`, `rad`, `atan`, `acos`, `tan`), `table.sort`, `table.remove`, `select`, `ipairs` et `print`. Le retrait d'`io`/`os` est un écart volontaire (classe D) : un profil est une donnée partageable et ne doit pas pouvoir agir sur le système. `unpack` est ajouté comme alias de `table.unpack` pour la compatibilité (3.11).

**REQ-LUA-051** (P1) — Les globales du §4.1 DOIVENT être installées avant le chargement du premier chunk. Les noms de l'API (globales, membres, constantes) sont sensibles à la casse et DOIVENT être exposés exactement comme au §4.
- Critère d'acceptation : une analyse statique des 579 scripts extraits du champ 3 des profils de `assets-neutral/profiles` (résolution des chaînes d'accès à partir des globales du §4.1) résout 100 % des couples (type, membre) typables contre les tables du §4, sans membre manquant ni mauvais séparateur `.`/`:` ; le test dynamique de REQ-ACC-011 ne produit aucune erreur de membre manquant.

**REQ-LUA-052** (P2) — `print(...)` DOIT convertir chaque argument par la fonction globale `tostring` en vigueur, les joindre par une tabulation, et écrire la ligne dans le journal du programme (canal « Lua », niveau information). Un nombre flottant de valeur entière représentable DOIT être écrit sans partie décimale (`3`, et non `3.0`) ; les autres nombres au format `%.14g`.
- Critère d'acceptation : `print("frame", 12.0, 0.4, true, nil)` → ligne `frame\t12\t0.4\ttrue\tnil`.
- Notes : la console de scripting de l'original est hors périmètre ; seul le journal est requis. Le format des nombres reproduit celui du dialecte de l'original (3.11).

**REQ-LUA-053** (P3) — `clear()` DOIT exister et ne rien faire (elle vidait la console de scripting de l'original).
- Critère d'acceptation : `clear()` → aucun effet, aucune erreur.

### 3.8 Erreurs

**REQ-LUA-060** (P1) — Une erreur de **compilation** d'un script, ou une erreur levée pendant l'exécution de son **chunk**, DOIT : marquer le script en erreur ; n'enregistrer aucun de ses hooks ; laisser en place les globales que le chunk a pu définir avant l'erreur ; journaliser le message ; poursuivre le chargement des autres scripts et du profil.
- Critère d'acceptation : profil de test dont le script `a` contient une faute de syntaxe et le script `b` est valide → `b` fonctionne, `a` n'est jamais appelé, un message d'erreur contenant le nom `a` est journalisé une fois.

**REQ-LUA-061** (P1) — Une erreur levée pendant un **hook** DOIT : interrompre ce hook ; marquer le script en erreur ; journaliser une fois le message (nom du script, nom du hook, message Lua avec numéro de ligne) ; poursuivre la séquence en cours avec les autres scripts. Pour `UpdateS`, les instances restantes de la liste de ce script ne sont pas traitées pour cette frame. La conversion et l'affichage de la frame continuent (SPEC-04 REQ-E3D-324). Le jeu ne s'arrête jamais à cause d'un script.
- Critère d'acceptation : script dont `UpdateS` fait `error("x")` à la 2e instance → seule la 1re instance a été traitée, les `UpdateS` des scripts suivants et les `LateUpdate` des autres scripts s'exécutent (pas celui du script fautif), la frame s'affiche, un seul message journalisé.

**REQ-LUA-062** (P1) — Un script en erreur NE DOIT plus recevoir **aucun** hook (y compris `Start`, `End`, `Load`, `Save`, `InputProcess`) jusqu'au rechargement du profil (changement de jeu). L'état d'erreur survit à une relance du même jeu (REQ-LUA-013).
- Critère d'acceptation : script en erreur dans `Update` → plus d'`Update` ni de `LateUpdate` ; reset du même jeu → ni `End` ni `Start` de ce script ; lancer un autre jeu puis revenir → le script refonctionne.

**REQ-LUA-063** (P1) — Une erreur interne du programme provoquée par un appel d'API (indice hors bornes, objet nil, argument invalide…) DOIT être convertie en erreur Lua ordinaire (traitée par REQ-LUA-061), jamais en plantage ni en exception non rattrapée.
- Critère d'acceptation : `Setting.Layer[6]`, `Frame:GetShape(9999)`, `shape.CData[-1]` → erreur Lua journalisée, le jeu continue.

**REQ-LUA-064** (P2) — L'implémentation DEVRAIT protéger le fil d'émulation contre un script qui ne rend pas la main (boucle infinie) : au-delà d'un budget d'exécution par appel de hook (au moins 1 seconde ou un nombre d'instructions équivalent), le hook est interrompu comme par une erreur (REQ-LUA-061).
- Critère d'acceptation : `Update` contenant `while true do end` → au bout du budget, le script passe en erreur, l'émulation reprend.
- Notes : l'original n'a pas de garde-fou (le jeu se fige). Aucun script du corpus ne boucle indéfiniment.

### 3.9 Données persistantes et savestates

**REQ-LUA-070** (P1) — La globale `PData` DOIT être une table Lua ordinaire créée à la création de l'état Lua avec `PData[1]` à `PData[64]` = 0. La globale `pdata_count` vaut 64. `GetPData(i)` renvoie `PData[(i mod 64) + 1]` et `SetPData(i, v)` affecte `PData[(i mod 64) + 1] = v` (modulo à résultat positif pour i ≥ 0).
- Critère d'acceptation : `GetPData(0)` lit `PData[1]` ; `SetPData(64, 5)` écrit `PData[1]` ; `#PData` = 64 à la création.

**REQ-LUA-071** (P1) — **Sauvegarde** (savestate, quicksave, slot) : le programme DOIT appeler `Save()` de chaque script actif dans l'ordre des scripts, **puis** capturer les données persistantes : les 64 valeurs `PData[1]` … `PData[64]`, chacune convertie en réel simple précision (valeur non numérique, nil compris → 0 ; chaîne numérique → sa valeur). Seules ces 64 entrées sont enregistrées : `PData[0]`, `PData[65]` et toute autre clé ne le sont pas.
- Critère d'acceptation : « Excitebike », script `camera` : `Save` écrit `PData[2] = Camera.Distance` → le savestate contient cette distance à la précision simple ; « Zelda II - The Adventure of Link (U) », script `cam` : `PData[0] = oldRot.x` n'est **pas** enregistré.

**REQ-LUA-072** (P1) [ROM] — **Chargement** d'un savestate : après `End` (REQ-LUA-021), le rechargement éventuel du profil (changement de jeu, REQ-LUA-014) et la restauration du cœur, le programme DOIT affecter `PData[1]` … `PData[64]` avec les 64 valeurs enregistrées (nombres flottants), **puis** appeler `Start()` de chaque script (REQ-LUA-020), **puis** `Load()` de chaque script actif, dans l'ordre des scripts.
- Critère d'acceptation : « DQ3_DW3 », script `_scene` : `Save` fait `PData[6] = scene` (scène 2) ; recharger ce savestate depuis le menu → au moment de `Load`, `PData[6]` vaut 2 et `scene` devient 2 ; `Start` voit déjà `PData[6]` = 2.
- Notes : l'original restaure `PData` avant `Start` dans tous les cas ; pour un jeu dont le profil est rechargé, la restauration a lieu ici après la création du nouvel état Lua, ce qui donne le même résultat observable.

**REQ-LUA-073** (P1) — Le format de stockage des données persistantes DOIT être : une suite de 64 réels IEEE-754 simple précision, dans l'ordre `PData[1]` à `PData[64]` (§4.6). Son emplacement dans le fichier de savestate est fixé par SPEC-07/SPEC-09. La compatibilité avec les savestates de l'original n'est pas requise (PLAN.md).
- Critère d'acceptation : savestate de test avec `PData[1] = 1.5`, `PData[64] = -2` → bloc de 256 octets dont les 4 premiers codent 1,5 et les 4 derniers −2 (petit-boutiste).

**REQ-LUA-074** (P1) [ROM] — Aucun autre élément de l'état Lua (globales, tables des scripts, état d'erreur, `enable`) NE DOIT être enregistré ni restauré par un savestate ou par le rollback ; le rollback (SPEC-07) ne restaure pas `PData` et n'appelle ni `Save` ni `Load`.
- Critère d'acceptation : rollback de 2 s dans « DQ3_DW3 » → aucun appel `Load`, `scene` inchangé.

**REQ-LUA-075** (P2) — Sérialisation pour libretro (SPEC-07 REQ-LOOP-063, DEC-07-Q4, DEC-R2-D5) : `retro_serialize` DOIT appeler `Save()` de chaque script actif dans l'ordre des scripts, puis capturer les 64 valeurs de `PData` ; `retro_unserialize` DOIT restaurer les 64 valeurs puis appeler `Load()` de chaque script actif dans l'ordre des scripts, **sans** appeler `End` ni `Start` (le profil reste chargé). Cette règle vaut pour tous les appels de l'hôte (savestate, rembobinage, netplay) ; le run-ahead n'est pas pris en charge en v1.
- Critère d'acceptation : un `retro_serialize` journalise exactement un appel `Save` par script actif ; un `retro_unserialize` exactement un appel `Load` par script actif et aucun appel `Start` ni `End`.
- Notes : remplace la règle de la v1.0 (aucun hook pour les appels fréquents), contraire à DEC-07-Q4.

### 3.10 API exposée

**REQ-LUA-080** (P1) — Le programme DOIT exposer les globales du §4.1 et, pour chaque type du §4.2 et du §4.3, les membres listés, avec le genre (propriété, méthode appelée par `:`, fonction appelée par `.`, constante, indexation, appel), le type, le droit d'accès et l'effet indiqués. Les membres P1 et P2 sont requis ; les membres P3 DEVRAIENT être fournis.
- Critère d'acceptation : §5.1 (couverture du corpus = 100 %) ; tests unitaires par membre selon la colonne « Effet ».

**REQ-LUA-081** (P1) — Les méthodes d'instance des objets hôtes s'appellent avec `:` (l'objet est le premier argument) ; les fonctions des tables de classe (`Palette`, `Pattern`, `NotiCenter`, `Vector3`, `Quaternion`…) s'appellent avec `.`. Appeler une méthode d'instance avec `.` sans passer l'objet DOIT lever une erreur Lua.
- Critère d'acceptation : `Frame:GetShape(0)` valide ; `Frame.GetShape(0)` → erreur ; `Palette.FrameColor(0)` valide.

**REQ-LUA-082** (P1) — **Données perso** `CData` d'une instance : `CData[i]` pour `i` = 0 à 3 DOIT lire et écrire `DeformSpeed` ligne 3, colonne `i` (`m30`…`m33`) ; pour `i` ≥ 4, `DeformOffset` ligne 3, colonne `i mod 4` (`m30`…`m33`) ; un indice négatif lève une erreur. Ces huit valeurs sont stockées dans le profil (matrices de déformation, SPEC-03) et servent de paramètres libres aux scripts ; elles sont copiées dans chaque instance de frame.
- Critère d'acceptation : `shape.CData[2] = 7` → `shape.DeformSpeed.m32` = 7 ; `shape.CData[5]` = `shape.DeformOffset.m31` ; `shape.CData[9]` = `shape.DeformOffset.m31`.
- Notes : la ligne 3 des deux matrices n'intervient pas dans la déformation (SPEC-04B) ; 40 profils utilisent `CData`.

**REQ-LUA-083** (P1) — **Formules des grandeurs dérivées** lues par les scripts : celles du tableau 4.4 de SPEC-04 (`BottomLeft`, `TopRight`, `BL`, `TR`, `size`, `Start`, `End`, `TStart`…), `Pos` et `Center` (SPEC-04 REQ-E3D-205). En outre :
1. `GetPos()` = position calculée à l'instant de l'appel avec le pivot de l'instance, `GetPos(p)` avec le pivot `p` (formule de REQ-E3D-205) ; `Pos` et `Center` ne sont mis à jour que par le calcul de fin de frame (SPEC-04 REQ-E3D-001 e) ou par `CalculatePos()` (instance) / `CalculatePos()` (shape, pour toutes ses instances) ; avant le premier calcul de la frame, leur valeur n'est pas spécifiée ;
2. `OriPos` = `BottomLeft ⊙ (1 − q) + TopRight ⊙ q`, où `q` = `DeformPivot` et `⊙` le produit composante par composante ;
3. `GetDeform(p)` et `GetDeformation(p)` : soit `S = size`, `t = time`, `r = (p − O) ⊘ S` avec `O = OriPos` pour `GetDeform` et `O = BL ⊙ (1 − q) + TR ⊙ q` pour `GetDeformation` ; pour chaque axe `a` ∈ {x, y, z} de rang `k` ∈ {0, 1, 2} : `d_a = sin(r.x·V[k,0] + W[k,0]) · sin(r.y·V[k,1] + W[k,1]) · sin(r.z·V[k,2] + W[k,2]) · sin(t·V[k,3] + W[k,3])` avec `V` = `DeformSpeed`, `W` = `DeformOffset` ; résultat `= d ⊙ DeformAmp ⊙ S` ;
4. `Overlap(o)` (o instance) : avec `A` = boîte en pixels (écran) de la shape de l'instance et `B` celle de la shape de `o`, vrai si et seulement si `A.pStart.x < B.pEnd.x` et `A.pEnd.x > B.pStart.x` et `A.pStart.y < B.pEnd.y` et `A.pEnd.y > B.pStart.y` (inégalités strictes) ; `Overlap(o, δ)` : vrai si et seulement si `A.pStart.x < B.pEnd.x + δ` et `A.pEnd.x + δ > B.pStart.x` et `A.pStart.y < B.pEnd.y + δ` et `A.pEnd.y + δ > B.pStart.y` ; `Overlap(rect)` : vrai si `rect` chevauche strictement le rectangle d'origine `(BottomLeft.x, BottomLeft.y)` et de taille `(size.x, size.y)` (définition du chevauchement : type `Rect`, §4.3) ; `OverlapRect(rect)` fait le même test avec l'origine `(BL.x, BL.y)`, une unité plus bas : l'écart d'un pixel entre les deux méthodes est celui de l'original et DOIT être conservé ;
5. `ContainsTag(t)` : vrai si `t` = `"*"` ou si `t` est l'un des tags de l'instance (exact, sensible à la casse).
- Critère d'acceptation : vecteurs TV-LUA-05 à TV-LUA-07 (§5.2).
- Notes : le point `OriPos` utilisé par `GetDeform` diffère du pivot `Pd` de la déformation réellement appliquée (SPEC-04B REQ-E3D-568) de `(−q.x, h₀ − q.y, 0)` ; un script qui prédit le déplacement rendu obtient donc une valeur décalée d'au plus un pixel. Cet écart est celui de l'original et DOIT être conservé (fidélité).

**REQ-LUA-084** (P1) — **Requêtes de frame** (`Frame`, §4.2) : elles portent sur les instances **scriptables** de la frame en cours de conversion, dans l'ordre de SPEC-04 REQ-E3D-220 :
1. `GetShapesWithTag(t)` : liste partagée (REQ-LUA-048) des instances dont `ContainsTag(t)` est vrai ;
2. `GetShapeWithTag(t)` : la première d'entre elles, ou nil ;
3. `GetShape(i)` : l'instance d'indice `i` (0-based) ; erreur si `i` est hors de `[0, shapeCount − 1]` ;
4. `shapeCount` : nombre d'instances scriptables ;
5. `GetShapeWithPos(x, y, t)` : la première instance telle que `BottomLeft.x ≤ x ≤ TopRight.x`, `BottomLeft.y ≤ y ≤ TopRight.y` et `ContainsTag(t)` ; si `t` est omis il vaut la chaîne vide, qui ne correspond à aucune instance : l'appel à deux arguments renvoie donc toujours nil.
- Critère d'acceptation : trame de test à deux instances scriptables de tags `a` puis `a,b` → `GetShapesWithTag("a").Count` = 2, `GetShapeWithTag("b")` = la seconde, `GetShape(1)` = la seconde, `GetShapeWithPos(x, y)` = nil, `GetShapeWithPos(x, y, "*")` = l'instance qui contient `(x, y)`.

**REQ-LUA-085** (P1) — **Requêtes de patterns** (`Pattern`, §4.2) : `GetPatternWithTag(t)` DOIT renvoyer la première **instance de pattern** (et non une instance de frame) dont `ContainsTag(t)` est vrai, en parcourant les patterns **permanents** dans l'ordre du registre (SPEC-04 REQ-E3D-060) puis leurs instances par indice croissant, ou nil ; `GetPatternsWithTag(t)` renvoie, dans une liste partagée (REQ-LUA-048), toutes ces instances dans le même ordre. Les patterns temporaires ne sont pas parcourus.
- Critère d'acceptation : « Jackal (U) », script `scene` : `Pattern.GetPatternWithTag("square").pattern` renvoie le pattern du profil portant le tag `square` ; mettre son `Disable` à vrai retire ce pattern des candidats dès la frame suivante (SPEC-04 REQ-E3D-093).

**REQ-LUA-086** (P1) [ROM] — **Shapes injectées** : `Frame:AddNewShape(P, pos, pal)`, `Frame:CreateNewShape(P, pos, pal)` et `Frame:InsertShape(P, pos, pal)` DOIVENT se comporter comme SPEC-04 REQ-E3D-230 à 232. Le premier argument choisit la variante : si `P` est une **instance** (de pattern ou de frame : c'est alors son pattern et son indice qui sont utilisés), la fonction renvoie l'instance d'indice `P.Index` de la nouvelle shape, seule activée ; si `P` est un **pattern**, elle renvoie la nouvelle shape, toutes instances activées. `pos` est une valeur `Vector3`, `pal` un entier (palette 0–7 de la shape créée ; l'attribut de tile vaut `pal − 4`).
- Critère d'acceptation : TV-E3D-14 (SPEC-04) ; « Moai-kun » : `Frame:AddNewShape(cloud_pattern, shape:GetPos(), shape.Palette)` avec `cloud_pattern` instance → renvoie une instance non scriptable dessinée dans la frame.

**REQ-LUA-087** (P2) — **Palette de frame** (`Palette`, §4.2) : `FrameColorIndex(e [, sel])`, `GetColorIndex(e [, sel])`, `SetColorIndex(e, v [, sel])` et `FrameColor(e)` DOIVENT suivre SPEC-04 REQ-E3D-252 : sans `sel` ou avec `sel = 0`, la frame **publiée** (la frame précédemment convertie) ; avec `sel ≠ 0`, la frame en cours de conversion. `e` est l'indice d'entrée 0 à 63 de la palette de frame ; `FrameColorIndex` renvoie l'indice de couleur NES (0–63) ; `FrameColor` renvoie la `Color` RVB opaque correspondante (table de SPEC-04 §4.1, composantes 0–1).
- Critère d'acceptation : TV-E3D-09 (SPEC-04) lu via `Palette.FrameColorIndex(e, 1)` dans `Update` ; « punchout » : `Palette.SetColorIndex(mac.Palette * 4 + 1, v, 1)` recolore la frame courante.

**REQ-LUA-088** (P1) [ROM] — **Caméra** (`Camera`, §4.2) : `Rot` DOIT renvoyer le vecteur lié d'orientation du **mode courant** (TPV ou FPV ont chacun le leur), en degrés (`x` tangage, `y` lacet, `z` roulis ; convention SPEC-06) ; `Distance` (LE) lit ou écrit la distance du mode courant, l'écriture étant bornée à `[0, 512 × SceneScale]` ; `Mode` (LE) vaut 0 (TPV) ou 1 (FPV) ; `SetTarget(x, y, z)` ou `SetTarget(v)` fixe le point visé en coordonnées scène (pixels NES, repère de SPEC-04B REQ-E3D-410 : le centre du cadre de référence, cible par défaut, est (128, 120, 0)) ; `LookAt` (LE) lit ou écrit ce même point ; `FpvShape` (LE, shape ou nil) mémorise la shape suivie et, si non nil, vise la position de sa première instance ; `SetFPV([s])` passe en mode FPV et démarre le suivi de `s` si fourni. Les effets visuels (lissage, cadrage) relèvent de SPEC-06.
- Critère d'acceptation : « Excitebike », script `camera` : `camRot.x, camRot.y = 0, 0` hors de la scène 2 → caméra de face ; `Camera.Distance = 1000` → `Camera.Distance` = 512 ; `Camera:SetTarget(128, 120, 0)` puis `Camera.LookAt` → `(128, 120, 0)`.

**REQ-LUA-089** (P2) — **Réglages** (`Setting`, `Light`, `Layer`, `Clipping`) : membres, alias, droits et effets DOIVENT être ceux de SPEC-09 (REQ-PERS-007 à 016, tables §4.1 et §4.2). Les globales `Light`, `Layer` et `Clipping` désignent toujours les objets **courants** (`Setting.Light`, `Setting.Layer`, `Setting.Clipping`), y compris après l'application des réglages d'un profil.
- Critère d'acceptation : les 22 chemins d'accès de SPEC-09 §5.6 ; `Light == Setting.Light` → vrai après un changement de jeu.

**REQ-LUA-090** (P2) — **Backdrop** (`Backdrop`, §4.2) : `Enable`, `Color`, `RecvShadow`, `SizeX/Y/Z`, `OfsX/Y/Z` DOIVENT lire et écrire l'état du plan de fond de SPEC-06, avec effet à la frame de rendu suivante. À chaque fin de partie (après `End`), cet état DOIT revenir à : taille (256, 240, 1), décalage (0, 0, 0), couleur noire opaque, `RecvShadow` vrai, `Enable` faux.
- Critère d'acceptation : « Micro Mages » : `Start` fait `Backdrop.Enable = false`, `Backdrop.OfsZ = 17.1`, `Backdrop.RecvShadow = true` → `Backdrop.OfsZ` = 17,1 ; retour au menu → `Backdrop.OfsZ` = 0.

**REQ-LUA-091** (P2) — **Pad de script** (`ScriptPad`) : `Enable(b)` active ou désactive le pad de script (SPEC-08 REQ-INP-070) ; `PadIndex` (LE, 0 ou 1) choisit le joueur ; `State` (L) est la dernière valeur de `InputProcess`. Les données de mains et de tête VR (`HandPos`, `RotPos`, `LeftHand`/`LH`, `RightHand`/`RH`, `Head`) DOIVENT renvoyer des vecteurs nuls (desktop sans VR).
- Critère d'acceptation : `ScriptPad:HandPos(0).y` → 0 ; `ScriptPad.Head.vel.x` → 0.

**REQ-LUA-092** (P2) — **Manettes** (`Gamepad`, `Gamepad1`, `Gamepad2`) : `Gamepad[i]` (i = 0 ou 1) DOIT renvoyer l'objet joueur `i`, dont `State` (L) est le masque d'actions courant du joueur (SPEC-08 §4.1, OU de toutes ses sources) ; `Gamepad.State` est le masque global (SPEC-08). `Gamepad1` ≡ `Gamepad[0]`, `Gamepad2` ≡ `Gamepad[1]`.
- Critère d'acceptation : « punchout », script `check_duck_pattern` (désactivé) : `Gamepad[0].State == 32` vaut vrai quand seul Bas est enfoncé (SPEC-08 REQ-INP-003).

**REQ-LUA-093** (P2) — **Notifications** (`NotiCenter`) : `NotiCenter.Notify(msg [, durée [, gravité]])` DOIT afficher la notification `msg` pendant `durée` secondes (défaut 4) avec la gravité donnée (0 normale, 1 avertissement, 2 erreur ; défaut 0), selon les règles de SPEC-10 REQ-UI-080/081/085 : une notification n'en remplace une autre encore visible que si sa gravité est supérieure ou égale. `NotiCenter.Show(msg [, gravité])` équivaut à `Notify(msg, 4, gravité)`.
- Critère d'acceptation : « punchout » : `NotiCenter.Notify("Use Double Punch To Start/UpperCut")` → notification normale de 4 s.

**REQ-LUA-094** (P2) — **Mémoire du cœur** : `Script:ReadMem(a)` DOIT renvoyer l'octet (0–255) à l'adresse CPU `a` (16 bits) lu sans effet de bord (SPEC-02 REQ-CORE-080), dans l'état de fin de la frame qui vient d'être émulée ; `Script:WriteMem(a, v)` écrit l'octet `v mod 256` (SPEC-02).
- Critère d'acceptation : « Excitebike », script `bike` : `Script:ReadMem(0x70)` dans `Update` renvoie la valeur de la RAM `$0070` à la fin de la frame courante.

**REQ-LUA-095** (P2) — **Membres sans effet en desktop** : `Video.SceneScale` (L : 1 tant que `Setting.VrRuntime` = 0 ; une écriture n'a aucun effet géométrique mais, si la valeur écrite diffère de 1 et de la dernière valeur écrite, elle déclenche la réinitialisation de vue de SPEC-06 REQ-SCN-002 ; SPEC-09 REQ-PERS-011), les données VR (REQ-LUA-091) et les métadonnées `Profile.Title`, `Maker`, `GameDev`, `Des` (LE : stockées pour la session, sans effet sur l'affichage de la bibliothèque, SPEC-03 REQ-PROF-022) DOIVENT exister et ne pas lever d'erreur.
- Critère d'acceptation : « Tetris-Tengen » : `Profile.Maker = "Geod"` puis `Profile.Maker` → `"Geod"` ; le titre affiché dans la bibliothèque reste celui du catalogue. « flappy_fpv » : `Video.SceneScale = 0.02` puis `Video.SceneScale` → 1, et la caméra revient à sa vue par défaut (SPEC-06 REQ-SCN-002).

**REQ-LUA-096** (P2) — **Valeurs vectorielles** (`Vector3`, `Quaternion`, `Color`, `IntVector2`, `Rect`) : constructeurs, constantes, opérateurs et fonctions du §4.3 DOIVENT suivre les définitions mathématiques qui y sont données (repère de la scène : x droite, y haut, z profondeur, angles en degrés).
- Critère d'acceptation : TV-LUA-08 à TV-LUA-10 (§5.2).

**REQ-LUA-097** (P2) — **Slot de texture** : `slot` d'une instance désigne la zone de texture (atlas) où est rangée la texture de son instance de pattern (SPEC-04B) ; `slot.w` et `slot.h` en donnent la largeur et la hauteur en pixels. Affecter à `shape.slot` le slot d'une autre instance (`shape.slot = bgShape.slot`) DOIT faire texturer l'instance de frame, pour cette frame, avec la texture de l'autre instance (coordonnées de texture et décalage `texOffset`/`texTilting` appliqués comme pour sa propre texture, SPEC-04B).
- Critère d'acceptation : « Adventures of Lolo (U) », script `bg1` : une instance de fond qui reçoit `shape.slot = bgShape.slot` est dessinée avec la texture de `bgShape` ; `shape.slot.w` d'une instance de pattern 4×2 tiles vaut 32.

### 3.11 Compatibilité du dialecte

**REQ-LUA-100** (P1) — Le runtime DOIT être Lua 5.4 (PLAN.md) et DOIT accepter sans modification les 579 scripts du corpus (compilation sans erreur, y compris les 2 profils qui utilisent `goto` et des étiquettes).
- Critère d'acceptation : les 579 scripts extraits du champ 3 des profils de `assets-neutral/profiles`, compilés par le runtime Lua 5.4 retenu, donnent 0 échec.

**REQ-LUA-101** (P1) — **Dialecte de l'original** : l'original n'exécute pas Lua 5.3 mais un moteur **compatible Lua 5.1 avec extensions** (identifiant `_VERSION` = `Lua 5.1`, `goto` et étiquettes acceptés, bibliothèque `bit`, pas d'opérateurs entiers ni bit à bit, un seul type de nombre en double précision). La réimplémentation DOIT neutraliser les différences de comportement qui touchent le corpus :
1. **nombres** : un seul type numérique côté original ; d'où REQ-LUA-040 (valeurs de l'hôte poussées en flottants) et REQ-LUA-052 (affichage sans `.0`) ;
2. **`math.fmod` et `%` par zéro** : l'original renvoie `nan` ; Lua 5.4 lève une erreur pour deux entiers. Avec REQ-LUA-040, les divisions par une valeur venue de l'hôte sont flottantes et renvoient `nan` comme l'original ; seule une division de deux littéraux ou entiers produits par le script peut lever une erreur (aucun cas dans le corpus) ;
3. **`math.atan(y, x)`** : l'original ignore le second argument ; Lua 5.4 calcule `atan2`. Le corpus n'appelle `math.atan` qu'avec un argument (2 occurrences) ; aucune adaptation n'est requise ;
4. **globales 5.1 absentes de 5.4** : `unpack` DOIT être fourni (alias de `table.unpack`) ; `math.pow(x, y)` (= `x ^ y`), `math.ldexp(m, e)` (= `m · 2^e`), `math.log10(x)`, `table.getn(t)` (= `#t`) et `loadstring` (= `load` sur une chaîne) DEVRAIENT être fournis pour les profils utilisateur ; `setfenv`, `getfenv`, `module`, `bit` PEUVENT être absents (le corpus n'en utilise aucun) ;
5. **fonctions de bibliothèque exigeant un entier** (`string.sub`, `string.rep`, `table.insert`, `table.remove`, `select`, `string.format("%d")`…) : Lua 5.4 refuse un flottant non entier là où l'original tronquait. Le corpus ne leur passe que des entiers ou des flottants de valeur entière (acceptés par Lua 5.4) ; aucune adaptation n'est requise ;
6. **ordre de `pairs`** et **opérateur `#`** sur une table à trous : non spécifiés dans les deux dialectes ; le corpus n'utilise `pairs` nulle part et n'applique `#` qu'à des séquences sans trou ou vidées par la fin.
- Critère d'acceptation : `_VERSION` peut valoir `Lua 5.4` ; `unpack({1, 2})` → 1, 2 ; `math.fmod(5, Frame.shapeCount)` avec 0 instance → `nan` sans erreur ; l'exécution des 89 profils sur les vecteurs de §5 ne produit aucune erreur Lua qui n'existe pas dans l'original.
- Notes : constat fait en boîte noire sur la bibliothèque Lua livrée avec l'original (informatif) : `_VERSION` = `Lua 5.1`, `7 // 2` refusé à la compilation, `math.fmod(1, 0)` = `nan`, `math.atan(1, 2)` = `math.atan(1)`, présence de `unpack`, `loadstring`, `setfenv`, `bit`, `jit`. Les 579 scripts compilent dans les deux moteurs. La décision PLAN.md (« proche du Lua 5.3 de l'original ») doit être lue avec ce constat (question ouverte Q-05-1).

**REQ-LUA-102** (P2) — **Arithmétique des valeurs vectorielles** : les opérateurs `+`, `-` (binaire et unaire), `*`, `/`, `==` sur `Vector3`, `Vector2`, `Quaternion`, `Color`, `IntVector2` DOIVENT être fournis par métaméthodes selon §4.3 ; comparer avec `==` une valeur vectorielle à un nombre ou à nil renvoie faux sans erreur.
- Critère d'acceptation : `Vector3(1, 2, 3) + Vector3(1, 1, 1) == Vector3(2, 3, 4)` → vrai ; `pos ~= Vector3.zero` évalué sans erreur ; `Vector3.zero == nil` → faux.

### 3.12 Performance

**REQ-LUA-110** (P2) [ROM] — Pour chaque profil embarqué, le temps total des hooks d'une frame (étapes 1 à 5 de REQ-LUA-025) DEVRAIT rester inférieur à 2 ms en moyenne et à 6 ms au 99e centile sur la machine de référence (SPEC-12), mesuré sur 10 000 frames de jeu représentatives.
- Critère d'acceptation : harnais `Sen.Harness` avec chronométrage des hooks sur les jeux de référence de SPEC-12.
- Notes : cible du projet, pas une mesure de l'original. Les profils les plus lourds parcourent toutes les instances deux fois par frame (« Adventures of Lolo », « Super Mario Bros 3 »).

**REQ-LUA-111** (P2) [ROM] — L'accès à un membre d'objet hôte et la création de valeurs vectorielles NE DEVRAIENT pas allouer de mémoire gérée à chaque accès au point de provoquer des pauses de ramasse-miettes visibles (une pause > 4 ms par minute de jeu est un défaut).
- Critère d'acceptation : instrumentation : le harnais relève la durée de chaque pause du ramasse-miettes du processus (événements de pause du runtime .NET) et la mémoire gérée allouée par la couche Lua (différence d'allocation mesurée autour des appels de hooks). Sur 10 minutes de « Super Mario Bros 3 (U) (PRG1) » en harnais, aucune pause > 4 ms ne survient pendant une exécution avec scripts qui ne survienne aussi sans scripts, et l'allocation moyenne de la couche Lua reste inférieure à 1 Kio par frame. Sans ROM, le même test s'exécute sur la trame synthétique de REQ-ACC-011 (P2).

## 4. Formats et interfaces

Conventions des tables. **Genre** : `prop` = propriété ou champ (accès par `.`) ; `méth` = méthode d'instance (appel par `:`) ; `fonc` = fonction de table (appel par `.`) ; `const` = constante ; `index` = indexation `obj[i]` ; `appel` = appel de la table elle-même (constructeur). **Accès** : `L` lecture seule, `LE` lecture-écriture (pour `méth`/`fonc` : `—`). **Type** : type Lua du résultat (`nombre`, `entier` = nombre de valeur entière, `booléen`, `chaîne`, `nil`) ou nom de type des §4.2 et §4.3 ; `Liste<T>` = liste hôte (§4.3). **P** : priorité. Les membres utilisés par le corpus sont P1 ou P2 ; les autres sont P3.

Coordonnées : « écran » = pixels NES, origine en haut à gauche, y vers le bas ; « scène » = SPEC-04 REQ-E3D-205 (x droite, y haut, z profondeur, unité = pixel NES).

### 4.1 Globales

| Nom | Type | Accès | P | Effet |
|---|---|---|---|---|
| `shape` | `Instance` | LE | P1 | instance de frame en cours dans `UpdateS` (REQ-LUA-025, 026) |
| `shape2D` | `Shape` | LE | P1 | shape de `shape` dans `UpdateS` |
| `pattern` | `Instance` | LE | P1 | instance de pattern en cours dans `BuildPattern` (REQ-LUA-030) |
| `frame` | nombre | LE | P1 | compteur de la frame (REQ-LUA-025) ; réaffecté à chaque frame |
| `time` | nombre | LE | P1 | `frame / 60`, secondes |
| `PData` | table | LE | P1 | données persistantes (3.9) |
| `pdata_count` | entier | LE | P3 | 64 |
| `GetPData` | fonction | — | P3 | `GetPData(i)` (REQ-LUA-070) |
| `SetPData` | fonction | — | P3 | `SetPData(i, v)` (REQ-LUA-070) |
| `Setting` | `SettingAPI` | L | P1 | réglages (REQ-LUA-089, SPEC-09) |
| `Light` | `Lumière` | L | P1 | ≡ `Setting.Light` |
| `Layer` | `ConfigCalques` | L | P3 | ≡ `Setting.Layer` |
| `Clipping` | `Découpe` | L | P3 | ≡ `Setting.Clipping` |
| `Camera` | `CameraAPI` | L | P1 | caméra (REQ-LUA-088) |
| `Frame` | `FrameAPI` | L | P1 | requêtes et injection sur la frame en cours (REQ-LUA-084, 086) |
| `FrameManager` | `FrameManagerClasse` | L | P3 | table de classe ; `FrameManager.Ins` ≡ `Frame` |
| `Palette` | `PaletteAPI` | L | P1 | palette de frame (REQ-LUA-087) |
| `Pattern` | `PatternAPI` | L | P1 | registre des patterns (REQ-LUA-085) |
| `Script` | `ScriptAPI` | L | P1 | scripts et mémoire (REQ-LUA-094) |
| `Backdrop` | `BackdropAPI` | L | P1 | plan de fond (REQ-LUA-090) |
| `Video` | `VideoAPI` | L | P2 | vidéo 3D (REQ-LUA-095) |
| `Profile` | `ProfilAPI` | L | P2 | métadonnées du profil (REQ-LUA-095) |
| `ScriptPad` | `ScriptPadAPI` | L | P2 | pad de script (REQ-LUA-091) |
| `Gamepad` | `ManettesAPI` | L | P2 | manettes (REQ-LUA-092) |
| `Gamepad1` | `Joueur` | L | P3 | ≡ `Gamepad[0]` |
| `Gamepad2` | `Joueur` | L | P3 | ≡ `Gamepad[1]` |
| `ZapperOffset` | `Vec3R` | L | P2 | décalage de visée du Zapper (SPEC-08), vecteur lié, champs modifiables |
| `NotiCenter` | `NotiAPI` | L | P2 | notifications (REQ-LUA-093) |
| `TLayer` | `TLayerEnum` | L | P2 | constantes de couche (§4.4) |
| `Align` | `AlignEnum` | L | P3 | constantes d'alignement (§4.4) |
| `CamMode` | `CamModeEnum` | L | P3 | constantes de mode caméra (§4.4) |
| `Vector3` | `Vector3Classe` | L | P1 | constructeur et constantes (§4.3) |
| `Vector2` | `Vector2Classe` | L | P3 | constructeur (§4.3) |
| `Quaternion` | `QuaternionClasse` | L | P2 | fonctions de rotation (§4.3) |
| `Color` | `ColorClasse` | L | P1 | constructeur de couleur (§4.3) |
| `IntVector2` | `IntVector2Classe` | L | P2 | constructeur (§4.3) |
| `IntVector3` | `IntVector3Classe` | L | P3 | constructeur (§4.3) |
| `Rect` | `RectClasse` | L | P2 | constructeur (§4.3) |
| `print` | fonction | — | P1 | REQ-LUA-052 |
| `clear` | fonction | — | P3 | REQ-LUA-053 |
| `RenewId` | fonction | — | P3 | ≡ `Profile:RenewId()` |
| `self` | `ScriptObj` | L | P3 | objet du script dont un hook est en cours d'appel (valeur non spécifiée pendant `LateUpdate`) |

Les bibliothèques standard (REQ-LUA-050) s'ajoutent à ces globales. Les noms de hooks (§2) sont des globales ordinaires affectées par les chunks.

### 4.2 Objets hôtes

#### Type `Instance`

Instance de pattern (profil) ou instance de frame (copie par frame, SPEC-04 REQ-E3D-190). Mêmes membres pour les deux ; les grandeurs liées à une shape (`BottomLeft`, `Pos`, `Shape2D`…) n'ont de sens que pour une instance de frame. Unités des transformations : SPEC-04B/SPEC-06.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Offset` | prop | `Vec3R` | L | P1 | décalage de position (scène) ; vecteur lié modifiable en place |
| `offset` | prop | `Vec3R` | L | P2 | alias de `Offset` |
| `Scale` | prop | `Vec3R` | L | P1 | échelle (facteurs) |
| `scale` | prop | `Vec3R` | L | P2 | alias de `Scale` |
| `Rot` | prop | `Vec3R` | L | P1 | rotation, angles d'Euler en degrés |
| `rot` | prop | `Vec3R` | L | P3 | alias de `Rot` |
| `Pivot` | prop | `Vec3R` | L | P1 | pivot, fraction (0–1) de la boîte, par axe |
| `pivot` | prop | `Vec3R` | L | P3 | alias de `Pivot` |
| `DeformAmp` | prop | `Vec3R` | L | P1 | amplitude de déformation (REQ-LUA-083) |
| `DeformPivot` | prop | `Vec3R` | L | P1 | pivot de déformation |
| `DeformSpeed` | prop | `Mat4R` | L | P1 | matrice des vitesses de déformation |
| `DeformOffset` | prop | `Mat4R` | L | P1 | matrice des phases de déformation |
| `CData` | prop | `DonnéesPerso` | L | P1 | huit paramètres libres (REQ-LUA-082) |
| `Alpha` | prop | nombre | LE | P1 | opacité 0–1 (SPEC-06) |
| `alpha` | prop | nombre | LE | P2 | alias de `Alpha` |
| `Enable` | prop | booléen | LE | P1 | vrai ⇔ style de rendu > 0 ; écrire faux masque l'instance (SPEC-04 REQ-E3D-191) |
| `RenderStyle` | prop | entier | LE | P3 | style de rendu (0 = masqué) |
| `Palette` | prop | entier | LE | P1 | lecture : palette perso si ≥ 0, sinon palette de la shape (0–7) ; écriture : fixe la palette perso |
| `customPalette` | prop | entier | LE | P1 | palette perso, −1 = aucune |
| `Layer` | prop | entier | LE | P1 | couche (§4.4 `TLayer`) |
| `layer` | prop | entier | LE | P2 | alias de `Layer` |
| `UI` | prop | booléen | LE | P1 | vrai ⇔ couche UI ; écrire vrai met la couche UI, écrire faux n'a pas d'effet |
| `ui` | prop | booléen | LE | P3 | alias de `UI` |
| `CastShadow` | prop | booléen | LE | P1 | l'instance projette une ombre (SPEC-06) |
| `RecvShadow` | prop | booléen | LE | P1 | l'instance reçoit les ombres |
| `CastingShadow` | prop | booléen | L | P3 | ombre effective : `Setting.Shadow` et non UI et `CastShadow` et `Layer[c].CastShadow` de sa couche effective |
| `ReceivingShadow` | prop | booléen | L | P3 | idem pour la réception |
| `sizeZ` | prop | entier | LE | P1 | profondeur en pixels (SPEC-04B) ; prise en compte par la géométrie à la construction |
| `geo` | prop | entier | LE | P2 | type de géométrie (§4.4 `Geo3DType`) ; pris en compte à la construction |
| `fillrate` | prop | nombre | LE | P2 | taux de remplissage des géométries cylindriques (SPEC-04B) |
| `texOffset` | prop | `Vec2R` | L | P1 | décalage de texture, fraction de la taille de la texture (SPEC-04B) |
| `texTilting` | prop | `Vec2R` | L | P1 | répétition de texture (1, 1 par défaut) |
| `slot` | prop | `SlotTexture` | LE | P1 | zone de texture (REQ-LUA-097) |
| `Shape2D` | prop | `Shape` | L | P1 | shape de l'instance de frame |
| `shape` | prop | `Shape` | L | P3 | alias de `Shape2D` |
| `refPattern3D` | prop | `Instance` | LE | P1 | instance de pattern d'origine (pour une instance de pattern : elle-même) |
| `refPattern` | prop | `Pattern` | L | P1 | pattern de `refPattern3D` |
| `pattern` | prop | `Pattern` | LE | P1 | pattern porteur : pour une instance de pattern, son pattern ; pour une instance de frame, le **pattern de frame** de sa shape (défini sous le type `Pattern`) |
| `Index` | prop | entier | L | P2 | indice de l'instance dans la liste d'instances de son pattern porteur (0-based) |
| `index` | prop | entier | L | P2 | alias de `Index` |
| `Sp` | prop | booléen | L | P1 | shape de sprites |
| `Bg` | prop | booléen | L | P1 | shape de fond |
| `BottomLeft` | prop | `Vector3` | L | P1 | SPEC-04 tableau 4.4 |
| `TopRight` | prop | `Vector3` | L | P1 | SPEC-04 tableau 4.4 |
| `BL` | prop | `Vector3` | L | P1 | SPEC-04 tableau 4.4 |
| `TR` | prop | `Vector3` | L | P1 | SPEC-04 tableau 4.4 |
| `size` | prop | `Vector3` | L | P1 | SPEC-04 tableau 4.4 |
| `Size` | prop | `Vector3` | L | P1 | alias de `size` |
| `tSize` | prop | `IntVector2` | L | P1 | taille en tiles de la grille du pattern porteur |
| `Pos` | prop | `Vector3` | L | P1 | position calculée (REQ-LUA-083) |
| `Center` | prop | `Vector3` | LE | P3 | centre calculé |
| `OriPos` | prop | `Vector3` | L | P2 | REQ-LUA-083 |
| `OriPosWoLayer` | prop | `Vector3` | L | P3 | `BL ⊙ (1 − q) + TR ⊙ q`, `q` = `DeformPivot` |
| `constraintSize` | prop | `IntVector2` | LE | P2 | taille de contrainte du pattern porteur (SPEC-04 REQ-E3D-062) |
| `constraintWidth` | prop | entier | LE | P2 | composante x de `constraintSize` |
| `constraintHeight` | prop | entier | LE | P3 | composante y de `constraintSize` |
| `selfConnected` | prop | booléen | LE | P3 | du pattern porteur |
| `skipGeo` | prop | booléen | LE | P3 | du pattern porteur |
| `TagStr` | prop | chaîne | L | P3 | chaîne de tags |
| `IsStatic` | prop | booléen | LE | P3 | faux si le tag `!static` est présent (SPEC-06) |
| `LAxis` | prop | booléen | LE | P3 | faux si le tag `_gaxis` est présent (SPEC-06) |
| `antiShadow` | prop | booléen | LE | P3 | SPEC-04B |
| `flip` | prop | booléen | LE | P3 | SPEC-04B |
| `solid` | prop | booléen | LE | P3 | SPEC-04B |
| `fullWarp` | prop | booléen | LE | P3 | SPEC-04B |
| `emptyColor` | prop | entier | LE | P3 | couleur vide (SPEC-04B) |
| `AnimEnable` | prop | booléen | L | P3 | vrai si une composante de `DeformAmp` est non nulle |
| `GetPos` | méth | `Vector3` | — | P1 | `GetPos()` ou `GetPos(pivot: Vector3)` (REQ-LUA-083) |
| `GetOriginalPos` | méth | `Vector3` | — | P3 | `GetOriginalPos(q)` = `BottomLeft ⊙ (1 − q) + TopRight ⊙ q` |
| `GetPosWoLayer` | méth | `Vector3` | — | P3 | `GetPosWoLayer(q)` = `BL ⊙ (1 − q) + TR ⊙ q` |
| `CalculatePos` | méth | nil | — | P2 | recalcule `Pos` et `Center` |
| `Overlap` | méth | booléen | — | P1 | `Overlap(i: Instance)`, `Overlap(i, delta: entier)`, `Overlap(r: Rect)` (REQ-LUA-083) |
| `OverlapRect` | méth | booléen | — | P3 | comme `Overlap(r)` avec l'origine `BL` |
| `ContainsTag` | méth | booléen | — | P1 | `ContainsTag(tag: chaîne)` (REQ-LUA-083) |
| `GetDeform` | méth | `Vector3` | — | P2 | `GetDeform(p: Vector3)` (REQ-LUA-083) |
| `GetDeformation` | méth | `Vector3` | — | P2 | `GetDeformation(p: Vector3)` (REQ-LUA-083) |

#### Type `Shape`

Shape d'une frame (SPEC-04 §2). Exposée comme `shape2D` et par `Instance.Shape2D`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `tStart` | prop | `IntVector2` | L | P1 | coin de la boîte en tiles (cellule) |
| `tEnd` | prop | `IntVector2` | L | P1 | coin opposé (inclus) |
| `tSize` | prop | `IntVector2` | L | P1 | `tEnd − tStart + (1, 1)` |
| `TStart` | prop | `IntVector2` | L | P1 | alias de `tStart` |
| `TEnd` | prop | `IntVector2` | L | P1 | alias de `tEnd` |
| `TSize` | prop | `IntVector2` | L | P1 | alias de `tSize` |
| `Start` | prop | `IntVector2` | L | P2 | `pStart` (écran) |
| `End` | prop | `IntVector2` | L | P2 | `pEnd` (écran, inclus) |
| `pStart` | prop | `IntVector2` | L | P3 | idem `Start` |
| `pEnd` | prop | `IntVector2` | L | P3 | idem `End` |
| `Size` | prop | `IntVector2` | L | P1 | `pEnd − pStart + (1, 1)` |
| `BottomLeft` | prop | `Vector2` | L | P2 | `(pStart.x, Hs − pEnd.y − 1)` |
| `TopRight` | prop | `Vector2` | L | P2 | `(pEnd.x, Hs − pStart.y − 1)` |
| `BL` | prop | `Vector2` | L | P3 | alias de `BottomLeft` |
| `TR` | prop | `Vector2` | L | P3 | alias de `TopRight` |
| `Bg` | prop | booléen | L | P1 | shape de fond |
| `bg` | prop | booléen | L | P2 | alias de `Bg` |
| `Hidden` | prop | booléen | L | P2 | SPEC-04 REQ-E3D-048 |
| `hidden` | prop | booléen | L | P1 | alias de `Hidden` |
| `Palette` | prop | entier | L | P1 | palette de la shape 0–7 (SPEC-04 REQ-E3D-047) |
| `palIndex` | prop | entier | L | P1 | alias de `Palette` |
| `TileCount` | prop | entier | L | P3 | nombre de tiles |
| `Inside` | prop | booléen | L | P3 | SPEC-04 REQ-E3D-045 |
| `AtBorder` | prop | booléen | L | P2 | négation de `Inside` |
| `Completed` | prop | booléen | L | P1 | SPEC-04 REQ-E3D-046 |
| `Age` | prop | entier | L | P3 | SPEC-04 REQ-E3D-207 |
| `InsCount` | prop | entier | L | P2 | nombre d'instances (= nombre d'instances du pattern de référence) |
| `Count` | prop | entier | L | P2 | alias de `InsCount` |
| `[i]` | index | `Instance` | L | P1 | instance de frame d'indice `i` (0-based) |
| `Shape3D` | méth | `Instance` | — | P3 | `Shape3D([i])` ≡ `[i]`, `i` = 0 par défaut |
| `shape3D` | prop | `Instance` | L | P2 | ≡ `[0]` |
| `refPattern` | prop | `Pattern` | L | P1 | pattern de référence (profil) |
| `pattern` | prop | `Pattern` | L | P3 | pattern propre à la shape |
| `layer` | prop | entier | L | P2 | couche du pattern propre à la shape |
| `geo` | prop | entier | L | P2 | type de géométrie du pattern propre à la shape |
| `sizeZ` | prop | entier | L | P3 | profondeur du pattern propre à la shape |
| `center` | prop | `Vector3` | L | P3 | centre écran `(pStart + pEnd + (1, 1)) / 2` |
| `CalculatePos` | méth | nil | — | P2 | `CalculatePos()` : recalcule `Pos`/`Center` de toutes ses instances |
| `Distance` | méth | nombre | — | P3 | `Distance(s: Shape)` = carré de la distance entre les `Pos` de leurs instances 0 |

#### Type `Pattern`

Pattern du profil, ou **pattern de frame** d'une shape. Le pattern de frame d'une shape est un objet `Pattern` créé pour la frame courante et détruit avec elle : sa grille est la boîte en tiles de la shape (`TSize`) ; ses instances sont les instances de frame de la shape, copies des instances du pattern de référence (`refPattern`) faites à la liaison de la shape (SPEC-04), dans le même ordre ; les modifications qu'un script y apporte ne valent que pour cette frame et ne touchent pas le pattern de référence. Les propriétés `layer`, `geo` et `sizeZ` d'une shape sont celles de l'instance 0 de son pattern de frame.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Disable` | prop | booléen | LE | P2 | exclut le pattern des candidats (SPEC-04 REQ-E3D-093) ; non sauvegardé |
| `tSize` | prop | `IntVector2` | L | P1 | taille de la grille en tiles `(W, H)` |
| `Size` | prop | `Vector2` | L | P2 | `pEnd − pStart + (1, 1)` (boîte de pixels du pattern) |
| `BottomLeft` | prop | `Vector2` | L | P3 | SPEC-04 REQ-E3D-064 |
| `TopRight` | prop | `Vector2` | L | P3 | SPEC-04 REQ-E3D-064 |
| `BL` | prop | `Vector2` | L | P3 | alias |
| `TR` | prop | `Vector2` | L | P3 | alias |
| `[i]` | index | `Instance` | L | P1 | instance de pattern d'indice `i` (0-based) |
| `pattern3D` | prop | `Instance` | L | P3 | ≡ `[0]` |
| `count` | prop | entier | L | P3 | nombre d'instances |
| `pattern3DList` | prop | `Liste<Instance>` | L | P3 | instances |
| `constraintSize` | prop | `IntVector2` | LE | P3 | taille de contrainte |
| `CSize` | prop | `IntVector2` | LE | P3 | alias |
| `SelfConnected` | prop | booléen | LE | P3 | SPEC-04 REQ-E3D-061 |
| `SkipGeo` | prop | booléen | LE | P3 | SPEC-04 REQ-E3D-062 |
| `permanent` | prop | booléen | L | P3 | pattern permanent |

#### Type `FrameAPI`

Global `Frame`. Les requêtes portent sur la frame en cours de conversion (REQ-LUA-084).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `GetShapesWithTag` | méth | `Liste<Instance>` | — | P1 | `GetShapesWithTag(tag)` (REQ-LUA-084, 048) |
| `GetShapeWithTag` | méth | `Instance` | — | P1 | `GetShapeWithTag(tag)` ou nil |
| `GetShape` | méth | `Instance` | — | P1 | `GetShape(i)` (0-based) |
| `GetShapeWithPos` | méth | `Instance` | — | P2 | `GetShapeWithPos(x, y [, tag])` |
| `shapeCount` | prop | entier | L | P1 | nombre d'instances scriptables |
| `frameCounter` | prop | entier | L | P3 | ≡ `frame` |
| `time` | prop | nombre | L | P3 | ≡ `time` |
| `AddNewShape` | méth | `Instance` | — | P2 | `AddNewShape(P, pos, pal)` (REQ-LUA-086) ; renvoie une `Shape` si `P` est un `Pattern` |
| `CreateNewShape` | méth | `Shape` | — | P2 | `CreateNewShape(P, pos, pal)` (REQ-LUA-086) ; renvoie une `Instance` si `P` est une `Instance` |
| `InsertShape` | méth | `Shape` | — | P3 | `InsertShape(P, pos, pal)` (REQ-LUA-086) |
| `PinNewFrame` | méth | nil | — | P2 | SPEC-04 REQ-E3D-233 |
| `ReleasePinnedFrame` | méth | nil | — | P2 | SPEC-04 REQ-E3D-233 |
| `AddRectFromPinnedFrame` | méth | nil | — | P2 | `AddRectFromPinnedFrame(x, y, w, h)` (SPEC-04 REQ-E3D-233) |
| `RestoreFromPinnedFrame` | méth | nil | — | P2 | `RestoreFromPinnedFrame([r: Rect])` (SPEC-04 REQ-E3D-233) |
| `GetShapesFromPinnedFrame` | méth | `Liste<Shape>` | — | P3 | `GetShapesFromPinnedFrame(r: Rect)` ou `(pos: Vector2, taille: Vector2)` : shapes de la frame épinglée qui chevauchent ; liste partagée |

#### Type `FrameManagerClasse`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Ins` | prop | `FrameAPI` | L | P3 | ≡ `Frame` |

#### Type `PaletteAPI`

Global `Palette` (fonctions de table). `sel` : 0 (défaut) = frame publiée, ≠ 0 = frame en cours (REQ-LUA-087).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `FrameColorIndex` | fonc | entier | — | P1 | `FrameColorIndex(e [, sel])` : indice NES 0–63 de l'entrée `e` (0–63) |
| `GetColorIndex` | fonc | entier | — | P3 | ≡ `FrameColorIndex` |
| `SetColorIndex` | fonc | nil | — | P2 | `SetColorIndex(e, v [, sel])` |
| `FrameColor` | fonc | `Color` | — | P1 | `FrameColor(e)` : couleur RVB de l'entrée `e` de la frame publiée |
| `bgColor` | prop | `Color` | L | P3 | couleur de fond effective (SPEC-06) |

#### Type `PatternAPI`

Global `Pattern` (fonctions de table, REQ-LUA-085).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `GetPatternWithTag` | fonc | `Instance` | — | P1 | `GetPatternWithTag(tag)` ou nil |
| `GetPatternsWithTag` | fonc | `Liste<Instance>` | — | P2 | `GetPatternsWithTag(tag)`, liste partagée |

#### Type `ScriptAPI`

Global `Script`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Get` | méth | `ScriptObj` | — | P2 | `Get(nom)` : script de ce nom ou nil |
| `GetScript` | méth | `ScriptObj` | — | P3 | `GetScript(nom)` ≡ `Get` ; `GetScript(i)` : i-ème script dans l'ordre des scripts (0-based) ou nil |
| `Count` | prop | entier | L | P3 | nombre de scripts |
| `ReadMem` | méth | entier | — | P1 | `ReadMem(adresse)` (REQ-LUA-094) |
| `WriteMem` | méth | nil | — | P3 | `WriteMem(adresse, octet)` (REQ-LUA-094) |

#### Type `ScriptObj`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Enable` | prop | booléen | LE | P2 | indicateur `enable` (REQ-LUA-007) |
| `enable` | prop | booléen | LE | P3 | alias |
| `Name` | prop | chaîne | L | P3 | nom |
| `name` | prop | chaîne | L | P3 | alias |
| `Error` | prop | booléen | L | P3 | script en erreur |
| `TagStr` | prop | chaîne | L | P3 | chaîne de tags |

#### Type `CameraAPI`

Global `Camera` (REQ-LUA-088 ; effets visuels SPEC-06).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Rot` | prop | `Vec3R` | L | P1 | orientation du mode courant, degrés |
| `Distance` | prop | nombre | LE | P1 | distance du mode courant, bornée `[0, 512 × SceneScale]` |
| `Mode` | prop | entier | LE | P2 | 0 = TPV, 1 = FPV |
| `LookAt` | prop | `Vector3` | LE | P2 | point visé (scène) |
| `FpvShape` | prop | `Shape` | LE | P2 | shape suivie en FPV |
| `SetTarget` | méth | nil | — | P2 | `SetTarget(x, y, z)` ou `SetTarget(v: Vector3)` |
| `GetTarget` | méth | `Vector3` | — | P3 | ≡ lecture de `LookAt` |
| `SetFPV` | méth | nil | — | P2 | `SetFPV([s: Shape])` |
| `SetTPV` | méth | nil | — | P3 | passe en mode TPV |
| `ToggleMode` | méth | nil | — | P3 | bascule TPV/FPV |
| `ResetView` | méth | nil | — | P3 | vue par défaut (SPEC-06) |
| `IsFPV` | prop | booléen | L | P3 | mode FPV |
| `IsTPV` | prop | booléen | L | P3 | mode TPV |
| `Target` | prop | `Vec3R` | L | P3 | point visé interne du mode courant (scène mise à l'échelle) |

#### Type `SettingAPI`

Global `Setting`. **Liste complète des membres et sémantique : SPEC-09** (tables §4.1, §4.2, §4.3). Le tableau ci-dessous reprend les membres utilisés par le corpus (contrôle de couverture) et ceux nécessaires à d'autres sections.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Layer` | prop | `ConfigCalques` | L | P1 | calques (SPEC-09) |
| `Clipping` | prop | `Découpe` | L | P1 | découpe `(gauche, bas, droite, haut)` (SPEC-04 REQ-E3D-002) |
| `Light` | prop | `Lumière` | L | P3 | lumière (≡ globale `Light`) |
| `EmptyTile` | prop | booléen | LE | P1 | SPEC-04 REQ-E3D-004 |
| `BgColor` | prop | booléen | LE | P1 | fond = couleur NES de la frame (vrai) ou `CustomBgColor` (faux) |
| `CustomBgColor` | prop | `Color` | LE | P1 | couleur de fond personnalisée |
| `SamePalette` | prop | booléen | LE | P2 | alias de `SameBgPalette` |
| `SameBgPalette` | prop | booléen | LE | P2 | SPEC-04 |
| `SameSpPalette` | prop | booléen | LE | P3 | SPEC-04 |
| `LocalCam` | prop | booléen | LE | P2 | vue de dessus (SPEC-06) |
| `TopDownCam` | prop | booléen | LE | P2 | alias de `LocalCam` |
| `GameSpeed` | prop | nombre | LE | P2 | vitesse d'émulation (SPEC-07, SPEC-09 REQ-PERS-010/012) |
| `SmoothTime` | prop | nombre | LE | P2 | lissage caméra, s |
| `VrRuntime` | prop | entier | LE | P2 | toujours 0 (SPEC-09 REQ-PERS-011) |
| `SceneRatio` | prop | nombre | LE | P2 | 1 en desktop (SPEC-09 REQ-PERS-011) |
| `SceneScale` | prop | nombre | LE | P3 | SPEC-09 REQ-PERS-013 |
| `Shadow` | prop | booléen | LE | P3 | ombres globales |
| `GetInputType` | méth | entier | — | P2 | `GetInputType(port)` : 0 Manette, 1 Zapper (SPEC-08 REQ-INP-060) |
| `SetInputType` | méth | nil | — | P2 | `SetInputType(t1, t2)` : types des ports 1 et 2 |

#### Type `ConfigCalques`

`Setting.Layer` (≡ globale `Layer`).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `[i]` | index | `Calque` | L | P1 | calque `i` = 0 à 5 (§4.4 `TLayer`), entier ou constante |
| `DefLayer` | prop | entier | LE | P3 | couche par défaut des shapes de fond non classées (SPEC-09) |
| `Auto` | prop | booléen | LE | P3 | SPEC-09 |

#### Type `Calque`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Offset` | prop | nombre | LE | P2 | décalage z du calque (SPEC-04 REQ-E3D-205) |
| `Aligned` | prop | entier | LE | P2 | alignement (§4.4 `Align`) |
| `Coef` | prop | nombre | L | P3 | 1 / 0 / −1 selon `Aligned` (SPEC-09 REQ-PERS-013) |
| `CastShadow` | prop | booléen | LE | P1 | les instances du calque projettent des ombres |
| `RecvShadow` | prop | booléen | LE | P2 | les instances du calque reçoivent les ombres |

#### Type `Découpe`

`Setting.Clipping` (≡ globale `Clipping`), vue vivante.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `[i]` | index | nombre | LE | P1 | `i` = 0 à 3 ≡ `x`, `y`, `z`, `w` |
| `x` | prop | nombre | LE | P3 | marge gauche |
| `y` | prop | nombre | LE | P3 | marge basse |
| `z` | prop | nombre | LE | P3 | marge droite |
| `w` | prop | nombre | LE | P3 | marge haute |

#### Type `Lumière`

Global `Light` (≡ `Setting.Light`, SPEC-06/SPEC-09).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Direction` | prop | `Vector3` | LE | P1 | direction de la lumière directionnelle |
| `Color` | prop | `Color` | LE | P1 | couleur de la lumière |
| `Intensity` | prop | nombre | LE | P3 | intensité |

#### Type `BackdropAPI`

Global `Backdrop` (REQ-LUA-090, SPEC-06).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Enable` | prop | booléen | LE | P1 | plan de fond affiché |
| `Color` | prop | `Color` | LE | P1 | couleur du plan |
| `RecvShadow` | prop | booléen | LE | P1 | le plan reçoit les ombres |
| `SizeX` | prop | nombre | LE | P2 | taille x (scène) |
| `SizeY` | prop | nombre | LE | P2 | taille y |
| `SizeZ` | prop | nombre | LE | P2 | taille z |
| `OfsX` | prop | nombre | LE | P2 | position x |
| `OfsY` | prop | nombre | LE | P2 | position y |
| `OfsZ` | prop | nombre | LE | P1 | position z |
| `Size` | prop | `Vec3R` | L | P3 | `(SizeX, SizeY, SizeZ)` |
| `Offset` | prop | `Vec3R` | L | P3 | `(OfsX, OfsY, OfsZ)` |

#### Type `VideoAPI`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `SceneScale` | prop | nombre | LE | P2 | ≡ `Setting.SceneScale` (REQ-LUA-095) |

#### Type `ProfilAPI`

Global `Profile` (REQ-LUA-095 ; champs SPEC-03 REQ-PROF-022).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Title` | prop | chaîne | LE | P2 | titre (champ 8) |
| `Maker` | prop | chaîne | LE | P2 | auteur (champ 9) |
| `GameDev` | prop | chaîne | LE | P2 | éditeur du jeu (champ 10) |
| `Des` | prop | chaîne | LE | P3 | description (champ 11) |
| `Id` | prop | chaîne | L | P3 | identifiant (champ 7) |
| `Ver` | prop | chaîne | L | P3 | version du format (champ 2) |
| `IsV2Format` | prop | booléen | L | P3 | `Ver` = « 2 » (SPEC-03) |
| `RenewId` | méth | nil | — | P3 | nouvel identifiant de 12 caractères (SPEC-03 REQ-PROF-052), sans autre effet |

#### Type `ScriptPadAPI`

Global `ScriptPad` (REQ-LUA-091, REQ-LUA-032).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Enable` | méth | nil | — | P2 | `Enable(b: booléen)` |
| `PadIndex` | prop | entier | LE | P3 | joueur du pad (0 ou 1) |
| `State` | prop | entier | L | P3 | dernier masque renvoyé par `InputProcess` |
| `HandPos` | méth | `Vector3` | — | P2 | `HandPos(main)` : (0, 0, 0) |
| `RotPos` | méth | `Vector3` | — | P3 | `RotPos(main)` : (0, 0, 0) |
| `LeftHand` | prop | `XRState` | L | P2 | état nul |
| `LH` | prop | `XRState` | L | P3 | alias |
| `RightHand` | prop | `XRState` | L | P2 | état nul |
| `RH` | prop | `XRState` | L | P3 | alias |
| `Head` | prop | `XRState` | L | P2 | état nul |
| `Pressed` | méth | booléen | — | P3 | `Pressed(masque)` : `State` ET `masque` ≠ 0 |

#### Type `ManettesAPI`

Global `Gamepad` (REQ-LUA-092).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `[i]` | index | `Joueur` | L | P2 | joueur `i` (0 ou 1) |
| `State` | prop | entier | L | P3 | masque global (SPEC-08) |
| `Pressed` | méth | booléen | — | P3 | `Pressed(masque)` sur le masque global |
| `GetInput` | méth | entier | — | P3 | `GetInput(port)` : octet du port (SPEC-08) |

#### Type `Joueur`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `State` | prop | entier | L | P2 | masque d'actions du joueur (SPEC-08 §4.1) |
| `Pressed` | méth | booléen | — | P3 | `Pressed(masque)` |

#### Type `NotiAPI`

Global `NotiCenter` (REQ-LUA-093).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Notify` | fonc | nil | — | P2 | `Notify(msg [, durée [, gravité]])` |
| `Show` | fonc | nil | — | P3 | `Show(msg [, gravité])` |

#### Type `DonnéesPerso`

`Instance.CData` (REQ-LUA-082).

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `[i]` | index | nombre | LE | P1 | paramètre libre `i` ≥ 0 |

#### Type `SlotTexture`

`Instance.slot` (REQ-LUA-097). Objet opaque partagé entre l'instance de pattern et ses instances de frame.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `w` | prop | entier | L | P1 | largeur de la texture en pixels |
| `h` | prop | entier | L | P1 | hauteur de la texture en pixels |

#### Type `XRState`

Valeur (copie). Tous les champs valent (0, 0, 0) en desktop.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `pos` | prop | `Vector3` | LE | P2 | position |
| `vel` | prop | `Vector3` | LE | P2 | vitesse |
| `acc` | prop | `Vector3` | LE | P2 | accélération |
| `rot` | prop | `Vector3` | LE | P2 | rotation (degrés) |

### 4.3 Vecteurs liés, valeurs vectorielles et listes

#### Type `Vec3R`

Vecteur lié (référence) : `Offset`, `Scale`, `Rot`, `Pivot`, `DeformAmp`, `DeformPivot`, `Camera.Rot`, `ZapperOffset`, `Backdrop.Size`/`Offset`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | nombre | LE | P1 | composante x |
| `y` | prop | nombre | LE | P1 | composante y |
| `z` | prop | nombre | LE | P1 | composante z |
| `content` | prop | `Vector3` | LE | P1 | lecture : copie `Vector3` ; écriture : remplace les trois composantes |
| `[i]` | index | nombre | LE | P2 | `i` = 0, 1, 2 ≡ `x`, `y`, `z` |

#### Type `Vec2R`

Vecteur lié 2D : `texOffset`, `texTilting`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | nombre | LE | P1 | composante x |
| `y` | prop | nombre | LE | P1 | composante y |
| `content` | prop | `Vector2` | LE | P3 | lecture copie ; écriture des deux composantes |
| `[i]` | index | nombre | LE | P3 | `i` = 0, 1 |

#### Type `Mat4R`

Matrice 4×4 liée : `DeformSpeed`, `DeformOffset`. `mRC` = ligne R, colonne C.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `m00` | prop | nombre | LE | P1 | ligne 0, colonne 0 |
| `m01` | prop | nombre | LE | P1 | |
| `m02` | prop | nombre | LE | P3 | |
| `m03` | prop | nombre | LE | P3 | |
| `m10` | prop | nombre | LE | P1 | |
| `m11` | prop | nombre | LE | P1 | |
| `m12` | prop | nombre | LE | P3 | |
| `m13` | prop | nombre | LE | P3 | |
| `m20` | prop | nombre | LE | P1 | |
| `m21` | prop | nombre | LE | P1 | |
| `m22` | prop | nombre | LE | P3 | |
| `m23` | prop | nombre | LE | P1 | |
| `m30` | prop | nombre | LE | P3 | ≡ `CData[0]` |
| `m31` | prop | nombre | LE | P3 | ≡ `CData[1]` (resp. `CData[5]` pour `DeformOffset`) |
| `m32` | prop | nombre | LE | P3 | |
| `m33` | prop | nombre | LE | P3 | |
| `[i]` | index | nombre | LE | P3 | `i` = 0 à 15, ordre par colonnes : `i = R + 4·C` |

#### Type `Liste<T>`

Liste hôte (référence) renvoyée par l'API. `T` = `Instance` ou `Shape`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `Count` | prop | entier | L | P1 | nombre d'éléments |
| `[i]` | index | T | L | P1 | élément `i` (0-based), erreur hors bornes |
| `Contains` | méth | booléen | — | P3 | `Contains(x)` |
| `IndexOf` | méth | entier | — | P3 | `IndexOf(x)` : indice ou −1 |

#### Type `Vector3`

Valeur (copie), trois composantes réelles.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | nombre | LE | P1 | |
| `y` | prop | nombre | LE | P1 | |
| `z` | prop | nombre | LE | P1 | |
| `magnitude` | prop | nombre | L | P2 | `√(x² + y² + z²)` |
| `sqrMagnitude` | prop | nombre | L | P3 | `x² + y² + z²` |
| `normalized` | prop | `Vector3` | L | P3 | `v / |v|`, ou (0, 0, 0) si `|v|` ≤ 10⁻⁵ |

Opérateurs : `a + b`, `a − b`, `−a` composante par composante ; `a * k`, `k * a` (k nombre) ; `a / k` ; `a == b` vrai si `|a − b|² < 10⁻¹⁰` (`~=` : négation).

#### Type `Vector3Classe`

Global `Vector3`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `()` | appel | `Vector3` | — | P1 | `Vector3(x, y [, z])`, `z` = 0 par défaut ; `Vector3()` = (0, 0, 0) |
| `zero` | prop | `Vector3` | L | P1 | (0, 0, 0) |
| `one` | prop | `Vector3` | L | P2 | (1, 1, 1) |
| `back` | prop | `Vector3` | L | P2 | (0, 0, −1) |
| `forward` | prop | `Vector3` | L | P3 | (0, 0, 1) |
| `up` | prop | `Vector3` | L | P3 | (0, 1, 0) |
| `down` | prop | `Vector3` | L | P3 | (0, −1, 0) |
| `left` | prop | `Vector3` | L | P3 | (−1, 0, 0) |
| `right` | prop | `Vector3` | L | P3 | (1, 0, 0) |
| `Cross` | fonc | `Vector3` | — | P2 | `Cross(a, b)` = `(a.y·b.z − a.z·b.y, a.z·b.x − a.x·b.z, a.x·b.y − a.y·b.x)` |
| `Dot` | fonc | nombre | — | P3 | `Dot(a, b)` |
| `Distance` | fonc | nombre | — | P3 | `Distance(a, b)` = `|a − b|` |
| `Lerp` | fonc | `Vector3` | — | P3 | `Lerp(a, b, t)` = `a + (b − a)·clamp(t, 0, 1)` |

#### Type `Vector2`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | nombre | LE | P1 | |
| `y` | prop | nombre | LE | P1 | |
| `magnitude` | prop | nombre | L | P3 | `√(x² + y²)` |

Opérateurs comme `Vector3` (deux composantes).

#### Type `Vector2Classe`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `()` | appel | `Vector2` | — | P3 | `Vector2(x, y)` |
| `zero` | prop | `Vector2` | L | P3 | (0, 0) |
| `one` | prop | `Vector2` | L | P3 | (1, 1) |

#### Type `Quaternion`

Valeur (copie), `(x, y, z, w)`, rotation unitaire.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | nombre | LE | P3 | |
| `y` | prop | nombre | LE | P3 | |
| `z` | prop | nombre | LE | P3 | |
| `w` | prop | nombre | LE | P3 | |
| `eulerAngles` | prop | `Vector3` | L | P2 | angles `(ax, ay, az)` en degrés, chacun ramené dans `[0, 360)`, tels que `Quaternion.Euler(ax, ay, az)` représente la même rotation ; pour `ax`, la valeur principale `asin` (dans `[−90, 90]` avant normalisation) |

Opérateurs : `q1 * q2` = produit de Hamilton (`q2` appliquée d'abord, puis `q1`) ; `q * v` (v `Vector3`) = rotation de `v` par `q` ; `q1 == q2` vrai si `|q1·q2| > 1 − 10⁻⁶` (produit scalaire 4D).

#### Type `QuaternionClasse`

Global `Quaternion`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `identity` | prop | `Quaternion` | L | P2 | (0, 0, 0, 1) |
| `Euler` | fonc | `Quaternion` | — | P2 | `Euler(v: Vector3)` ou `Euler(x, y, z)` : rotation de `z` degrés autour de l'axe z, puis `x` autour de l'axe x, puis `y` autour de l'axe y (`Euler = Ry(y) * Rx(x) * Rz(z)`), avec `Rₐ(θ) = (sin(θ/2)·â, cos(θ/2))` |
| `AngleAxis` | fonc | `Quaternion` | — | P2 | `AngleAxis(angle, axe)` : rotation de `angle` degrés autour de `axe` normalisé ; axe nul → `identity` |
| `Lerp` | fonc | `Quaternion` | — | P2 | `Lerp(a, b, t)` : `t` ramené dans `[0, 1]` ; `b'` = `b` si `a·b` ≥ 0, sinon `−b` ; résultat = normalisation de `a·(1 − t) + b'·t` |
| `Slerp` | fonc | `Quaternion` | — | P3 | interpolation sphérique, `t` ramené dans `[0, 1]` |

#### Type `Color`

Valeur (copie), composantes réelles 0–1.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `r` | prop | nombre | LE | P3 | rouge |
| `g` | prop | nombre | LE | P3 | vert |
| `b` | prop | nombre | LE | P3 | bleu |
| `a` | prop | nombre | LE | P3 | alpha |

Opérateurs : `==` (égalité exacte des quatre composantes), `+`, `−`, `*` (par un nombre ou composante par composante).

#### Type `ColorClasse`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `()` | appel | `Color` | — | P1 | `Color(r, g, b [, a])`, `a` = 1 par défaut |
| `black` | prop | `Color` | L | P3 | (0, 0, 0, 1) |
| `white` | prop | `Color` | L | P3 | (1, 1, 1, 1) |

#### Type `IntVector2`

Valeur (copie), deux entiers.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | entier | LE | P1 | |
| `y` | prop | entier | LE | P1 | |

Opérateurs : `+`, `−` composante par composante ; `==` égalité des deux composantes ; `a / k` (k entier) : division entière tronquée de chaque composante.

#### Type `IntVector2Classe`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `()` | appel | `IntVector2` | — | P2 | `IntVector2(x, y)` (troncature) ; sans argument (0, 0) |
| `zero` | prop | `IntVector2` | L | P3 | (0, 0) |
| `one` | prop | `IntVector2` | L | P3 | (1, 1) |

#### Type `IntVector3Classe`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `()` | appel | `IntVector3` | — | P3 | `IntVector3(x, y, z)` |

#### Type `IntVector3`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | entier | LE | P3 | |
| `y` | prop | entier | LE | P3 | |
| `z` | prop | entier | LE | P3 | |

#### Type `Rect`

Valeur (copie) : origine `(x, y)` et taille `(width, height)`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `x` | prop | nombre | LE | P3 | |
| `y` | prop | nombre | LE | P3 | |
| `width` | prop | nombre | LE | P3 | |
| `height` | prop | nombre | LE | P3 | |
| `Overlaps` | méth | booléen | — | P3 | `Overlaps(r)` : chevauchement strict (`xMax > r.x`, `x < r.xMax`, `yMax > r.y`, `y < r.yMax`, avec tailles négatives normalisées) |

#### Type `RectClasse`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `()` | appel | `Rect` | — | P2 | `Rect(x, y, largeur, hauteur)` |

### 4.4 Énumérations et constantes

#### Type `TLayerEnum`

Global `TLayer` ; indice de `Setting.Layer[i]`.

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `UNIDENTIFIED` | const | entier | L | P3 | 0 : non classée (couche effective : L1 pour un sprite, `Layer.DefLayer` pour le fond) |
| `UI` | const | entier | L | P3 | 1 : interface (sans ombre) |
| `L1` | const | entier | L | P2 | 2 |
| `L2` | const | entier | L | P2 | 3 |
| `L3` | const | entier | L | P3 | 4 |
| `L4` | const | entier | L | P3 | 5 |
| `COUNT` | const | entier | L | P3 | 6 |

#### Type `AlignEnum`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `FRONT` | const | entier | L | P3 | 0 |
| `CENTER` | const | entier | L | P3 | 1 |
| `BACK` | const | entier | L | P3 | 2 |
| `COUNT` | const | entier | L | P3 | 3 |

#### Type `CamModeEnum`

| Membre | Genre | Type | Accès | P | Effet |
|---|---|---|---|---|---|
| `TPV` | const | entier | L | P3 | 0 : vue troisième personne |
| `FPV` | const | entier | L | P3 | 1 : vue subjective |

Autres valeurs entières sans table globale : **type de géométrie** (`geo`) : 0 `Default`, 1 `HCYLINDER`, 2 `VCYLINDER`, 3 `CUBE`, 4 `HALFHCYLINDER`, 5 `HALFVCYLINDER` (SPEC-04B) ; **type de port** : 0 Manette, 1 Zapper ; **gravité de notification** : 0 normale, 1 avertissement, 2 erreur ; **masques d'actions** : SPEC-08 §4.1 (A = 1, B = 2, Select = 4, Start = 8, Haut = 16, Bas = 32, Gauche = 64, Droite = 128, puis actions étendues à partir de 256).

### 4.5 Hooks

| Hook | Arguments | Retour | Appelé | Conditions | Exigence |
|---|---|---|---|---|---|
| `Start` | — | ignoré | démarrage de partie | non en erreur | REQ-LUA-020 |
| `End` | — | ignoré | fin de partie | non en erreur | REQ-LUA-021 |
| `Update` | — | ignoré | chaque frame, étape 2 | actif | REQ-LUA-025 |
| `UpdateS` | — (globales `shape`, `shape2D`) | ignoré | chaque frame, étape 4, une fois par instance liée | actif à l'étape 3 | REQ-LUA-025 |
| `LateUpdate` | — | ignoré | chaque frame, étape 5 | actif | REQ-LUA-025 |
| `BuildPattern` | — (globale `pattern`) | ignoré | construction d'une instance de pattern liée | actif | REQ-LUA-030 |
| `Save` | — | ignoré | avant capture d'un savestate | actif | REQ-LUA-071 |
| `Load` | — | ignoré | après `Start`, au chargement d'un savestate | actif | REQ-LUA-072 |
| `InputProcess` | `index` (entier) | masque (entier) | avant chaque frame émulée, pad de script activé | actif, dernier script chargé qui le définit | REQ-LUA-032 |
| `DetectShape` | — | — | jamais | — | REQ-LUA-031 |

### 4.6 Bloc des données persistantes

| Octets | Contenu |
|---|---|
| 0–3 | `PData[1]`, réel IEEE-754 32 bits petit-boutiste |
| 4·(k−1) … 4·k−1 | `PData[k]`, k = 1 … 64 |
| 252–255 | `PData[64]` |

Taille fixe : 256 octets.

## 5. Vecteurs de test

### 5.1 Couverture de l'API par le corpus (informatif)

Cette section est **informative** (DEC-R2-T7) : elle rapporte une mesure faite par l'outillage du côté sale, qui n'est jamais un critère d'acceptation. Les critères correspondants (REQ-LUA-004, 051, 100) sont exécutables sur `assets-neutral/profiles`. Outils (côté sale) : `lua_corpus.py` (extraction exacte des 579 scripts depuis le champ 3 des profils), `lua_api_check.py` (analyse lexicale du corpus, résolution des chaînes d'accès par inférence de type à partir des globales du §4.1, contrôle de chaque membre contre les tables du §4.2–4.4, contrôle du genre d'appel `.`/`:`), `lua_collisions.py`, `script_order.ps1`, `lua_dialect_probe.ps1`, `lua54_probe.py`.

Résultats de `lua_api_check.py` sur cette version du document (rapport complet : `tools/out/lua_api_check.txt`) :

| Mesure | Valeur |
|---|---|
| Scripts / profils analysés | 579 / 89 |
| Lignes de tables lues dans ce document | 42 globales, 46 types, 370 membres |
| Globales de l'hôte lues par le corpus | 27 noms, tous au §4.1 : `Backdrop`, `Camera`, `Color`, `Frame`, `Gamepad`, `IntVector2`, `Light`, `NotiCenter`, `PData`, `Palette`, `Pattern`, `Profile`, `Quaternion`, `Rect`, `Script`, `ScriptPad`, `Setting`, `TLayer`, `Vector3`, `Video`, `ZapperOffset`, `frame`, `pattern`, `print`, `shape`, `shape2D`, `time` |
| Accès membres typés par inférence / non typés | 6 350 / 3 047 |
| Couples (type, membre) distincts résolus | 189, **0 manquant** |
| Appels avec le mauvais séparateur (`.` / `:`) | 0 |
| Noms de membres sur valeurs non typées | 81 distincts : 71 présents dans les tables du §4, 10 champs de tables Lua des scripts (`delta`, `delta1`, `dis`, `ex`, `ey`, `id`, `s`, `start`, `sx`, `sy`), 0 inconnu ; fautes de frappe `hiden`/`Hiden` documentées (REQ-LUA-042) |
| Globales lues jamais affectées | `null`, `nill`, `state` : fautes des profils, valent nil (comportement Lua ordinaire) |
| Lignes de membres P1 / P2 / P3 observées dans le corpus | P1 105 sur 106, P2 92 sur 93, P3 34 sur 161 ; les 2 lignes P1/P2 non observées par l'outil sont utilisées via des valeurs non typables (`Instance.CalculatePos` dans « Excitebike », `Pattern[i]` dans « City Connection » et « Chip n Dale Rescue Rangers (U) ») |

### 5.2 Vecteurs

**TV-LUA-01 — Ordre des hooks.** Profil de test à deux scripts `b` (tags `t`) et `a` (tags `*`), dans cet ordre dans le fichier ; une trame à deux instances scriptables `i1` (tag `t`) et `i2` (tag `u`). Chaque hook ajoute son nom à une table globale `log`. Attendu après une frame : `a.Update, b.Update, a.UpdateS(i1), a.UpdateS(i2), b.UpdateS(i1), a.LateUpdate, b.LateUpdate`. Chunks exécutés dans l'ordre `b`, `a` ; `Start` appelés dans l'ordre `a`, `b`.

**TV-LUA-02 — Collision de fonctions.** « Super Spike V Ball (U) » chargé : la globale `Connect` est la fonction définie par le chunk de `foot_body` (paramètres `foot, body`), et non celle de `bg` ; le `UpdateS` de `bg` appelle donc la version de `foot_body` (vérifiable en comparant `Connect` à une copie capturée à la fin de l'exécution de chaque chunk dans un harnais de test).

**TV-LUA-03 — Script désactivé.** « KungFu » : après lancement, `Setting.Layer[3].CastShadow` = vrai ; aucune erreur ; `UpdateS` de `hud` jamais appelé.

**TV-LUA-04 — Données persistantes.** Script de test dont `Save` fait `PData[1] = 0.1; PData[0] = 7; PData[2] = "12"; PData[3] = true; PData[4] = nil`. Sauvegarder (quicksave), puis exécuter `PData[0] = 99; PData[1] = 5`, puis recharger le quicksave (même jeu, état Lua conservé). Attendu au moment de `Load` : `PData[1]` = 0,100000001490116 (valeur simple précision), `PData[2]` = 12, `PData[3]` = 0, `PData[4]` = 0, `PData[0]` = 99 (non enregistré, non restauré), `math.type(PData[2])` = `"float"`.

**TV-LUA-05 — Positions.** Instance de frame d'une shape de fond `pStart = (16, 32)`, `pEnd = (31, 47)`, `sizeZ = 8`, couche L1 (calque : `Offset` = 0, alignement FRONT), `Offset = (0, 0, 0)`, `Pivot = (0,5 ; 0,5 ; 0,5)`, `DeformPivot = (0,5 ; 0,5 ; 0,5)`. Attendu : `BottomLeft = (16, 201, −4)`, `TopRight = (31, 216, 4)`, `BL = (16, 200, −4)`, `TR = (31, 215, 4)`, `size = (16, 16, 8)`, `OriPos = (23,5 ; 208,5 ; 0)`, `GetPos()` = `(24, 208, 4)` (SPEC-04 REQ-E3D-205 : coin bas `(16, 200, 0)`, coin haut `(32, 216, 8)`).

**TV-LUA-06 — Overlap.** Shapes A : `pStart (0, 0)`, `pEnd (7, 7)` ; B : `pStart (7, 0)`, `pEnd (14, 7)` ; `iA`, `iB` leurs instances 0. Attendu : `iA:Overlap(iB)` → faux (`7 > 7` est faux) ; `iA:Overlap(iB, 1)` → vrai ; B en `pStart (9, 0)`, `pEnd (16, 7)` : `iA:Overlap(iB, 1)` → faux (`8 > 9` est faux).

**TV-LUA-07 — Déformation.** Instance TV-LUA-05 avec `DeformAmp = (0, 0, 1)`, `DeformSpeed` nulle sauf `m23 = 2`, `DeformOffset` = π/2 sur les lignes 0 à 2 et 0 sur la ligne 3, `time = 0,25` s : `GetDeform(OriPos)` = `(0, 0, sin(π/2)³ · sin(0,5 + π/2) · 1 · 8)` = `(0 ; 0 ; 7,02066)` (± 10⁻⁴).

**TV-LUA-08 — Vecteurs.** `Vector3(1, 2) == Vector3(1, 2, 0)` → vrai ; `Vector3.Cross(Vector3.back, Vector3(1, 0, 0))` = `(0, −1, 0)` ; `Vector3(3, 4, 0).magnitude` = 5 ; `(Vector3(1, 1, 1) * 2).y` = 2.

**TV-LUA-09 — Quaternions.** `Quaternion.Euler(0, 90, 0).eulerAngles` = `(0, 90, 0)` (±10⁻⁴) ; `(Quaternion.Euler(0, 0, 30) * Quaternion.Euler(0, 0, 60)).eulerAngles.z` = 90 ; `Quaternion.Euler(-30, 0, 0).eulerAngles.x` = 330 ; `(Quaternion.AngleAxis(90, Vector3(0, 1, 0)) * Vector3(1, 0, 0))` = `(0, 0, −1)` ; `Quaternion.Lerp(Quaternion.identity, Quaternion.Euler(0, 90, 0), 2).eulerAngles.y` = 90.

**TV-LUA-10 — Couleurs et palette.** `Color(1, 0, 0).a` = 1 ; trame TV-E3D-09 (SPEC-04), frame suivante sans changement de palette : `Palette.FrameColor(5)` = `Color(252/255, 56/255, 0, 1)`.

**TV-LUA-11 — Nombres.** `math.type(frame)` = `"float"` ; `print(frame)` à la frame 120 → `120` ; `math.fmod(5, Frame.shapeCount)` sans instance → `nan` (affiché `nan` ou `-nan`), sans erreur ; `Script:ReadMem(0x6C + 0.5)` = `Script:ReadMem(0x6C)`.

**TV-LUA-12 — Erreurs.** Script `e` dont `Update` vaut `function Update() local x = nil; return x.y end` : à la frame 1, une ligne de journal contenant `e`, `Update` et `attempt to index` ; aux frames suivantes, plus aucun appel de `e` ; le jeu continue ; un reset du même jeu n'appelle ni `End` ni `Start` de `e`.

**TV-LUA-13 — Membres inconnus.** `shape.hiden` → nil ; `shape.NoSuch = 1` → erreur `cannot set NoSuch, no such field` ; `Frame:GetShape(Frame.shapeCount)` → erreur.

**TV-LUA-14 — Liste partagée.** Trame à 3 instances de tag `a` et 1 de tag `b` : `local x = Frame:GetShapesWithTag("a"); local n = x.Count; Frame:GetShapesWithTag("b")` → `n` = 3, `x.Count` = 1.

**TV-LUA-15 — Chargement et réglages.** Profil de test dont les réglages ont `EmptyTile` faux et dont le script a `Setting.EmptyTile = true` au niveau principal → `Setting.EmptyTile` faux au premier `Update` (REQ-LUA-015).

**TV-LUA-16 — Savestate complet** [ROM]. « DQ3_DW3 » en scène 2 (combat) : sauvegarder dans un slot, quitter vers le menu, charger le slot → séquence journalisée `End` (tous), [rechargement du profil si changement de jeu], `Start` (tous, dans l'ordre), `Load` (actifs, dans l'ordre) ; `scene` = 2 et `Setting.EmptyTile` conforme à la scène dès la première frame.

### 5.3 Profils embarqués qui exercent chaque règle

| Règle | Profils |
|---|---|
| globales partagées entre scripts (010) | Adventures of Lolo 1/2/3, Super Mario Bros 3 (U) (PRG1), Jackal (U), DQ3_DW3, Zelda II |
| collisions de fonctions (012) | Super Spike V Ball (U), Dragon Warrior (U) (PRG1), Jackal (U) |
| script désactivé dont `Start` agit (007) | KungFu, Jackie Chan s Action Kung Fu (U), Jackal (U) (`ov`) |
| activation d'un script par un autre (007) | Jackal (U) |
| ordre non ordinal (004) | Excitebike |
| `BuildPattern` (030) | empty (2D Mode), Mega Man 2 |
| `Save`/`Load` et `PData` (071–072) | Blaster Master (U), DQ3_DW3, Dragon Warrior II (U), Excitebike, Final_Fantasy_USA, Life Force (U), punchout, Super Spike V Ball (U), Zelda II |
| `PData[0]` non persistant (071) | Blaster Master (U), DQ3_DW3, Dragon Warrior II (U), Final_Fantasy_USA, Life Force (U), punchout, Zelda II |
| `InputProcess`, `ScriptPad` (032, 091) | flappy_fpv, flappy_tpv, punchout |
| référence `Camera.Rot` gardée (044) | 11 profils, ex. Antarctic Adventure (J), Excitebike, DQ3_DW3, Legend Of Zelda |
| `slot` réaffecté (097) | 38 profils, ex. Adventures of Lolo 1/2/3, Super Mario Bros 3, City Connection, Mega Man 2 |
| `ReadMem` (094) | Duck Hunt (World), Excitebike, Jackal (U), Lunar Pool, Mega Man 1, Mega Man 2, Super Mario Bros, Super Spike V Ball (U), punchout |
| liste partagée (048) | Excitebike, Super Spike V Ball (U) |
| notifications (093) | punchout |
| métadonnées de profil (095) | Tetris-Tengen |
| `goto` (100) | Burger Time, Jackal (U) (13 occurrences de `goto`/étiquettes) |

## 6. Questions ouvertes

- **Q-05-1** — *Close par DEC-05-Q1*. Dialecte : la décision PLAN.md (Lua 5.4) reste valable (tout le corpus compile et les écarts sont neutralisés par REQ-LUA-040, 052 et 101), mais la justification « proche du Lua 5.3 de l'original » est inexacte : l'original exécute un moteur compatible 5.1 (constat boîte noire). Faut-il amender PLAN.md ? Alternative possible : un moteur compatible 5.1 — non recommandée (Lua 5.4 suffit et la décision P1 est prise).
- **Q-05-2** — *Close par DEC-05-Q2* (compteur à 0 au lancement du jeu, D). Origine du compteur `frame` : SPEC-04 REQ-E3D-043 fixe « frames depuis le démarrage du jeu » ; l'original compte les frames émulées depuis le lancement du programme, sans remise à zéro entre parties. Seule la phase des alternances `frame % 2` (Tetris-*, Road Fighter, « Super Spike V Ball ») en dépend ; sans effet visible durable. À arbitrer (proposition : garder SPEC-04).
- **Q-05-3** — *Close par DEC-05-Q3* (D). Recréation de l'état Lua à chaque changement de profil (REQ-LUA-014) : écart assumé avec l'original (une seule machine Lua pour la session). À confirmer par le juge (round 2) sur l'enchaînement de plusieurs jeux.
- **Q-05-4** — *Close par DEC-05-Q4* (retard d'une frame, fidèle). `Palette.FrameColorIndex` sans second argument lit la frame publiée, en retard d'une frame (REQ-LUA-087) ; 60 appels du corpus en dépendent. Dans l'original le retard varie selon la file de rendu (0 à 2 frames). La spec fixe 1 frame ; vérifier visuellement sur « Super Mario Bros 3 (U) (PRG1) » et « Mega Man 2 » au round 2.
- **Q-05-5** — *Close par DEC-05-Q5* (fidélité). `UpdateS` des scripts désactivés pendant l'étape 4 (REQ-LUA-025, notes) et état d'erreur conservé lors d'une relance du même jeu (REQ-LUA-062) : fidèles à l'original ; un profil utilisateur pourrait préférer qu'un reset efface les erreurs. Proposition : fidélité.
- **Q-05-6** — *Close par DEC-05-Q6* (seuls les membres du §4 sont requis). L'original expose davantage de membres que le §4 (pratiquement tous les membres publics de ses objets et des types de vecteurs) ; seuls les membres utiles ou cohérents ont été retenus. Un profil utilisateur qui en utilise d'autres obtiendra nil (REQ-LUA-042). À réévaluer si des profils utilisateur réels sont fournis.

## Historique

- 1.1-draft (AMD-001, relecture R2) : critères REQ-LUA-004/051/100 rendus exécutables sur assets-neutral (DEC-R2-T7), §5.1 informatif ; portée ASCII de la collation (REQ-LUA-004) ; REQ-LUA-075 alignée sur DEC-R2-D5 (hooks Save/Load en libretro) ; Video.SceneScale aligné sur REQ-SCN-002 (vérifié) ; écarts fidèles Overlap/OverlapRect et GetDeform documentés ; pattern de frame défini ; instrumentation de REQ-LUA-111 ; étiquettes [ROM] ; questions closes.
