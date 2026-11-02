# AMD-059 — API Lua de 3dSen 2.0 : membres restants

Version : 1.0 | 2026-11-02 | **Normatif** (R1, demande du propriétaire du 2026-11-02 : « le support du nouveau Lua doit être complet »). **Complète AMD-040** (REQ-LUA-112 et REQ-LUA-114) et SPEC-05 §4.2 (`SettingAPI`). AMD-055 REQ-SCN-078 (identifiants de style, 1 accepté) est inchangé.
Priorité : P2.

## 1. Constat

Vérification en boîte noire contre 3dSen 2.0 (passe R5) :

- **Profils réels** : les 151 membres Lua distincts que lisent ou écrivent les scripts des 101 profils intégrés de la 2.0 existent tous dans Zaxnes. Les 19 profils dont la ROM est disponible ont tourné 1 500 frames, avec et sans entrées, sans **aucune** erreur Lua.
- **Surface exposée par la 2.0 et absente de Zaxnes** : un même script de sonde, exécuté dans les deux programmes, montre que les membres ajoutés en 2.0 et non encore spécifiés **lèvent une erreur en écriture** dans Zaxnes (« cannot set …, no such field »), ce qui coupe tous les hooks du script fautif (REQ-LUA-062), et renvoient nil en lecture. Valeurs relevées dans 3dSen 2.0 (réglages d'usine) :

| Lecture | 3dSen 2.0 | Zaxnes avant |
|---|---|---|
| `Setting.LibraryView` | 0 | nil |
| `Setting.MaxQueuedFrames` ; après écriture de 5 | 2 ; 2 | nil ; **erreur** |
| `Setting.RefreshRate` ; après écriture de 100 | 90 ; 90 | nil ; **erreur** |
| `Setting.Msaa` ; après écriture de 3 | 2 ; 2 | nil ; **erreur** |
| `Setting.VisualStyleParameters` | une liste | nil |
| `CS.Nes3D.CameraControl.DistanceMax` | 512 | **erreur** (index d'un nil) |
| `CS.Nes3D.SettingManager.DefaultMsaa` | 2 | **erreur** |

Les membres déjà spécifiés par AMD-040 donnent les mêmes valeurs des deux côtés (`CS.Nes3D.Info.Version`, `Camera.ResetDistance`, `Setting.Fov`, `SceneScale`, `RunAheadFrames` en lecture, `Get/Set/ResetVisualStyleParameter(s)` avec le style 4, `BuildExportMeshes`, `Shape2D.Count`), aux écarts assumés près (§3).

## 2. Exigences

**REQ-LUA-116** (P2) — **Membres de `Setting` ajoutés par la 2.0.** `SettingAPI` (SPEC-05 §4.2) DOIT offrir :

| Membre | Accès | Type | Défaut | Écriture d'une valeur v (entier, partie fractionnaire tronquée) | Effet dans Zaxnes |
|---|---|---|---|---|---|
| `LibraryView` | L, É | entier | 0 | v est mémorisé tel quel | aucun ; persisté (clé `ui.library_view`, portée U) |
| `MaxQueuedFrames` | L, É | entier | 2 | v ≤ 1 → 1, sinon 2 | **c'est** le réglage `video.max_queued_frames` d'AMD-052 (REQ-LOOP-077) : même valeur, même effet |
| `RefreshRate` | L, É | entier | 90 | remplacé par la valeur la plus proche de {72, 90, 120} ; à égalité de distance, la **plus petite** (81 → 72, 105 → 90) | aucun (fréquence d'un casque de réalité virtuelle) ; persisté (clé `vr.refresh_rate`, portée U) |
| `Msaa` | L, É | entier | 2 | v ≤ 1 → 0 ; 2 ≤ v ≤ 3 → 2 ; v ≥ 4 → 4 | aucun : l'anticrénelage reste `video.anti_alias` (AMD-032) ; persisté (clé `video.msaa_2_0`, portée U) **(D)** |
| `VisualStyleParameters` | L, É | liste | vide | voir ci-dessous | — |

- Aucune de ces écritures NE DOIT lever d'erreur, quelle que soit la valeur numérique écrite.
- **`VisualStyleParameters`** : en lecture, une liste (indices 0 à `Count − 1`, comme les autres listes de SPEC-05) des paramètres de style **écrits** depuis leur dernière remise à zéro, dans l'ordre de leur première écriture. Chaque élément offre en lecture `Style` (entier, identifiant du style tel que relu par REQ-SCN-078, donc 4 pour Enhanced), `ParameterId` (chaîne) et `Value` (nombre, valeur normalisée). `SetVisualStyleParameter` qui renvoie vrai ajoute l'élément ou met à jour sa valeur ; `ResetVisualStyleParameters(style)` retire ceux du style. En écriture, nil remet **tous** les paramètres de tous les styles à leur défaut et vide la liste ; une liste précédemment lue rétablit exactement ses éléments (les autres paramètres reviennent à leur défaut) ; toute autre valeur lève une erreur Lua.
- Les quatre réglages persistés reçoivent leur ligne dans SPEC-09 §4.1, sans interface (`UI` : aucune ; libretro : non exposé), sauf `MaxQueuedFrames`, qui est la ligne existante d'AMD-052.
- Critère d'acceptation (auto) : `Setting.MaxQueuedFrames = 5` → 2 ; `= 0` → 1 et `video.max_queued_frames` vaut 1.
- Critère d'acceptation (auto) : `Setting.RefreshRate = 100` → 90 ; `= 81` → 72 ; `= 105` → 90 ; `= 200` → 120.
- Critère d'acceptation (auto) : `Setting.Msaa = 3` → 2 ; `= 1` → 0 ; `= 8` → 4 ; `video.anti_alias` inchangé.
- Critère d'acceptation (auto) : `Setting.LibraryView = 7` → 7.
- Critère d'acceptation (auto) : après `ResetVisualStyleParameters(4)`, `SetVisualStyleParameter(4, "spl.bend_zx", 40)` et `SetVisualStyleParameter(4, "spl.ao", 2)` → `VisualStyleParameters.Count` = 2, élément 0 : `Style` 4, `ParameterId` `"spl.bend_zx"`, `Value` 30 ; `Setting.VisualStyleParameters = nil` → `Count` 0 et `GetVisualStyleParameter(4, "spl.bend_zx")` → 0.

**REQ-LUA-117** (P2) — **Complément de la table `CS`** (amende REQ-LUA-112, dont la règle « tout autre champ vaut nil » reste vraie pour tout ce qui n'est pas listé ici). `CS.Nes3D` DOIT contenir en plus :

| Chemin | Valeur |
|---|---|
| `CS.Nes3D.CameraControl.DistanceMax` | 512 (la borne de REQ-SCN-011) |
| `CS.Nes3D.SettingManager.DefaultMsaa` | 2 |
| `CS.Nes3D.VisualStyle.Classic` | 0 |
| `CS.Nes3D.VisualStyle.SoftPixelLight` | 4 (Enhanced) |

- Ces valeurs sont des nombres, comparables à `Setting.VisualStyle` et utilisables comme argument de `Get/Set/ResetVisualStyleParameter(s)` et en écriture de `Setting.VisualStyle`. Une écriture dans ces tables n'a aucun effet sur le programme. Tout autre champ de `CS.Nes3D.CameraControl`, `CS.Nes3D.SettingManager` ou `CS.Nes3D.VisualStyle` vaut nil.
- Critère d'acceptation (auto) : `CS.Nes3D.CameraControl.DistanceMax` → 512 ; `CS.Nes3D.SettingManager.DefaultMsaa` → 2 ; `Setting.VisualStyle = CS.Nes3D.VisualStyle.SoftPixelLight` puis `Setting.VisualStyle == CS.Nes3D.VisualStyle.SoftPixelLight` → vrai ; `CS.Nes3D.CameraControl.Autre` → nil.

## 3. Écarts assumés avec 3dSen 2.0

- **(D) `RunAheadFrames`** : la 2.0 mémorise toute valeur ≥ 0 écrite par Lua (7 relu 7) ; Zaxnes garde la borne [0, 3] d'AMD-040 / AMD-041, seule plage que l'émulation exécute.
- **(D) Style 1** : la 2.0 n'a pas de style 1 (`SetVisualStyleParameter(1, …)` → faux) ; Zaxnes garde l'alias d'AMD-055 REQ-SCN-078.
- **(D) `Setting.VisualStyle` est un nombre** dans Zaxnes, un énuméré dans la 2.0 (`tostring` donne « Classic: 0 ») ; les comparaisons avec `CS.Nes3D.VisualStyle.*` donnent le même résultat.
- **(D) `Msaa` sans effet** : dans la 2.0 il fixe l'anticrénelage à la session suivante ; Zaxnes a son propre réglage (`video.anti_alias`, AMD-032) qu'un script ne doit pas changer à l'insu de l'utilisateur.
- **(D) `RefreshRate`** : la 2.0 le ramène aussi à 90 quand un effet d'écran du style actif est allumé ; sans réalité virtuelle dans Zaxnes, cette règle n'est pas reprise.
- **(D) Reste de `CS`** : la 2.0 donne aux scripts l'accès à **tout** le code C# du programme par `CS` ; Zaxnes n'expose que les chemins listés (REQ-LUA-112, REQ-LUA-117). `CS.Nes3D.Info.Title` (« Zaxnes ») et `AppId` (nil) restent les choix d'AMD-040.

## Historique

- 1.0 (2026-11-02) : rédaction après la sonde Lua exécutée dans 3dSen 2.0 et dans Zaxnes (`verify/delta-v2/harness/lua2/`, `R5-report.md` « API Lua 2.0 »).
