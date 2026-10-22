# SPEC-09 — Persistance : réglages, bibliothèque, sauvegardes, emplacements de fichiers
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00 (glossaire), SPEC-02 (cœur, savestate du cœur, régions), SPEC-03 (format de profil, message de réglages embarqué, association ROM ↔ profil), SPEC-04 (conversion 2D→3D : paramètres de conversion), SPEC-05 (Lua : objets `Setting`, `Light`, `Layer`, `Clipping`, données persistantes), SPEC-06 (scène : caméra, lumière, ombres, skybox, TV), SPEC-07 (boucle : vitesse, région, savestates, slots, rollback), SPEC-08 (entrées : bindings), SPEC-10 (UI : panneaux de réglages, bibliothèque, notifications), SPEC-11 (audio)

---

## 1. Objet et périmètre

Ce document fixe :
1. le **modèle de réglages** complet : chaque réglage, son nom visible depuis Lua, son type, sa valeur par défaut, son domaine, sa portée, le moment où il prend effet et la spec qui définit son comportement (§3.1 et tableaux §4.1) ;
2. ce qui est **mémorisé par jeu** (bibliothèque) et la clé utilisée ;
3. les **slots de savestate** sur disque : nombre, nommage, vignettes, métadonnées, suppression, contenu requis ;
4. la persistance des profils utilisateur, de la RAM sauvegardée par pile, des bindings et des petits états d'interface ;
5. les **emplacements de fichiers** : disposition observable de l'original (informative) et disposition exigée pour la réimplémentation, premier lancement, fichiers corrompus, versions et migrations ;
6. les contraintes qui permettront d'exposer le modèle en **core options libretro**.

Décision de cadrage (PLAN.md) : **aucune compatibilité binaire** avec les fichiers de réglages, de bibliothèque ou de sauvegarde de l'original n'est exigée. La réimplémentation choisit ses formats (JSON recommandé pour tout ce qui est éditable, binaire libre pour les savestates). En revanche, le **modèle** (liste, types, défauts, bornes, portées, effets) est exhaustif, car les profils embarqués et leurs scripts Lua lisent et écrivent ces réglages (exemple réel : `Setting.Layer[3].CastShadow = true`).

Une exception d'interopérabilité subsiste : les profils (fichiers `.bytes` / `.3dn`, SPEC-03) **embarquent un message de réglages**. Les numéros de champ de ce message sont donnés dans la colonne « N° » des tableaux §4.1 pour permettre à SPEC-03 de le décoder. Leur encodage (couleurs, vecteurs, listes) relève de SPEC-03.

Hors périmètre : la VR (les réglages VR sont listés, lus, conservés mais sans effet, REQ-PERS-011), l'éditeur de profils et ses réglages propres (listés pour exhaustivité, conservés sans effet), l'enregistrement de films, l'export de scène en OBJ.

## 2. Concepts et vocabulaire

- **Réglage** : une valeur nommée du modèle, identifiée par une **clé stable** (colonne « Clé » des tableaux §4.1, ex. `camera.fov`).
- **Portée** d'un réglage :
  - **U — préférence utilisateur** : globale à l'application, choisie par l'utilisateur, persistée dans le fichier de réglages, **conservée** quand un profil est chargé ;
  - **P — paramètre de scène du profil** : sa valeur courante est **remplacée par la valeur embarquée dans le profil** quand on charge un jeu différent du précédent (REQ-PERS-005) ; les scripts Lua la modifient en cours de partie ; l'original l'écrit aussi dans son fichier de réglages, mais cette valeur n'a pas d'effet observable puisqu'elle est écrasée au chargement suivant ;
  - **S — session** : jamais persisté ; valeur initiale au lancement du programme ;
  - **D — dérivé** : lecture seule, calculé à partir d'autres réglages ou de l'état de la plateforme ;
  - **J — par jeu** : mémorisé dans la bibliothèque, pour une entrée de jeu (§3.3).
- **Jeu** (entrée de bibliothèque) : un profil (embarqué ou utilisateur) et ses données mémorisées. Il est identifié par l'**identifiant de profil** (chaîne, SPEC-03). Le jeu particulier **« 2D Mode »** est un profil générique utilisable avec n'importe quelle ROM.
- **Slot** : emplacement de savestate. 36 slots numérotés `1`…`36` et un slot rapide `quick` (SPEC-07).
- **Racine utilisateur** : dossier où la réimplémentation range toutes ses données modifiables (REQ-PERS-070).
- **Couche de session** : valeur temporaire d'un réglage U posée par un script Lua pendant une partie (REQ-PERS-010).
- Les indices Lua des tableaux exposés (`Layer[i]`, `Clipping[i]`) sont des **entiers à base 0**, comme dans l'original.

## 3. Exigences

### 3.1 Modèle de réglages

**REQ-PERS-001** (P1) — L'implémentation DOIT fournir un modèle de réglages contenant **chacun** des réglages des tableaux §4.1.1 à §4.1.10, avec sa clé stable, son type, sa valeur par défaut, son domaine et sa portée tels qu'indiqués.
- Critère d'acceptation : l'énumération programmatique du modèle renvoie au moins toutes les clés des tableaux §4.1 ; pour chacune, type, défaut et domaine sont égaux à ceux du tableau.
- Notes : la réimplémentation PEUT ajouter des réglages, à condition de leur donner une clé nouvelle (REQ-PERS-080).

**REQ-PERS-002** (P1) — Premier lancement : en l'absence de fichier de réglages, chaque réglage DOIT prendre la valeur de la colonne « Défaut ».
- Critère d'acceptation : racine utilisateur vide → au démarrage, `camera.fov` = 70, `video.frame_rate` = 90, `scene.skybox` = −1, `audio.volume` = 1, `render.shadow` = vrai, `light.direction` = (1, −1, 1), `layer[2].offset` = 0, `layer[5].offset` = 36.

**REQ-PERS-003** (P1) — Validation : une valeur hors domaine, qu'elle vienne d'un fichier, de l'UI ou d'un script, DOIT être ramenée dans le domaine selon la règle de la colonne « Domaine » ; à défaut de règle, la valeur par défaut DOIT être utilisée. Règles fixées par l'original :
1. `video.frame_rate` différent de 60, 90 et 120 → 90 (appliqué au démarrage) ;
2. `emu.core` → reste de la division par le nombre de cœurs disponibles ; ce nombre vaut au moins 1 (un cœur est toujours fourni, SPEC-02 REQ-CORE-120) et vaut exactement 1 en v1, si bien que la valeur effective est toujours 0 ;
3. `emu.game_speed` hors [−8 ; 8] → borné (les commandes ±1 de l'UI bornent à −8 et 8) ;
4. `conv.sprite_tolerance` négatif → valeur absolue.
- Critère d'acceptation : fichier contenant `video.frame_rate = 75` → valeur effective 90 ; `emu.game_speed = 12` → 8 ; `conv.sprite_tolerance = −3` → 3.

**REQ-PERS-004** (P1) — Chaque réglage énuméré DOIT accepter, depuis Lua, **la valeur numérique** de son énumération (colonne « Domaine ») aussi bien que la constante nommée correspondante exposée par SPEC-05.
- Critère d'acceptation : le script `Setting.Layer[2].Aligned = 2` donne au calque 2 l'alignement « arrière » (BACK) ; `Setting.Layer[2].Aligned = Align.BACK` produit le même état.

**REQ-PERS-005** (P1) — Chargement d'un profil : quand un jeu **différent du jeu précédemment chargé** est lancé (ou chargé depuis un slot), ou quand le jeu est « 2D Mode », l'implémentation DOIT :
1. remplacer la valeur de **tous** les réglages de portée P par les valeurs du message de réglages embarqué dans le profil (SPEC-03). L'ensemble des réglages de portée P du §4.1 est **exactement** la liste de SPEC-03 REQ-PROF-041, qui fait foi en cas d'écart ;
2. laisser inchangés tous les réglages de portée U, y compris l'**intensité** de la lumière (alors que sa direction et sa couleur sont de portée P) ;
3. forcer `scene.use_nes_bg_color` (`Setting.BgColor`) à **vrai**, quelle que soit la valeur du profil ;
4. réinitialiser les drapeaux d'ombre des six calques, **quelles que soient** les valeurs stockées dans le profil : `CastShadow` vrai pour le seul calque 2 (L1) ; `RecvShadow` vrai pour les calques 2, 3, 4 et 5, faux pour 0 et 1 ;
5. exécuter ensuite le hook Lua de démarrage (SPEC-05), qui peut modifier ces valeurs.
- Critère d'acceptation : régler l'intensité lumineuse à 1,3 et le fond personnalisé à rouge, lancer un profil dont le message de réglages porte une intensité 0,8, une direction (0, −1, 1), `BgColor` faux, `CastShadow` vrai sur tous les calques → après chargement et avant le hook de démarrage : intensité 1,3, direction (0, −1, 1), fond personnalisé rouge, `BgColor` vrai, `CastShadow` = [faux, faux, vrai, faux, faux, faux], `RecvShadow` = [faux, faux, vrai, vrai, vrai, vrai].
- Notes : l'étape 4 explique pourquoi les profils embarqués réactivent les ombres de leurs calques par script (`Setting.Layer[3].CastShadow = true`, 26 occurrences). Si le profil ne contient pas de message de réglages exploitable, le comportement est défini par SPEC-03.

**REQ-PERS-006** (P2) — Relancer **le même jeu** (reset, relance depuis la bibliothèque, chargement d'un slot du même jeu) NE DOIT PAS réappliquer les valeurs P du profil : les valeurs modifiées par les scripts pendant la partie précédente de ce jeu sont conservées. Le jeu « 2D Mode » fait exception (rechargé à chaque fois, REQ-PERS-005).
- Critère d'acceptation : jeu A dont un script met `Setting.EmptyTile = true` en cours de partie seulement (pas dans le hook de démarrage) ; quitter vers le menu, relancer A → `Setting.EmptyTile` vaut vrai avant toute exécution de script ; lancer B puis A → valeur du profil A.

**REQ-PERS-007** (P1) — Exposition Lua : les globales `Setting`, `Light`, `Layer` et `Clipping` (SPEC-05) DOIVENT désigner le modèle **vivant** : `Light` ≡ `Setting.Light`, `Layer` ≡ `Setting.Layer`, `Clipping` ≡ `Setting.Clipping`. Toute écriture depuis Lua DOIT être visible immédiatement par la lecture suivante (Lua, UI, rendu) et prendre effet selon la colonne « Effet ». Les membres marqués « lecture » dans la table §4.2 NE DOIVENT PAS être assignables (une tentative est une erreur Lua, SPEC-05). `Layer[i]` accepte un entier 0 à 5 ou une constante de calque (`TLayer`, SPEC-05) et renvoie l'objet calque, non réassignable ; ses champs le sont. `Clipping[i]` accepte un entier 0 à 3 en lecture et en écriture, et équivaut à `x`, `y`, `z`, `w`.
- Critère d'acceptation : `Clipping[1] = 16` puis lecture de `Setting.Clipping.y` → 16 ; `Layer[3].Offset = 24` puis `Setting.Layer[3].Offset` → 24 et le rendu décale le calque L2 dès la frame suivante ; `Setting.Light = nil` → erreur Lua sans plantage.

**REQ-PERS-008** (P1) — Les noms Lua de la colonne « Lua » des tableaux §4.1 et de la table §4.2 font partie du contrat d'interopérabilité : ils DOIVENT être exposés exactement (casse comprise), y compris les **alias** `Setting.SamePalette` (≡ `SameBgPalette`) et `Setting.TopDownCam` (≡ `LocalCam`).
- Critère d'acceptation : les 22 chemins d'accès utilisés par les profils embarqués (liste §5.6) s'exécutent sans erreur sur un profil de test.

**REQ-PERS-009** (P2) — Notification de changement : toute modification effective d'un réglage (valeur différente de la précédente) DOIT émettre un événement portant la clé du réglage, afin que les sous-systèmes appliquent l'effet décrit en colonne « Effet ». Une écriture de la même valeur NE DOIT PAS émettre d'événement.
- Critère d'acceptation : abonné de test ; écrire `camera.fov` = 80 → un événement `camera.fov` ; réécrire 80 → aucun événement.

**REQ-PERS-010** (P2) — Écritures Lua sur des réglages U : dans l'original, quand un script modifie un réglage de portée U (cas réels : `Setting.CustomBgColor`, `Setting.GameSpeed`), la nouvelle valeur devient la préférence et elle est enregistrée à la sortie. La réimplémentation DEVRAIT appliquer ces écritures comme une **couche de session** : la valeur effective (lue par Lua, par le rendu et affichée par l'UI) est celle du script jusqu'à la fin de la partie (arrêt du jeu ou changement de jeu), puis la préférence de l'utilisateur est rétablie ; la préférence persistée ne change que par l'UI.
- Critère d'acceptation : fond personnalisé utilisateur = bleu ; un script le met à la couleur de palette 0x0F ; quitter le jeu → fond personnalisé = bleu ; relancer le programme → bleu.
- Notes : écart volontaire (classe D) : il évite qu'un profil modifie durablement la préférence de l'utilisateur. Voir Q-3.

**REQ-PERS-011** (P1) — Réglages VR (desktop uniquement) : `Setting.VrRuntime` DOIT toujours valoir 0 en lecture et toute écriture DOIT être ignorée sans erreur ; `Setting.SceneRatio` DOIT valoir 1 en lecture tant que `VrRuntime` vaut 0 (la valeur écrite est conservée, sans effet géométrique ; une écriture qui change la valeur conservée déclenche toutefois la réinitialisation de vue de SPEC-06 REQ-SCN-002) ; `vr.mirroring` et `vr.control_method` DOIVENT être stockés et restitués mais n'ont aucun effet. Ces réglages NE DOIVENT PAS apparaître dans l'UI desktop.
- Critère d'acceptation : script `Setting.VrRuntime = 1` puis `print(Setting.VrRuntime)` → 0 ; script `Setting.SceneRatio = 5` puis `print(Setting.SceneRatio)` → 1, taille de la scène inchangée ; les profils qui testent `Setting.VrRuntime > 0` prennent la branche non-VR.

**REQ-PERS-012** (P2) — Réinitialisations automatiques : à chaque **fin de partie** (arrêt du jeu, changement de jeu, sortie), `emu.game_speed` DOIT revenir à 0 et `camera.smooth_time` à 0,4. À chaque **démarrage du programme** et à chaque chargement d'un jeu **différent** du précédent (avant le hook de démarrage), les types des ports d'entrée valent `Manette`/`Manette` (SPEC-08 REQ-INP-060, DEC-09-Q6, D).
- Critère d'acceptation : vitesse réglée à +4 pendant une partie, revenir au menu → le curseur de vitesse affiche 0 ; un script met `Setting.SmoothTime = 0.7`, changer de jeu → 0,4.
- Notes : par conséquent la vitesse persistée est en pratique toujours 0 au lancement suivant.

**REQ-PERS-013** (P2) — Réglages dérivés : les membres de portée D DOIVENT être calculés ainsi :
1. `TargetFramerate` : si `video.vsync` = 1 → fréquence de rafraîchissement de l'écran ; si 2 → la moitié ; sinon → `video.frame_rate` ;
2. `TargetTimePerFrame` = 1 / `TargetFramerate` (secondes) ;
3. `SceneScale` = `RootScale` × `SceneRatio` (desktop : 1 × 1 = 1) ; écrire `SceneScale` modifie `SceneRatio` (sans effet en desktop, REQ-PERS-011) ;
4. `SkyboxEnable` = (`scene.skybox` > 0) ;
5. `ZapperEnable` = (au moins un port de type `Zapper`) ;
6. `ResScale` = facteur d'échelle de l'indice `video.res_scale` : 1, 2/3, 1/2, 1/3, 1/4 pour les indices 0 à 4 ;
7. `TVData` = informations de cadence de la région `emu.tv_mode` (SPEC-02) ;
8. `Layer[i].Coef` = 1 pour l'alignement avant (FRONT), 0 pour centré (CENTER), −1 pour arrière (BACK).
- Critère d'acceptation : `video.vsync` = 0 et `video.frame_rate` = 120 → `Setting.TargetFramerate` = 120 et `TargetTimePerFrame` ≈ 0,008333 ; `Layer[1].Coef` (défaut CENTER) = 0.

**REQ-PERS-014** (P3) — Les constantes Lua de la table §4.3 (identifiants de notification sous forme de chaînes, `DefaultSmoothTime`, `VRScale`, `NonVRScale`) DEVRAIENT être exposées en lecture seule sur `Setting` avec les valeurs indiquées.
- Critère d'acceptation : `print(Setting.FovId)` → `FovId` ; `print(Setting.DefaultSmoothTime)` → 0.4.
- Notes : aucun profil embarqué ne les utilise (inventaire Lua) ; elles sont gardées pour les profils utilisateur.

**REQ-PERS-015** (P3) — `Setting:Save()` appelé depuis Lua DEVRAIT déclencher l'écriture immédiate du fichier de réglages ; `Setting:Load()` DEVRAIT relire ce fichier et appliquer **seulement** les réglages U qu'il contient. `Setting:SetRuntime(n)` DOIT être sans effet (desktop). `Setting:GetInputType(port)` et `Setting:SetInputType(t1, t2)` sont définis par SPEC-08 (REQ-INP-060) et SPEC-05.
- Critère d'acceptation : modifier le volume par l'UI, appeler `Setting:Save()` depuis la console Lua, tuer le processus, relancer → nouveau volume conservé.

**REQ-PERS-016** (P2) — Réglages de l'éditeur et du film (`editor.selection_mode`, `editor.saved_patterns`, `editor.saved_anim_patterns`, `movie.mode`) : ils DOIVENT être acceptés à la lecture d'un profil et restitués à Lua, et n'ont aucun effet dans la réimplémentation. `movie.mode` DOIT rester à 0 (NONE) : une écriture d'une autre valeur PEUT être ignorée.
- Critère d'acceptation : un profil contenant des listes d'enregistrements de patterns se charge sans erreur ; `print(Setting.MovieMode)` → 0.

**REQ-PERS-017** (P2) — Réglages sans effet observable dans l'original : `scene.map_enable`, `video.anti_alias` et `input.emulated_menu_button` sont persistés et exposés mais n'ont aucun effet observable dans la version 1.1.2 de l'original. La réimplémentation DOIT les stocker et les exposer. Pour `video.anti_alias`, elle PEUT appliquer un anticrénelage plein écran (1 : de type FXAA, 2 : de type SMAA, SPEC-06). Pour `input.emulated_menu_button`, elle NE DOIT PAS appliquer d'effet : le réglage est conservé et exposé seulement pour la cohérence des données (DEC-08-6, DEC-R2-D7).
- Critère d'acceptation : modifier ces trois réglages dans l'UI, relancer → valeurs conservées.
- Notes : réponse à la question ouverte n° 6 de SPEC-08 : on garde le réglage pour la cohérence des données, sans effet (DEC-R2-D7).

### 3.2 Fichier de réglages de la réimplémentation

**REQ-PERS-020** (P1) — Les réglages de portée U, ainsi que `ui.fullscreen`, DOIVENT être persistés dans un fichier de réglages de la racine utilisateur (`settings.json`, §4.5.1), indexé par **clé stable**. Les réglages P et S NE DOIVENT PAS être nécessaires au démarrage : leur présence éventuelle dans le fichier PEUT être ignorée.
- Critère d'acceptation : modifier FOV, volume, skybox, région, quitter, relancer → valeurs conservées ; supprimer du fichier toutes les clés autres que U → démarrage normal.

**REQ-PERS-021** (P1) — Moment d'écriture : le fichier de réglages DOIT être écrit à la sortie normale du programme (SPEC-07 REQ-LOOP-052). Il DEVRAIT aussi être écrit au plus 2 s après toute modification par l'UI, pour résister à un arrêt brutal. Toute écriture DOIT être **atomique** : écriture d'un fichier temporaire dans le même dossier, puis remplacement.
- Critère d'acceptation : modifier le volume, attendre 3 s, tuer le processus → au relancement, le volume est conservé (comportement DEVRAIT) ; tuer le processus pendant une écriture → le fichier reste lisible (ancienne ou nouvelle version complète).
- Notes : l'original n'écrit ses réglages qu'à la sortie.

**REQ-PERS-022** (P1) — Fichier corrompu : si le fichier de réglages est illisible (syntaxe invalide, encodage, type racine incorrect), l'implémentation DOIT démarrer avec les valeurs par défaut, renommer le fichier en `settings.json.corrupt-<AAAAMMJJ-HHMMSS>` et afficher une notification non bloquante (SPEC-10) « Settings file was unreadable and has been reset. ». Si le fichier est lisible mais qu'une valeur isolée est invalide (type ou domaine), seule cette valeur DOIT être remplacée (REQ-PERS-003), sans notification.
- Critère d'acceptation : écrire `{{{` dans le fichier → démarrage avec les défauts, fichier renommé, notification affichée ; écrire `"camera.fov": "abc"` → FOV = 70, autres valeurs conservées.
- Notes : l'original revient silencieusement aux défauts ; la notification et la copie de secours sont des améliorations (classe D).

**REQ-PERS-023** (P2) — Compatibilité ascendante : les clés inconnues présentes dans le fichier (réglages d'une version plus récente, réglages VR) DOIVENT être conservées telles quelles lors de la réécriture.
- Critère d'acceptation : ajouter `"future.x": 3` au fichier, modifier le volume, quitter → `"future.x": 3` est toujours présent.

**REQ-PERS-024** (P2) — Versions : chaque fichier persistant de la réimplémentation (réglages, bibliothèque, index des slots, bindings) DOIT porter un entier `schemaVersion`. Un fichier d'une version antérieure DOIT être migré à la lecture puis réécrit à la version courante ; avant la première réécriture migrée, une copie `<nom>.v<N>.bak` DOIT être conservée. Un fichier d'une version **supérieure** DOIT être lu au mieux (clés connues), sans être réécrit pendant la session, avec une notification « Settings were created by a newer version. ».
- Critère d'acceptation : fichier `schemaVersion: 1` lu par une version 2 → copie `settings.json.v1.bak` créée, fichier réécrit en version 2 ; fichier `schemaVersion: 99` → valeurs connues appliquées, fichier inchangé après la sortie.

**REQ-PERS-025** (P3) — L'implémentation N'EST PAS TENUE de lire les fichiers de l'original (`setting.cfg`, `sav.cfg`, `input.cfg`, `rom.cfg`, savestates `.sav`). Elle PEUT proposer un import unique de `rom.json` (format texte décrit au §4.4), qui contient les chemins de ROM et dates de dernière partie par identifiant de profil.
- Critère d'acceptation : sans objet si l'import n'est pas proposé ; s'il l'est, les chemins de ROM importés sont associés aux bons jeux.

### 3.3 Bibliothèque et données par jeu

**REQ-PERS-030** (P1) — La bibliothèque DOIT contenir une entrée par profil disponible (embarqué ou utilisateur), identifiée par l'**identifiant de profil** (chaîne, SPEC-03). Deux profils de même identifiant NE DOIVENT PAS coexister : un profil embarqué l'emporte sur un profil utilisateur, et entre profils utilisateur le premier trouvé l'emporte (ordre de parcours libre). Le doublon est ignoré et journalisé.
- Critère d'acceptation : placer dans le dossier des profils utilisateur un `.3dn` dont l'identifiant est celui d'un profil embarqué → la bibliothèque n'a qu'une entrée pour cet identifiant, celle du profil embarqué.

**REQ-PERS-031** (P1) — Données mémorisées par jeu (portée J) : `romPath` (chemin absolu du fichier ROM choisi, vide si aucun), `title` (titre affiché), `lastPlayed` (date du dernier lancement, secondes Unix, 0 si jamais), jaquette (fichier image, REQ-PERS-036). **Aucun autre réglage** n'est mémorisé par jeu : ni caméra, ni vitesse, ni région, ni données Lua (celles-ci ne vivent que dans les savestates).
- Critère d'acceptation : lancer un jeu, tourner la caméra, quitter, relancer le programme et le même jeu → la caméra est dans sa position initiale ; la ROM n'est pas redemandée.

**REQ-PERS-032** (P1) — Association ROM : l'association est faite **par l'utilisateur**, entrée par entrée, sans contrôle de somme : au premier lancement d'une entrée sans `romPath`, ou sur l'action « re-sélectionner la ROM » (SPEC-07 REQ-LOOP-020, SPEC-10), le navigateur de fichiers s'ouvre (extensions `nes`, `zip`, `7z`) et le chemin choisi DOIT être enregistré dans l'entrée et persisté immédiatement.
- Critère d'acceptation : choisir une ROM pour le jeu X, tuer le processus, relancer, sélectionner X → le jeu démarre sans navigateur.
- Notes : si le fichier a disparu, le lancement échoue avec la notification d'erreur de chargement de ROM (SPEC-07) et `romPath` est conservé.

**REQ-PERS-033** (P1) — `lastPlayed` DOIT être mis à l'heure courante (secondes Unix) à chaque lancement du jeu depuis la bibliothèque. Le tri « Recent » de la bibliothèque (SPEC-10) DOIT ordonner par `lastPlayed` décroissant puis par titre ; le tri « A-Z » par titre croissant.
- Critère d'acceptation : lancer B puis A → en tri « Recent », A précède B.

**REQ-PERS-034** (P2) — Titre : à la création d'une entrée, ou si son titre mémorisé est vide, `title` DOIT prendre le titre du profil (pour un profil utilisateur sans titre : le nom de son fichier sans extension). Le titre est modifiable par l'utilisateur dans le fichier de bibliothèque et DOIT alors être respecté. L'identifiant NE DOIT PAS être modifiable : une entrée dont l'identifiant ne correspond à aucun profil est supprimée au démarrage (REQ-PERS-035).
- Critère d'acceptation : modifier `title` d'une entrée dans `library.json` → la bibliothèque affiche le nouveau titre ; vider ce titre → titre du profil.
- Notes : le fichier texte de l'original porte l'avertissement « Don't touch id parameter, you will regret it ». La réimplémentation PEUT reprendre un avertissement équivalent.

**REQ-PERS-035** (P1) — Réconciliation au démarrage : les entrées de bibliothèque dont l'identifiant ne correspond à aucun profil disponible DOIVENT être supprimées ; une entrée vierge DOIT être créée pour chaque nouveau profil.
- Critère d'acceptation : supprimer le `.3dn` d'un profil utilisateur hors du programme, relancer → l'entrée a disparu de la bibliothèque et du fichier après la sortie.

**REQ-PERS-036** (P2) — Jaquette par jeu : une image optionnelle par entrée, stockée en PNG dans le dossier des jaquettes, **réduite pour tenir dans 256 × 192 px en conservant les proportions** (facteur = max(largeur/256, hauteur/192)), sans canal alpha. Sources : import d'un fichier `jpg`/`png` par l'utilisateur ; capture de l'image de pause du jeu courant (action « Create game art », SPEC-10) ; pour « 2D Mode », mise à jour automatique avec la capture de pause chaque fois que le menu s'affiche. Le nom de fichier DOIT être dérivé de manière déterministe du titre et de l'identifiant, en remplaçant tout caractère interdit dans un nom de fichier par `_` (forme de l'original : `<Titre>_3dsen_<Id>.png`).
- Critère d'acceptation : importer une image 1024 × 1024 → fichier PNG 192 × 192 dans le dossier des jaquettes, affiché sur la tuile ; supprimer le jeu (profil utilisateur) → fichier supprimé.

**REQ-PERS-037** (P2) — Écriture de la bibliothèque : elle DOIT être persistée après une sélection de ROM, un import ou une suppression de profil et à la sortie, et DEVRAIT l'être aussi après la mise à jour de `lastPlayed`. Fichier illisible → bibliothèque reconstruite à partir des profils, entrées vierges, fichier renommé `*.corrupt-<horodatage>` et notification (comme REQ-PERS-022).
- Critère d'acceptation : corrompre `library.json` → démarrage normal, tous les jeux présents sans ROM associée, notification affichée.

### 3.4 Profils utilisateur

**REQ-PERS-040** (P1) — Le dossier des profils utilisateur (`profiles/` de la racine utilisateur) DOIT être créé s'il manque et parcouru au démarrage à la recherche des fichiers `*.3dn` (SPEC-03). Un fichier illisible ou de format obsolète DOIT être ignoré et journalisé, sans empêcher le démarrage.
- Critère d'acceptation : placer un fichier `.3dn` tronqué et un valide → seul le valide apparaît dans la bibliothèque.

**REQ-PERS-041** (P1) — Import d'un profil (« Import custom profile », SPEC-10) :
1. fichier illisible ou sans identifiant → notification (4 s) avec le texte exact défini par SPEC-10 REQ-UI-084, seule source normative (le chemin y est suivi directement de `has an obsolate…`, sans espace) ;
2. identifiant inconnu → copie du fichier dans le dossier des profils utilisateur (même nom de fichier, écrasement si un fichier de ce nom existe), nouvelle entrée de bibliothèque ;
3. identifiant d'un profil utilisateur existant → l'ancien fichier est supprimé et remplacé par la copie ; l'entrée de bibliothèque (ROM, titre, date, jaquette) est conservée ;
4. identifiant d'un profil embarqué → aucun changement.
Puis la bibliothèque est persistée.
- Critère d'acceptation : importer deux fois de suite une version modifiée d'un même profil utilisateur → une seule entrée, `romPath` conservé, contenu du profil = dernière version.

**REQ-PERS-042** (P1) — Suppression d'un jeu (profils utilisateur seulement ; un profil embarqué NE DOIT PAS être supprimable) : après confirmation « Do you want to delete the game and its save states ? » (SPEC-10), l'implémentation DOIT arrêter l'émulation si ce jeu est le jeu courant, effacer tous les slots associés à ce jeu (y compris le slot rapide), supprimer le fichier de profil, la jaquette et l'entrée de bibliothèque.
- Critère d'acceptation : jeu utilisateur avec slots 3 et 7 et le slot rapide → après suppression, ces slots sont vides et leurs fichiers absents, le `.3dn` est supprimé du dossier.

### 3.5 Savestates et slots

**REQ-PERS-050** (P1) — L'implémentation DOIT gérer **36 slots numérotés** `1` à `36` et **un slot rapide** `quick`, **communs à tous les jeux** (un slot n'appartient pas à un jeu). Chaque slot porte : son identifiant, l'identifiant du jeu associé (vide = slot vide), une vignette, la date de sauvegarde (secondes Unix).
- Critère d'acceptation : sauvegarder le jeu A dans le slot 1 puis le jeu B dans le slot 2 → l'onglet Load liste les deux slots avec leurs titres de jeu respectifs.

**REQ-PERS-051** (P1) — Sauvegarde dans un slot numéroté (SPEC-07 REQ-LOOP-042) : la vignette DOIT être la capture prise à la pause, au format 256 × 192 px RGB ; un slot occupé est écrasé sans confirmation ; les métadonnées des slots DOIVENT être persistées immédiatement, puis le savestate écrit.
- Critère d'acceptation : sauvegarder, tuer le processus aussitôt après la fin de l'écriture, relancer → le slot affiche jeu, date et vignette, et se charge.

**REQ-PERS-052** (P1) — Slot rapide (SPEC-07 REQ-LOOP-043, REQ-LOOP-045) : la quick save enregistre le jeu courant dans le slot `quick` avec une vignette noire (pas de capture) ; la quick load charge ce slot **y compris s'il appartient à un autre jeu que le jeu courant** (le jeu du slot est alors lancé). Le slot rapide n'apparaît dans aucune des listes Load/Save.
- Critère d'acceptation : quick save dans A, lancer B, quick load → A est relancé à l'état sauvegardé.

**REQ-PERS-053** (P1) — Chargement d'un slot (SPEC-07 REQ-LOOP-044) : le jeu associé est retrouvé par son identifiant ; s'il n'existe plus, échec avec `The game profile doesn't exist anymore.`. Le chargement NE DOIT PAS demander la ROM : l'état du cœur est autonome (SPEC-02).
- Critère d'acceptation : effacer le `romPath` du jeu A dans la bibliothèque, charger un slot de A → la partie reprend sans navigateur de fichiers.

**REQ-PERS-054** (P1) — Suppression d'un slot (SPEC-10) : après confirmation « Do you want to delete the slot <id> ? », le fichier de savestate DOIT être supprimé et le slot redevenir vide ; les métadonnées sont persistées.
- Critère d'acceptation : supprimer le slot 4 → il disparaît de l'onglet Load, reste présent (vide) dans l'onglet Save, son fichier est absent.

**REQ-PERS-055** (P1) — Cohérence au démarrage : un slot associé à un identifiant de jeu absent de la bibliothèque DOIT être vidé et son fichier supprimé. Un slot dont le fichier de savestate est manquant DOIT être vidé. Si l'index des slots est illisible, l'implémentation DEVRAIT reconstruire les métadonnées à partir des en-têtes des fichiers de savestate présents (REQ-PERS-056) ; à défaut, tous les slots sont vides (comportement de l'original) et une notification est affichée.
- Critère d'acceptation : supprimer à la main le fichier du slot 2 → au démarrage, le slot 2 est vide ; corrompre l'index → les slots dont le fichier existe réapparaissent avec jeu et date (comportement DEVRAIT).

**REQ-PERS-056** (P1) — Contenu d'un fichier de savestate (format libre, binaire recommandé) :
1. une **signature** fixe et un **numéro de version de format** ;
2. un en-tête de métadonnées : identifiant du cœur et sa version de format d'état, identifiant du jeu, empreinte SHA-1 de la ROM (contenu PRG + CHR), date, identifiant du slot ;
3. l'état complet du cœur (SPEC-02), y compris la RAM de cartouche de 8 Kio ;
4. les données persistantes des scripts Lua (tableau de 64 nombres, SPEC-05) ;
5. l'état de la caméra (SPEC-06/SPEC-07 REQ-LOOP-040) ;
6. l'état appris de la conversion : registre d'images, patterns appris et patterns temporaires, état du tracker (SPEC-04, DEC-R2-T5, **D**).
La vignette PEUT être stockée dans le fichier ou à côté. Il NE DOIT PAS contenir les frames converties ni les maillages. Pour un jeu et un cœur donnés, la taille des parties 3 à 6 DOIT être bornée par une valeur annoncée avant la sérialisation et valable pour toute la session (exigence libretro, REQ-ARCH-013).
- Critère d'acceptation : un fichier dont la signature ou la version est inconnue est refusé avec `Corrupted save state.` (SPEC-07 REQ-LOOP-046) ; deux sauvegardes successives sans frame émulée entre elles produisent des parties 3 à 6 identiques octet pour octet ; recharger un savestate puis rejouer les mêmes entrées reproduit les mêmes frames converties (REQ-ARCH-013).
- Notes : l'empreinte ROM n'est pas exigée par l'original (il n'en a pas) ; elle permet de détecter un savestate « 2D Mode » chargé avec une autre ROM (Q-4).

**REQ-PERS-057** (P2) — Listes : l'onglet Load DOIT lister seulement les slots occupés ; l'onglet Save DOIT lister le premier slot vide (plus petit numéro libre), s'il en existe, toujours en première position, suivi de tous les slots occupés (SPEC-10 REQ-UI-051, qui fait foi). L'ordre des slots occupés suit `ui.sort_option` : « A-Z » → par titre du jeu puis date décroissante ; « Recent » → par date décroissante puis titre (présentation : SPEC-10).
- Critère d'acceptation : trois slots de deux jeux, tri « Recent » → le plus récent est en tête.

**REQ-PERS-058** (P2) — L'historique de rollback (SPEC-07) N'EST PAS persisté : il vit en mémoire et il est vidé à chaque démarrage, chargement et reset d'un jeu et à chaque changement de cœur.
- Critère d'acceptation : jouer 1 min, quitter, relancer le même jeu, demander un rollback → aucun effet.

**REQ-PERS-059** (P3) — Ouvrir directement un fichier de savestate depuis le navigateur de fichiers PEUT être proposé (l'original l'accepte dans un mode secondaire). Si c'est le cas, le jeu est déterminé par l'en-tête (REQ-PERS-056).
- Critère d'acceptation : sans objet si la fonction n'est pas proposée.

### 3.6 RAM sauvegardée par pile

**REQ-PERS-060** (P2) — Fait observable : l'original **ne persiste pas** la RAM de cartouche sauvegardée par pile. Les sauvegardes internes des jeux (Zelda, Final Fantasy…) sont perdues à la fermeture, sauf via un savestate. La réimplémentation DEVRAIT persister cette RAM pour les ROM dont l'en-tête indique une pile : un fichier par ROM, nommé d'après l'empreinte SHA-1 de la ROM (`battery/<sha1>.srm`, contenu brut de 8 Kio, ou la taille déclarée par la cartouche), relu au chargement de la ROM et écrit à l'arrêt du jeu, à la sortie et au plus toutes les 30 s si son contenu a changé.
- Critère d'acceptation : ROM avec pile, sauvegarder dans le jeu, revenir au menu, quitter, relancer le jeu → la sauvegarde interne est présente.
- Notes : écart volontaire (classe D), nécessaire en libretro (mémoire `SAVE_RAM`). La clé par empreinte de ROM, plutôt que par identifiant de jeu, permet d'utiliser « 2D Mode » avec plusieurs ROM. Le chargement d'un savestate remplace cette RAM par celle du savestate, comme sur la console.

### 3.7 Bindings et petits états d'interface

**REQ-PERS-065** (P1) — Les données de bindings de SPEC-08 REQ-INP-050 (jeux clavier J1/J2, jeux par identité de manette, dernière manette de J1/J2) DOIVENT être persistées dans `input.json` (racine utilisateur) avec un `schemaVersion`, à la sortie et après chaque modification (SPEC-08 REQ-INP-051). Fichier absent ou illisible → bindings par défaut (SPEC-08 REQ-INP-052), fichier renommé `*.corrupt-<horodatage>`.
- Critère d'acceptation : remapper J1 « A » sur `K`, relancer → `K` actif ; corrompre le fichier → bindings par défaut, démarrage normal.

**REQ-PERS-066** (P2) — Le dernier dossier ouvert dans le navigateur de fichiers DOIT être mémorisé de façon persistante et proposé à l'ouverture suivante (clé `ui.last_browse_dir`), comme l'exige SPEC-10 REQ-UI-105 ; l'original le conserve d'une exécution à l'autre.
- Critère d'acceptation : choisir une ROM dans `D:\roms`, relancer, ouvrir le navigateur → il s'ouvre sur `D:\roms`.

**REQ-PERS-067** (P2) — L'état plein écran (`ui.fullscreen`) DOIT être persisté et restauré au lancement. La taille et la position de la fenêtre en mode fenêtré DEVRAIENT l'être aussi (`ui.window_rect`).
- Critère d'acceptation : passer en plein écran, quitter, relancer → plein écran.

### 3.8 Emplacements de fichiers

**REQ-PERS-070** (P1) — Racine utilisateur : par défaut, sous Windows, `%LOCALAPPDATA%\<NomProduit>\` (le nom produit est fixé par SPEC-01). Toutes les données modifiables y sont rangées ; le dossier d'installation NE DOIT PAS être modifié en mode normal.
- Critère d'acceptation : installation dans un dossier en lecture seule → le programme fonctionne, réglages et sauvegardes sont créés sous `%LOCALAPPDATA%`.

**REQ-PERS-071** (P1) — Mode portable : si un fichier nommé `portable.txt` (contenu indifférent) est présent à côté de l'exécutable, la racine utilisateur DOIT être le dossier `userdata\` situé à côté de l'exécutable (créé au besoin). Une option de ligne de commande `--userdata <dossier>` DEVRAIT permettre d'imposer une autre racine ; elle a priorité sur les deux règles précédentes.
- Critère d'acceptation : créer `portable.txt` → après lancement, `userdata\settings.json` existe à côté de l'exécutable et rien n'est écrit sous `%LOCALAPPDATA%`.

**REQ-PERS-072** (P1) — Disposition de la racine utilisateur (créée au démarrage pour ce qui manque) :

| Chemin | Contenu |
|---|---|
| `settings.json` | réglages U (§4.5.1) |
| `input.json` | bindings (REQ-PERS-065) |
| `library.json` | bibliothèque (§4.5.2) |
| `saves/index.json` | métadonnées des 37 slots (§4.5.3) |
| `saves/<id>.state` | savestate du slot `<id>` (`1`…`36`, `quick`) |
| `saves/<id>.png` | vignette du slot (si stockée à part) |
| `battery/<sha1>.srm` | RAM sauvegardée par pile (REQ-PERS-060) |
| `profiles/*.3dn` | profils utilisateur importés |
| `covers/*.png` | jaquettes |
| `logs/` | journaux d'exécution (rotation libre) |

- Critère d'acceptation : premier lancement sur racine vide → les dossiers `saves`, `battery`, `profiles`, `covers`, `logs` existent ; les fichiers JSON sont créés au plus tard à la sortie.
- Notes : les noms sont normatifs pour la réimplémentation desktop ; le frontend libretro les remplace par les dossiers fournis par l'hôte (REQ-PERS-086).

**REQ-PERS-073** (P1) — Si la racine utilisateur ne peut pas être créée ou écrite, le programme DOIT démarrer quand même avec les réglages par défaut gardés en mémoire, afficher une notification d'erreur « User data folder is not writable: <chemin> » et désactiver les fonctions d'écriture (sauvegarde de slot, import), qui affichent alors la même notification si on les utilise.
- Critère d'acceptation : racine pointée sur un dossier en lecture seule (`--userdata`) → démarrage, notification, jeu jouable, sauvegarde de slot refusée avec notification.

**REQ-PERS-074** (P2) — Noms de fichiers dérivés de données utilisateur (jaquettes, éventuels exports) : tout caractère interdit par le système de fichiers DOIT être remplacé par `_`, et le nom DOIT inclure l'identifiant de profil pour être unique.
- Critère d'acceptation : jeu titré `A/B:C?` d'identifiant `42` → jaquette `A_B_C__3dsen_42.png` (ou nom équivalent unique, sans caractère interdit).

**REQ-PERS-075** (P2) — Premier lancement : bibliothèque = tous les profils embarqués et utilisateur, sans ROM associée, `lastPlayed` = 0, aucun slot occupé, réglages par défaut, bindings par défaut ; aucune boîte de dialogue n'est requise.
- Critère d'acceptation : racine vide → menu principal affiché avec la bibliothèque complète, sans erreur ni notification.

### 3.9 Exposition en core options libretro

**REQ-PERS-080** (P1) — Clés stables : chaque réglage DOIT avoir une clé de la forme `[a-z0-9_]+(\.[a-z0-9_\[\]]+)*` (colonne « Clé »). Une clé publiée NE DOIT JAMAIS être renommée ni réutilisée pour un autre sens ; un réglage retiré garde sa clé réservée (lecture tolérée, ignorée).
- Critère d'acceptation : test de non-régression qui compare l'ensemble des clés du modèle à une liste de référence versionnée : aucune clé disparue sans mention « retirée ».

**REQ-PERS-081** (P1) — Domaine énumérable : chaque réglage U DOIT pouvoir fournir la **liste finie ordonnée** de ses valeurs admises, chacune avec un libellé. Pour les réglages continus, la discrétisation de la colonne « libretro » des tableaux §4.1 DOIT être utilisée (ex. `audio.volume` : 0 à 100 % par pas de 5 %).
- Critère d'acceptation : pour chaque réglage U exposé à libretro, l'énumération renvoie une liste non vide, sans doublon, qui contient la valeur par défaut.

**REQ-PERS-082** (P1) — Indépendance : le modèle de réglages DOIT être une bibliothèque sans dépendance à l'UI, à la fenêtre ni au système de fichiers. La lecture et l'écriture passent par une interface de stockage abstraite (lire ou écrire un blob nommé), fournie par l'hôte. Le modèle NE DOIT PAS être un état global : chaque instance d'émulation possède le sien (PLAN.md).
- Critère d'acceptation : test unitaire qui instancie deux modèles en mémoire, modifie l'un, et vérifie que l'autre est intact, sans toucher au disque.

**REQ-PERS-083** (P2) — Correspondance libretro : la clé de core option DOIT être `3dsen_` suivi de la clé stable, `.` `[` `]` remplacés par `_` (ex. `camera.fov` → `3dsen_camera_fov`). Les valeurs DOIVENT être les libellés de l'énumération (REQ-PERS-081). Chaque réglage porte une **catégorie** (colonne « Thème ») utilisable comme catégorie d'options v2, et un indicateur « exposé en libretro » (colonne « libretro » ; « — » = non exposé).
- Critère d'acceptation : la génération automatique de la table d'options libretro depuis le modèle produit `3dsen_camera_fov` avec les valeurs `45`…`95` et le défaut `70`.

**REQ-PERS-084** (P2) — Moment d'effet : chaque réglage DOIT exposer une métadonnée « effet » parmi `immédiat`, `prochain chargement de jeu`, `redémarrage` (colonne « Effet »), utilisable par un frontend pour signaler qu'un redémarrage est nécessaire.
- Critère d'acceptation : `camera.fov` porte « immédiat », `emu.auto_tv_mode` porte « prochain chargement de ROM » (noté `ROM` au §4.1), `ui.window_rect` porte « redémarrage ».

**REQ-PERS-085** (P2) — Les réglages P et S NE DOIVENT PAS être exposés comme core options (ils appartiennent au profil ou au script). Une modification venant de l'hôte libretro DOIT suivre le même chemin qu'une modification par l'UI (validation REQ-PERS-003, notification REQ-PERS-009).
- Critère d'acceptation : modifier `3dsen_camera_fov` dans RetroArch → la caméra se réinitialise avec le nouveau champ de vision comme depuis l'UI desktop.

**REQ-PERS-086** (P2) — Frontend libretro : les slots, la bibliothèque et les jaquettes sont gérés par l'hôte et NE DOIVENT PAS être exigés ; la sérialisation (`retro_serialize`) DOIT contenir les parties 3 à 6 de REQ-PERS-056 (dont l'état appris de la conversion, DEC-R2-T5) précédées d'une signature et d'une version ; la RAM par pile DOIT être exposée comme mémoire `SAVE_RAM` ; les profils utilisateur sont cherchés dans le dossier système de l'hôte (sous-dossier défini par SPEC-01).
- Critère d'acceptation : savestate RetroArch puis chargement → reprise identique ; fichier `.srm` écrit par RetroArch pour une ROM à pile.

---

## 4. Formats et interfaces

### 4.1 Catalogue des réglages

Légende des colonnes :
- **Clé** : clé stable de la réimplémentation (REQ-PERS-080).
- **Lua** : chemin d'accès Lua (`Setting.` omis) ; « — » = non exposé. `(lecture)` = non assignable.
- **N°** : numéro de champ dans le message de réglages embarqué dans les profils (SPEC-03) ; « — » = absent du message.
- **Portée** : U, P, S, D, J (§2).
- **Effet** : `imm.` = immédiat (au plus tard à la frame de rendu suivante) ; `jeu` = au prochain chargement de jeu ; `ROM` = au prochain chargement de ROM ; `redém.` = au redémarrage.
- **UI** : présence dans le panneau de réglages desktop de l'original (libellé observé), « — » = absent.
- **libretro** : discrétisation pour les core options (REQ-PERS-081) ; « — » = non exposé.

#### 4.1.1 Vidéo et rendu

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `video.graphic_quality` | `GraphicQuality` | 7 | entier | 0 | 0 Low, 1 Medium, 2 High | P (DEC-09-Q1) | imm. | aucun sur desktop (libellé « Graphic » : Low/Medium/High dans l'original, masqué ; SPEC-10 REQ-UI-062) | Low/Medium/High | conservé dans le modèle, sans effet visible (SPEC-06 REQ-SCN-062) |
| `video.vsync` | `VSync` | 9 | entier | 0 | 0 désactivée, 1 chaque rafraîchissement, 2 un rafraîchissement sur deux | U | imm. | — | — | cadence de rendu, SPEC-07 REQ-LOOP-014 |
| `video.frame_rate` | `FrameRate` | 10 | entier | 90 | {60, 90, 120} ; autre → 90 | U | imm. | « Render Rate » : 60fps/90fps/120fps | — | plafond d'images/s quand `vsync` = 0, SPEC-07 |
| `video.skip_frame` | `SkipFrame` | 30 | booléen | vrai | — | U | imm. | case « Fast Render » | — | omettre le rendu si aucune nouvelle frame émulée, SPEC-07 REQ-LOOP-014 |
| `video.render_method` | `RenderMethod` | 28 | entier | 0 | 0 Direct, 1 Render Tex | U | imm. | « Direct / Render Tex » (masqué sous Windows) | — | 0 : rendu direct ; 1 : rendu hors écran mis à l'échelle `ResScale` puis affiché, SPEC-06 |
| `video.res_scale` | `ResScaleIndex` | 29 | entier | 0 | 0 Native, 1 2/3, 2 1/2, 3 1/3, 4 1/4 | U | imm. (si `render_method` = 1) | aucun sur desktop (« Tex Scale » de l'original non exposé, SPEC-10 REQ-UI-062, DEC-10-Q2) | Native…1/4 | facteur de résolution du rendu hors écran, conservé dans le modèle ; application facultative (SPEC-06 REQ-SCN-063) |
| `video.anti_alias` | `AntiAlias` | 32 | entier | 0 | 0 None, 1 FFAA, 2 SMAA | U | imm. | « Anti Alias » : None/FFAA/SMAA | None/FFAA/SMAA | sans effet dans l'original (REQ-PERS-017), SPEC-06 |
| `render.mesh_algo` | `RenderMode` | 13 | énum | 0 | 0 GREEDY, 1 MARCHING, 2 GREEDY_MULTICOLOR | U | imm. (régénération, SPEC-07 REQ-LOOP-024) | liste déroulante « Render » | les 3 valeurs | algorithme de mesh par défaut ; un pattern peut imposer le sien, SPEC-04 |
| `render.shadow` | `Shadow` | 17 | booléen | vrai | — | U | imm. | case « Shadow » | on/off | interrupteur global des ombres ; ombre effective = `Shadow` ∧ drapeau du pattern ∧ drapeau du calque ; jamais pour les éléments d'UI du jeu, SPEC-06 |
| `ui.fullscreen` | — | — | booléen | faux | — | U | imm. | case « Full Screen » | — | plein écran (REQ-PERS-067) |

#### 4.1.2 Caméra

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `camera.fov` | `Fov` | 38 | réel (degrés) | 70 | UI : 45 à 95 par pas de 5 (curseur −5…+5, valeur = 70 + 5 × position) ; script : non borné dans l'original, la réimplémentation DEVRAIT borner à [10 ; 150] | U | imm. : réinitialise la vue caméra | curseur « FOV » | 45, 50, …, 95 | champ de vision vertical ; la distance par défaut est recalculée pour cadrer l'image, SPEC-06 |
| `camera.local_cam` | `LocalCam`, alias `TopDownCam` | 25 | booléen | faux | — | P | imm. | — | — | mode « vue de dessus » : l'orientation est composée autour de l'axe avant puis de l'axe latéral et la caméra ne vise plus le centre de l'image, SPEC-06 |
| `camera.smooth_time` | `SmoothTime` | — | réel (s) | 0,4 | > 0 | S | imm. | — | — | temps de lissage des mouvements de caméra ; remis à 0,4 en fin de partie (REQ-PERS-012), SPEC-06 |

#### 4.1.3 Lumière

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `light.direction` | `Light.Direction` | 1.1 | vecteur 3 réels | (1, −1, 1) | non nul | P | imm. | — | — | direction de la lumière directionnelle, SPEC-06 |
| `light.color` | `Light.Color` | 1.2 | couleur RVBA (réels 0..1) | blanc (1, 1, 1, 1) | — | P | imm. | — | — | couleur de la lumière, SPEC-06 |
| `light.intensity` | `Light.Intensity` | 1.3 | réel | 1,0 | UI : 0,5 à 1,5 par pas de 0,01 (curseur 0…100, valeur = 0,5 + position/100) | **U** | imm. | curseur « Light » | 0.50 à 1.50 par pas de 0,05 | intensité de la lumière, SPEC-06 ; préférence conservée au chargement de profil (REQ-PERS-005) |

(N° « 1.k » : champ k du sous-message lumière, lui-même champ 1 du message de réglages.)

#### 4.1.4 Fond, skybox, TV

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `scene.skybox` | `Skybox` | 21 | entier | −1 | −1 Default, 0 Color, 1 Gradient, 2 Fantasy | U | imm. | liste « Skybox » (Default/Color/Gradient/Fantasy) | les 4 valeurs | −1 : pas de skybox, fond = couleur de fond NES (ou noir au menu) ; 0 : fond uni `CustomBgColor` ; 1, 2 : skybox n° 1, 2, SPEC-06 |
| `scene.custom_bg_color` | `CustomBgColor` | 3 | couleur RVBA | noir (0, 0, 0, 1) | — | U | imm. | sélecteur de couleur (visible si Skybox = Color) | 8 préréglages + défaut (Q-5) | couleur de fond quand `BgColor` est faux ou que `Skybox` ≠ −1, SPEC-06 |
| `scene.use_nes_bg_color` | `BgColor` | 2 | booléen | vrai | — | P (forcé vrai au chargement, REQ-PERS-005) | imm. | — | — | vrai et `Skybox` = −1 → fond = couleur universelle de la palette NES ; sinon `CustomBgColor`, SPEC-06 |
| `scene.tv_box` | `TVBox` | 35 | entier | 0 | 0 None, 1 TV 1, 2 TV 2, 3 TV 3 | U | imm. | liste « TV Box » | les 4 valeurs | modèle de téléviseur 3D autour de l'image, affiché seulement pendant le jeu, SPEC-06 |
| `scene.map_enable` | `MapEnable` | 5 | booléen | faux | — | P | — | — | — | sans effet observable (REQ-PERS-017) |

#### 4.1.5 Découpe de l'image (Clipping)

Marges en pixels NES retirées de l'image convertie. Indices Lua 0 à 3 ≡ `x`, `y`, `z`, `w`.

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `clip.left` | `Clipping[0]`, `Clipping.x` | 4.1 | réel | 0,1 | [0 ; 256] | P | imm. | — | — | marge gauche |
| `clip.bottom` | `Clipping[1]`, `Clipping.y` | 4.2 | réel | 8,1 | [0 ; 248] | P | imm. | — | — | marge basse (bas de l'écran NES) |
| `clip.right` | `Clipping[2]`, `Clipping.z` | 4.3 | réel | 8,1 | [0 ; 256] | P | imm. | — | — | marge droite |
| `clip.top` | `Clipping[3]`, `Clipping.w` | 4.4 | réel | 0,1 | [0 ; 248] | P | imm. | — | — | marge haute (haut de l'écran NES) |

Convention (DEC-R2-T1, DEC-09-Q2) : indice 0 = gauche, 1 = **bas**, 2 = droite, 3 = **haut**, « haut » désignant le haut de l'écran NES. Effet (défini par SPEC-04 REQ-E3D-002 pour le filtrage des tiles et par SPEC-04B REQ-E3D-410 / SPEC-06 pour la découpe et le cadrage) : avec les marges (c0, c1, c2, c3), une tile émise par le cœur à la position écran (x, y) (y vers le bas, SPEC-02) est conservée si et seulement si x + 8 > c0, x ≤ 256 − c2, y + 8 > c3 (bord haut) et y ≤ 248 − c1 (bord bas) ; le rectangle visible est R = [c0, 256 − c2] × [c1, 240 − c3] en coordonnées scène, soit les lignes NES [8 + c3, 248 − c1]. La cible de la caméra est décalée de ((c0 − c2)/2, (c1 − c3)/2) × échelle de scène. Les valeurs non entières (0,1, 8,1…) sont voulues : elles évitent les égalités aux bords. Le domaine [0 ; 248] des marges verticales correspond à la hauteur de la grille de conversion ; une valeur hors domaine est bornée (REQ-PERS-003).

#### 4.1.6 Calques (Layer)

Six calques indexés 0 à 5 (constantes Lua `TLayer`, SPEC-05) : 0 UNIDENTIFIED, 1 UI, 2 L1, 3 L2, 4 L3, 5 L4.

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | Comportement |
|---|---|---|---|---|---|---|---|---|
| `layer.default_layer` | `Layer.DefLayer` | 6.2 | énum calque | 2 (L1) | 0 à 5 | P | jeu / imm. | calque attribué aux patterns qui n'en précisent pas, SPEC-04 |
| `layer.auto` | `Layer.Auto` | 6.3 | booléen | faux | — | P | imm. | conservé pour la compatibilité des données, sans effet (SPEC-04 REQ-E3D-342) |
| `layer[i].offset` | `Layer[i].Offset` | 6.1.1 | réel (unités de scène = pixels NES) | i = 0 : 0 ; i = 1 : 0 ; i = 2…5 : 0, 12, 24, 36 | libre | P | imm. | décalage en profondeur du calque, SPEC-04/SPEC-06 |
| `layer[i].aligned` | `Layer[i].Aligned` | 6.1.2 | énum | i = 0 : FRONT ; i = 1 : CENTER ; i = 2…5 : FRONT | 0 FRONT, 1 CENTER, 2 BACK | P | imm. | alignement en profondeur des volumes du calque, SPEC-04 |
| `layer[i].coef` | `Layer[i].Coef` (lecture) | — | réel | dérivé | 1, 0, −1 | D | — | REQ-PERS-013 point 8 |
| `layer[i].cast_shadow` | `Layer[i].CastShadow` | 6.1.3 | booléen | vrai pour i = 2 seulement (l'état de premier lancement de l'original n'est pas reproduit, DEC-04B-Q5, D) | — | P | imm. | le calque projette des ombres, SPEC-06 ; réinitialisé au chargement de profil (REQ-PERS-005) |
| `layer[i].recv_shadow` | `Layer[i].RecvShadow` | 6.1.4 | booléen | vrai pour i = 2…5, faux pour 0 et 1 (valeur après tout chargement de profil ; l'état de premier lancement de l'original n'est pas reproduit, DEC-04B-Q5, D) | — | P | imm. | le calque reçoit des ombres, SPEC-06 ; réinitialisé au chargement de profil |

(N° « 6.1.k » : champ k de chaque élément de la liste répétée de 6 calques, champ 1 du sous-message calques, lui-même champ 6 du message de réglages ; à la lecture, la liste remplace entièrement la liste courante.)

Les calques ne sont pas affichés dans l'UI desktop et ne sont pas exposés en libretro.

#### 4.1.7 Émulation

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `emu.game_speed` | `GameSpeed` | 11 | réel (valeurs entières en UI) | 0 | [−8 ; 8] | U (remis à 0 en fin de partie, REQ-PERS-012) | imm. | curseur « Speed » −8…+8 ; hotkeys ±1 | −8 … 8 | facteur k = 1 + s/4 (s ≥ 0), 1/(1 − s/4) (s < 0), SPEC-07 REQ-LOOP-015 |
| `emu.tv_mode` | `TVMode` | 34 | énum | 0 (NTSC) | 0 NTSC, 1 PAL, 2 MULTI (non proposé par l'UI), 3 DENDY | **P** (Q-1) | imm. (cœur et cadence) | « TV Region » : NTSC/PAL/DENDY (positions 0, 1, 2 → valeurs 0, 1, 3) | NTSC/PAL/DENDY | région du cœur, SPEC-02/SPEC-07 REQ-LOOP-017 |
| `emu.auto_tv_mode` | `AutoTVMode` | 33 | booléen | faux | — | U | ROM | — | on/off | au chargement d'une ROM, `TVMode` prend la région de l'en-tête, SPEC-07 REQ-LOOP-017 ; voir Q-1 pour l'ordre avec le profil |
| `emu.core` | `Core` | 22 | entier | 0 | indice de cœur ; modulo le nombre de cœurs | U | imm. (réinitialise l'historique de rollback, recharge) | — | — | choix du cœur NES (un seul en desktop), SPEC-02 |

#### 4.1.8 Paramètres de conversion 2D→3D

Tous de portée P, non affichés dans l'UI desktop, non exposés en libretro ; comportement défini par SPEC-04.

| Clé | Lua | N° | Type | Défaut | Domaine | Effet | Comportement |
|---|---|---|---|---|---|---|---|
| `conv.same_bg_palette` | `SameBgPalette`, alias `SamePalette` | 18 | booléen | faux | — | imm. | reconnaissance des tiles de fond sans tenir compte de la palette |
| `conv.same_sp_palette` | `SameSpPalette` | 37 | booléen | faux | — | imm. | idem pour les sprites |
| `conv.sprite_tolerance` | `SpriteTolerance` | 19 | entier | 0 | ≥ 0 | imm. | tolérance de regroupement des sprites |
| `conv.scan_direction` | `ScanDirection` | 20 | entier | 0 | 0 par lignes (tri en y), 1 par colonnes (tri en x) | imm. | ordre de parcours des tiles de fond pour la détection des patterns |
| `conv.empty_tile` | `EmptyTile` | 24 | booléen | faux | — | imm. | inclure dans la conversion les tiles de fond vides (motif entièrement transparent) |

#### 4.1.9 Audio, entrées, interface

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `audio.volume` | `Volume` | 12 | réel | 1,0 | [0 ; 1] ; UI 0…100 % par pas de 1 | U | imm. (< 100 ms) | curseur volume | 0 à 100 % par pas de 5 | volume général (émulation, musique, sons d'interface), SPEC-11 |
| `audio.bg_music` | `BGMusic` | 31 | booléen | vrai | — | U | imm. | case « BG Music » | — | musique du menu, SPEC-11 |
| `input.port_type[0]`, `input.port_type[1]` | `GetInputType(p)` / `SetInputType(t1, t2)` | — | énum | 0, 0 | 0 Manette, 1 Zapper | S | imm. | — | — | type des ports, SPEC-08 REQ-INP-060 |
| `input.zapper_enable` | `ZapperEnable` (lecture) | — | booléen | dérivé | — | D | — | — | — | REQ-PERS-013 |
| `input.emulated_menu_button` | `EmulatedMenuButton` | 27 | booléen | faux | — | U | imm. | case « Menu = Select + Start » | — | REQ-PERS-017, SPEC-08 |
| `ui.sort_option` | `SortOption` | 36 | entier | 0 | 0 A-Z, 1 Recent | U | imm. | liste « Sort by » de la bibliothèque | — | tri de la bibliothèque et des slots, SPEC-10, REQ-PERS-033/057 |
| `ui.last_browse_dir` | — | — | chaîne | dossier personnel | chemin | U | imm. | — | — | REQ-PERS-066 |
| `ui.window_rect` | — | — | 4 entiers | taille par défaut SPEC-10 | — | U | redém. | — | — | REQ-PERS-067 |

#### 4.1.10 VR, éditeur, film, valeurs internes et dérivées

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Comportement dans la réimplémentation |
|---|---|---|---|---|---|---|---|
| `vr.runtime` | `VrRuntime` | 8 | entier | 0 | lecture toujours 0 | U (figé) | REQ-PERS-011 |
| `vr.scene_ratio` | `SceneRatio` | 26 | réel | 1 | lecture 1 en desktop | U | REQ-PERS-011 |
| `vr.mirroring` | `Mirroring` | 23 | booléen | vrai | — | U | stocké, sans effet |
| `vr.control_method` | `ControlMethod` | 16 | entier | 0 | — | U | stocké, sans effet |
| `editor.selection_mode` | `SelectionMode` | — | énum | 0 (Shape3D) | 0 Shape3D, 1 Tile | S | sans effet (REQ-PERS-016) |
| `editor.saved_patterns` | `SavedPattern3D` | 14 | liste d'enregistrements (SPEC-03) | 5 entrées « Config1 »…« Config5 » | — | P | conservé, sans effet |
| `editor.saved_anim_patterns` | `SaveAnimPattern3D` | 15 | liste d'enregistrements (SPEC-03) | 5 entrées « Deform1 »…« Deform5 » | — | P | conservé, sans effet |
| `movie.mode` | `MovieMode` | — | énum | 0 | 0 NONE, 1 PLAYBACK, 2 RECORDING | S | figé à 0 (REQ-PERS-016) |
| `scene.root_scale` | `RootScale` | — | réel | 1 | 1 en desktop | S | facteur d'échelle de la scène (VR : 0,005) |
| `scene.scene_scale` | `SceneScale` | — | réel | dérivé | — | D | REQ-PERS-013 point 3 |
| `scene.skybox_enable` | `SkyboxEnable` (lecture) | — | booléen | dérivé | — | D | REQ-PERS-013 point 4 |
| `video.res_scale_factor` | `ResScale` (lecture) | — | réel | dérivé | — | D | REQ-PERS-013 point 6 |
| `video.target_framerate` | `TargetFramerate` (lecture) | — | réel | dérivé | — | D | REQ-PERS-013 point 1 |
| `video.target_time_per_frame` | `TargetTimePerFrame` (lecture) | — | réel | dérivé | — | D | REQ-PERS-013 point 2 |
| `emu.tv_data` | `TVData` (lecture) | — | objet | dérivé | — | D | REQ-PERS-013 point 7, SPEC-02 |
| `app.engine_mode` | `EngineMode`, `IsProfileMode`, `IsRomMode` (membres statiques, lecture ; non utilisés par les profils embarqués) | — | énum | 1 (PROFILE) | 0 ROM, 1 PROFILE | S | toujours PROFILE (le mode ROM est celui de l'éditeur) ; `IsProfileMode` vrai, `IsRomMode` faux |

### 4.2 Membres Lua de `Setting` et des objets associés

Contrat d'interopérabilité (SPEC-05 fixe la mécanique d'accès et les erreurs). L = lecture, É = écriture.

| Objet | Membre | Accès | Renvoie |
|---|---|---|---|
| `Setting` | `SelectionMode`, `ControlMethod`, `RootScale`, `SceneScale`, `SmoothTime`, `BgColor`, `CustomBgColor`, `MapEnable`, `LocalCam`, `TopDownCam`, `GraphicQuality`, `VrRuntime`, `VSync`, `FrameRate`, `Fov`, `GameSpeed`, `Volume`, `RenderMode`, `SavedPattern3D`, `SaveAnimPattern3D`, `Shadow`, `SameBgPalette`, `SamePalette`, `SameSpPalette`, `SpriteTolerance`, `ScanDirection`, `Skybox`, `Core`, `Mirroring`, `EmptyTile`, `MovieMode`, `SceneRatio`, `EmulatedMenuButton`, `RenderMethod`, `ResScaleIndex`, `SkipFrame`, `BGMusic`, `AntiAlias`, `AutoTVMode`, `TVMode`, `TVBox`, `SortOption` | L, É | valeur du réglage (§4.1) |
| `Setting` | `TargetFramerate`, `TargetTimePerFrame`, `SkyboxEnable`, `ZapperEnable`, `ResScale`, `TVData` | L | valeur dérivée |
| `Setting` | `Light`, `Layer`, `Clipping` | L | objet lumière, objet calques, objet découpe |
| `Setting` | `SetRuntime(n)` | méthode | aucun effet (REQ-PERS-015) |
| `Setting` | `GetInputType(port)` | méthode | type du port 0 ou 1 (SPEC-08) |
| `Setting` | `SetInputType(t1, t2)` | méthode | fixe les types des ports 0 et 1 (SPEC-08) |
| `Setting` | `Save()`, `Load()` | méthode | REQ-PERS-015 |
| `Setting` | `PropertyChanged` | événement | PEUT être non supporté depuis Lua (aucun profil ne l'utilise) |
| `Light` | `Direction` (vecteur 3), `Color` (couleur), `Intensity` (réel) | L, É | §4.1.3 |
| `Layer` | `DefLayer`, `Auto` | L, É | §4.1.6 |
| `Layer` | `[i]` (i entier 0–5 ou constante `TLayer`) | L | objet calque |
| objet calque | `Offset`, `Aligned`, `CastShadow`, `RecvShadow` | L, É | §4.1.6 |
| objet calque | `Coef` | L | §4.1.6 |
| `Clipping` | `x`, `y`, `z`, `w`, `[0]`…`[3]` | L, É | §4.1.5 |
| `Clipping` | `target` | L, É | les quatre marges sous forme de vecteur 4 |

### 4.3 Constantes Lua de `Setting` (REQ-PERS-014)

| Constante | Valeur | | Constante | Valeur |
|---|---|---|---|---|
| `SelectionModeId` | "SelectionModeId" | | `SkyboxId` | "SkyboxId" |
| `ControlMethodId` | "ControlMethodId" | | `CoreId` | "CoreId" |
| `SceneScaleId` | "SceneScaleId" | | `MirroringId` | "MirroringId" |
| `LightId` | "LightId" | | `EmptyTileId` | "EmptyTileId" |
| `LightDirectionId` | "Light.DirectionId" | | `InputTypeId` | "InputTypeId" |
| `LightColorId` | "Light.ColorId" | | `MovieModeId` | "MovieModeId" |
| `LightIntensityId` | "Light.IntensityId" | | `SceneRatioId` | "SceneRatioId" |
| `BgColorId` | "BgColorId" | | `EmuMenuId` | "EmuMenuId" |
| `CustomBgColorId` | "CustomBgColorId" | | `RenderMethodId` | "RenderMethodId" |
| `ClippingId`, `ClippingElementId` | "Clipping" | | `ResScaleId` | "ResScaleId" |
| `MapEnableId` | "MapEnable" | | `SkipFrameId` | "SkipFrameId" |
| `LayerId` | "Layer" | | `BgMusicId` | "BgMusicId" |
| `LayerAlignedId` | "Layer.Aligned" | | `AntiAliasId` | "AntiAliasId" |
| `LayerOffsetId` | "Layer.Offset" | | `AutoTVModeId` | "AutoTVModeId" |
| `LayerDefLayerId` | "Layer.DefLayer" | | `TVModeId` | "TVModeId" |
| `LayerAutoId` | "Layer.Auto" | | `TVBoxId` | "TVBoxId" |
| `LocalCamId` | "LocalCamId" | | `SortOptionId` | "SortOptionId" |
| `GraphicQualityId` | "GraphicQuality" | | `ShadowId` | "ShadowId" |
| `VrRuntimeId` | "VrRuntimeId" | | `SameBgPaletteId` | "SameBgPaletteId" |
| `PreVrRuntimeId` | "PreVrRuntimeId" | | `SameSpPaletteId` | "SameSpPaletteId" |
| `VSyncId` | "VSyncId" | | `SpriteToleranceId` | "SpriteToleranceId" |
| `FrameRateId` | "FrameRateId" | | `ScanDirectionId` | "ScanDirectionId" |
| `FovId` | "FovId" | | `SavedPattern3DId` | "SavePatternId" |
| `GameSpeedId` | "GameSpeedId" | | `SavedAnimPattern3DId` | "SaveAnimPatternId" |
| `VolumeId` | "VolumeId" | | `DefaultSmoothTime` | 0.4 |
| `RenderModeId` | "RenderModeId" | | `VRScale` | 0.005 |
| | | | `NonVRScale` | 1 |

### 4.4 Disposition observable de l'original (informatif)

Racine des données de l'original (Windows) : `%USERPROFILE%\AppData\LocalLow\Geod Studio\3dSen\`. L'utilisateur y voit :

| Élément | Rôle observable |
|---|---|
| `setting.cfg` | réglages (binaire), écrit à la sortie ; illisible → défauts silencieux |
| `input.cfg` | bindings (binaire), écrit à la sortie |
| `rom.json` | bibliothèque, texte JSON indenté : objet avec `"Warning": "Don't touch id parameter,  you will regret it"` et `"Items"`, liste d'objets `{ "id", "romPath", "title", "lastTime" }` (`lastTime` en secondes Unix) |
| `rom.cfg` | ancienne bibliothèque binaire : lue seulement si `rom.json` est absent, supprimée sinon |
| `sav.cfg` | métadonnées des slots, vignettes comprises (binaire) ; illisible → 36 slots vides |
| `sav\1.sav` … `sav\36.sav`, `sav\quick.sav` | savestates |
| `CustomProfiles\*.3dn` | profils utilisateur importés |
| `CoverArt\<Titre>_3dsen_<Id>.png` | jaquettes ; d'anciennes jaquettes `.jpg` de même nom sont lues une fois puis supprimées |
| `Export\` | exports de scène (hors périmètre) |
| `Player.log` | journal d'exécution |

Le dernier dossier du navigateur de fichiers et l'état plein écran sont conservés par l'original dans la base de registre (`HKCU\Software\Geod Studio\3dSen`). Le dossier d'installation contient un dossier `3dn\` réservé au mode éditeur (hors périmètre). Il n'y a ni fichier de RAM par pile, ni dossier par jeu.

### 4.5 Formats de la réimplémentation

Encodage UTF-8 sans BOM, JSON indenté. Les exemples sont normatifs pour la structure, pas pour l'ordre des clés.

#### 4.5.1 `settings.json`

```json
{
  "schemaVersion": 1,
  "values": {
    "camera.fov": 70,
    "audio.volume": 1.0,
    "scene.skybox": -1,
    "scene.custom_bg_color": "#000000FF",
    "light.intensity": 1.0,
    "emu.tv_mode": "NTSC",
    "ui.fullscreen": false
  }
}
```
- booléens → `true`/`false` ; entiers et réels → nombres ; énumérations → libellé de la colonne « Domaine » (le nombre est aussi accepté à la lecture) ; couleurs → `"#RRGGBBAA"` ; vecteurs → tableau de nombres.
- Une clé absente prend sa valeur par défaut.

#### 4.5.2 `library.json`

```json
{
  "schemaVersion": 1,
  "notice": "Do not edit the id field.",
  "games": [
    { "id": "10", "title": "Digdug", "romPath": "D:\\roms\\digdug.nes", "lastPlayed": 1760950000, "cover": "Digdug_3dsen_10.png" }
  ]
}
```

#### 4.5.3 `saves/index.json`

```json
{
  "schemaVersion": 1,
  "slots": [
    { "id": "1", "gameId": "10", "savedAt": 1760950100, "file": "1.state", "thumbnail": "1.png" },
    { "id": "quick", "gameId": "", "savedAt": 0 }
  ]
}
```
37 entrées exactement (`1`…`36`, `quick`) ; `gameId` vide = slot vide.

#### 4.5.4 Fichier de savestate

Structure minimale (encodage binaire libre, petit-boutiste recommandé) : signature de 8 octets `SEN3DSAV`, version de format (entier 32 bits), longueur et contenu de l'en-tête de métadonnées (REQ-PERS-056 point 2, en JSON compact ou binaire), puis les sections « cœur », « Lua » (64 réels double précision), « caméra » et « conversion » (état appris, DEC-R2-T5), chacune précédée de sa longueur. Toute section de longueur incohérente rend le fichier invalide (`Corrupted save state.`).

### 4.6 Table libretro (extrait normatif)

Les réglages U marqués d'une discrétisation dans §4.1 sont exposés. Exemples :

| Core option | Valeurs | Défaut | Catégorie |
|---|---|---|---|
| `3dsen_camera_fov` | 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95 | 70 | Caméra |
| `3dsen_light_intensity` | 0.50, 0.55, …, 1.50 | 1.00 | Lumière |
| `3dsen_scene_skybox` | Default, Color, Gradient, Fantasy | Default | Scène |
| `3dsen_scene_tv_box` | None, TV 1, TV 2, TV 3 | None | Scène |
| `3dsen_render_mesh_algo` | GREEDY, MARCHING, GREEDY_MULTICOLOR | GREEDY | Rendu |
| `3dsen_render_shadow` | enabled, disabled | enabled | Rendu |
| `3dsen_emu_auto_tv_mode` | enabled, disabled | disabled | Émulation |
| `3dsen_emu_game_speed` | −8 … 8 | 0 | Émulation |

Non exposés en libretro (gérés par l'hôte ou sans objet) : `video.vsync`, `video.frame_rate`, `video.skip_frame`, `video.render_method`, `video.res_scale` (la résolution est celle de l'hôte), `ui.*`, `audio.bg_music`, `input.emulated_menu_button`, `emu.core`, tous les réglages P, S, D, VR, éditeur et film. `audio.volume` PEUT être exposé (l'hôte a son propre volume).

## 5. Vecteurs de test

### 5.1 Validation des valeurs (REQ-PERS-003)

| Clé | Valeur lue | Valeur effective |
|---|---|---|
| `video.frame_rate` | 60 / 90 / 120 / 75 / 0 | 60 / 90 / 120 / 90 / 90 |
| `emu.game_speed` | −9 / 3,4 / 8 | −8 / 3,4 / 8 |
| `conv.sprite_tolerance` | −3 | 3 |
| `scene.skybox` | 7 | −1 (défaut) |
| `video.res_scale` | 5 | 0 (défaut) |
| `emu.core` (1 cœur) | 3 | 0 |

### 5.2 Conversions UI ↔ valeur (observées dans l'original)

| Contrôle | Position UI | Valeur du réglage |
|---|---|---|
| Curseur FOV (−5…+5, entier) | −5 / 0 / +5 | 45 / 70 / 95 |
| Curseur Light (0…100, entier) | 0 / 50 / 100 | 0,5 / 1,0 / 1,5 |
| Curseur volume (0…100, entier) | 0 / 37 / 100 | 0 / 0,37 / 1 |
| Liste Skybox | Default / Color / Gradient / Fantasy | −1 / 0 / 1 / 2 |
| Liste TV Region | NTSC / PAL / DENDY | 0 / 1 / 3 |
| Liste Render Rate | 60fps / 90fps / 120fps | 60 / 90 / 120 |
| Liste Tex Scale | Native / 2/3 / 1/2 / 1/3 / 1/4 | 0 / 1 / 2 / 3 / 4 (facteur 1, 0,667, 0,5, 0,333, 0,25) |
| Hotkey vitesse + depuis 8 | — | 8 (borné) |

### 5.3 Vitesse (rappel SPEC-07)

| `emu.game_speed` | −8 | −4 | −1 | 0 | 1 | 4 | 8 |
|---|---|---|---|---|---|---|---|
| facteur k | 1/3 | 1/2 | 0,8 | 1 | 1,25 | 2 | 3 |

### 5.4 Chargement de profil (REQ-PERS-005)

État avant : intensité U = 1,2 ; `Skybox` = 1 ; `Fov` = 80 ; `EmptyTile` = vrai (jeu précédent B) ; `TVMode` = PAL ; `GraphicQuality` = 2.
Profil A embarqué : intensité 1,0, direction (−1, −1, 1), `EmptyTile` faux, `TVMode` NTSC, `GraphicQuality` 0, `BgColor` faux, `Skybox` 0, `Fov` 60.
État après chargement de A, avant le hook de démarrage : intensité 1,2 ; direction (−1, −1, 1) ; `Skybox` 1 ; `Fov` 80 ; `EmptyTile` faux ; `TVMode` NTSC ; `GraphicQuality` 0 ; `BgColor` vrai ; ombres des calques = (cast : 0 0 1 0 0 0 ; recv : 0 0 1 1 1 1).

### 5.5 Cohérence des slots au démarrage (REQ-PERS-055)

| Slot | `gameId` | Fichier présent | Jeu dans la bibliothèque | Résultat |
|---|---|---|---|---|
| 1 | "10" | oui | oui | occupé |
| 2 | "999" | oui | non | vidé, fichier supprimé |
| 3 | "10" | non | oui | vidé |
| quick | "" | — | — | vide |

### 5.6 Chemins d'accès utilisés par les profils embarqués (nombre d'occurrences)

`Setting.EmptyTile` (51), `Setting.Clipping[]` (38), `Setting.Layer[].CastShadow` (26), `Setting.BgColor` (15), `Light.Direction` (13), `Setting.CustomBgColor` (9), `Setting.SamePalette` (9), `Light.Color` (9), `Setting.Layer[]` (8), `Setting.LocalCam` (8), `Setting:GetInputType` (6), `Setting:SetInputType` (6), `Setting.GameSpeed` (4), `Setting.VrRuntime` (4), `Setting.Layer[].Offset` (3), `Setting.Layer[].Aligned` (3), `Setting.TopDownCam` (2), `Setting.Layer[].RecvShadow` (1), `Setting.SmoothTime` (1), `Setting.SameBgPalette` (1), `Setting.SceneRatio` (1). Indices `Layer[]` effectivement utilisés : 2, 3, 4, 5. Valeurs `Aligned` écrites sous forme numérique : 0, 1, 2.

## 6. Questions ouvertes

- **Q-1** — *Close par DEC-09-Q1* (portée P, fidèle). **Portée de `GraphicQuality` et de `TVMode`.** Dans l'original, ces deux réglages ne font pas partie des préférences conservées : ils sont remplacés par la valeur embarquée dans le profil au chargement d'un jeu différent. Conséquences : la qualité choisie par l'utilisateur est écrasée par celle du profil, et la région du profil s'applique **après** la détection automatique (`AutoTVMode`), qui n'a donc d'effet qu'à la relance du même jeu. Cette spec reste fidèle (portée P). Faut-il plutôt traiter la qualité graphique comme une préférence U (écart de classe D) ? Et l'ordre région du profil puis région automatique est-il à conserver ? À trancher avec SPEC-06 et SPEC-07 (REQ-LOOP-017).
- **Q-2** — *Close par DEC-R2-T1 et DEC-09-Q2* (indice 1 = bas, 3 = haut ; §4.1.5 corrigé). **Repère vertical du Clipping.** Le filtrage utilise `y + 8 > c3` et `y ≤ 248 − c1` : le sens de y (vers le haut ou vers le bas) dépend du repère des tiles de SPEC-02. À aligner avec SPEC-04, en particulier le libellé « haut » ou « bas » de `clip.top` et `clip.bottom`.
- **Q-3** — *Close par DEC-09-Q3* (couche de session, D). **Couche de session pour les écritures Lua (REQ-PERS-010).** L'original fait persister les écritures des scripts sur des préférences (couleur de fond personnalisée, vitesse). La couche de session proposée change ce comportement de façon observable : à valider par R0. À défaut, revenir au comportement d'origine.
- **Q-4** — *Close par DEC-09-Q4* (pas de contrôle, information journalisée). **Empreinte ROM dans les savestates.** L'original ne contrôle pas la ROM. Pour « 2D Mode », un slot peut être rechargé alors qu'une autre ROM est associée à l'entrée ; comme l'état du cœur est autonome, le jeu sauvegardé reprend (SPEC-07 REQ-LOOP-046). Faut-il seulement avertir quand l'empreinte diffère, ou ignorer la différence ? Proposition : l'ignorer et la journaliser.
- **Q-5** — *Close par DEC-09-Q5* (8 préréglages). **`scene.custom_bg_color` en libretro.** Les core options ne permettent pas de choisir une couleur libre. Il faut définir une liste de préréglages (proposition : noir, blanc, gris, bleu ciel NES 0x21, bleu foncé, vert, rouge, violet) ; à fixer avec SPEC-06.
- **Q-6** — *Close par DEC-09-Q6* (retour à Manette/Manette, D ; REQ-PERS-012). **Types des ports au changement de jeu.** Le type des ports n'est ni persisté ni réinitialisé au chargement de profil ; ce sont les profils Zapper qui restaurent les types à leur fermeture (SPEC-08 REQ-INP-060). Si un profil utilisateur ne le fait pas, le Zapper reste actif dans le jeu suivant. Faut-il remettre `Manette`/`Manette` à chaque chargement de jeu différent (écart de classe D) ?
- **Q-7** — *Close par DEC-09-Q7* (« Sen », `%LOCALAPPDATA%\Sen`, `portable.txt`, sous-dossier libretro `sen`). **Nom produit et racine utilisateur.** Le nom du dossier `%LOCALAPPDATA%\<NomProduit>` et le sous-dossier système libretro sont à fixer par SPEC-01.

## Historique

- 1.1-draft (AMD-001, relecture R2) : sens des marges de Clipping corrigé (indice 1 = bas, 3 = haut, DEC-R2-T1) et domaine [0 ; 248] ; savestate avec état appris de la conversion (REQ-PERS-056, DEC-R2-T5) ; onglet Save aligné sur REQ-UI-051 ; REQ-PERS-066 en P2 ; « bouton Menu émulé » sans effet (DEC-R2-D7) ; types de ports au changement de jeu (DEC-09-Q6) ; renvoi normatif vers REQ-PROF-041 ; défauts d'ombre de premier lancement non reproduits ; GraphicQuality et res_scale absents de l'UI ; texte d'import renvoyé à SPEC-10 ; forme de §4.4 ; questions closes.
- 1.1 (R2-review-v1.1 N-2, N-3) : `retro_serialize` inclut l'état appris de la conversion (parties 3 à 6, section « conversion ») ; `video.graphic_quality` et `emu.tv_mode` (portée P) retirés des core options (REQ-PERS-085).
