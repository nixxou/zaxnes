# AMD-063 — 3dSen 2.1 : objets transparents sans grille, membres Lua de la 2.1

Version : 1.2 | 2026-11-09 | **Normatif** (R1, choix du propriétaire du 2026-11-09 : reporter la
2.1 sur `dotnetupdated`). **Amende** AMD-057 (§2, « Parois entre tuiles »), **AMD-058 §2** (le
retrait des parois cesse d'être un moyen exclu pour les surfaces transparentes ; il le reste pour
les surfaces opaques), **AMD-059 REQ-LUA-117** (un champ de `CS.Nes3D.CameraControl` dont
l'écriture a un effet), **AMD-042 REQ-LOOP-069 et REQ-LOOP-070** (contenu et restauration d'un
snapshot), le critère d'AMD-057 §2 (objet opaque) et AMD-040
(`Info.Version`) ; **précise** SPEC-06 REQ-SCN-040 et REQ-SCN-065 ; **complète** SPEC-05 §4.2.
Ajoute REQ-SCN-087, REQ-LUA-118 à REQ-LUA-121.
Priorité : P2.
Historique : 1.0 (rédaction) ; 1.1 (relecture `R2-review-AMD-062-063.md` : 63-1 à 63-13) ;
1.2 (relecture `R2-review-AMD-062-063-v1.1.md` : N-63-1 à N-63-6).

## 1. Constat

- **Grille des objets transparents.** Depuis AMD-057, chaque tuile est dessinée avec toutes ses
  faces, comme dans 3dSen 2.0. Sur un objet transparent fait de tuiles pleines voisines (eau,
  verre, glace), le passage de profondeur de REQ-SCN-040 laisse visible, au raccord de deux
  tuiles, un liseré d'un pixel dessiné par la paroi latérale confondue avec la jointure : une
  grille régulière et pâle apparaît. 3dSen 2.1 la supprime (notes de version : « the faint grid
  between the blocks of see-through objects (water, glass) is gone »). Profils concernés, par
  exemple : Super Mario Bros. 3, Felix the Cat, Fire 'n Ice, Ninja Gaiden III, Blaster Master,
  Bubble Bobble (variantes de fond d'opacité 0,1 à 0,8, le plus souvent 0,5 ou 0,7 ; 628
  variantes d'opacité inférieure à 1 sur les 101 profils, aucune supérieure à 1). 43 des 101
  profils changent en outre l'opacité d'instances par script (`.Alpha`).
- **API Lua.** La 2.1 ajoute trois membres et passe `CS.Nes3D.Info.Version` à `"2.1"`. Aucun
  des 101 profils livrés ne les utilise ; ils sont repris pour les profils écrits pour la 2.1.

## 2. Objets transparents sans grille

**REQ-SCN-087** — Définitions :
- une variante (SPEC-03 §4.4) est **à faces internes retirables** si son algorithme de mesh
  effectif n'est pas MARCHING (SPEC-04B ; `rm = 0` suit le réglage global), quelles que soient son
  opacité et son style de rendu ;
- une tuile d'une telle variante est **pleine** si `D` (le `sizeZ` du profil) est > 0 et si, dans le champ de
  8 × 8 × `D` voxels construit pour elle (SPEC-04B REQ-E3D-420), **les trois fentes de chaque
  voxel** sont non vides (REQ-E3D-422), `emptyColor` compris (REQ-E3D-424). Les géométries creuses
  (`CUBE`, cylindres) ne produisent donc pas de tuile pleine, ce qui est voulu ;
- une instance de frame est **transparente** si son opacité (`Alpha` de l'instance, SPEC-05 §4.2,
  éventuellement changée par un script) est strictement inférieure à 1 et son style de rendu (de
  l'instance) n'est pas 2 ; sinon elle est **opaque**. Seule l'instance dessinée décide de
  l'opacité : la valeur `alpha` du profil n'intervient que par elle.

À chaque frame dessinée :
1. **Carte du fond.** Pour chaque cellule de la grille de fond (35 × 33, SPEC-04 §2), on retient
   au plus une instance : parcourant la liste ordonnée des instances de la frame (REQ-E3D-040, y
   compris celles des shapes restaurées d'une frame épinglée), pour chaque instance d'une shape
   **de fond** et pour chaque tuile de cette shape, la cellule de la tuile reçoit cette instance si
   elle n'en a pas encore, ou si l'instance retenue est opaque et la nouvelle transparente.
2. **Faces retirées.** Pour chaque tuile dessinée d'une instance **de fond transparente** dont la
   variante d'origine (`refPattern3D`) déclare cette tuile pleine, on retire sa face latérale
   **gauche**, **droite**, **haute** ou **basse** quand la cellule voisine de ce côté
   (4-voisinage dans la grille de fond) a une instance retenue à l'étape 1 qui est transparente et
   dont la tuile de cette cellule est pleine pour la variante d'origine de cette instance. Une
   cellule voisine hors de la grille ne retire rien. Les faces avant et arrière, toutes les faces
   d'une tuile non pleine, et toutes les faces des tuiles de sprites sont toujours dessinées.
3. Le reste du dessin (ombres, ordre de tri, passage de profondeur, matériau transparent) est
   inchangé. Une tuile opaque garde toutes ses faces (AMD-057 inchangé pour elle). SPEC-04B
   REQ-E3D-425 continue de régir le champ de voxels et le maillage **construit** d'une tuile ; le
   retrait porte sur le maillage **dessiné**, tuile dessinée par tuile dessinée.

AMD-057 « Parois entre tuiles » s'entend désormais **hors** REQ-SCN-087. AMD-058 §2 est amendé :
l'exclusion « du retrait des parois » ne vaut plus que pour les surfaces opaques. REQ-SCN-065 (ni
grille ni fissure à la jointure de deux tuiles voisines d'une surface plane) s'applique aussi aux
surfaces **transparentes** faites de tuiles pleines. REQ-SCN-040 est précisée : son passage de
profondeur ne suffit pas à masquer les parois confondues avec une jointure ; REQ-SCN-087 les
retire. Pour une surface transparente, le critère de REQ-SCN-065 se mesure sur fond uniforme, la
« couleur de la surface » étant la couleur mélangée attendue. Le critère d'AMD-057 §2 (« chacune
a 24 sommets ») porte sur un objet **opaque**.
- Critère d'acceptation (auto) : scène synthétique, objet de fond 4 × 4 tuiles identiques,
  `geo = Default`, `fullWarp = faux`, `emptyColor = 0`, `sizeZ = 4`, aucun pixel de couleur 0 (donc
  tuiles pleines), opacité 0,7, GREEDY ; lecture sur l'export par tuile dessinée (`--dump-meshes`, Q-081)
  → les 4 tuiles d'angle ont 2 faces latérales retirées (16 sommets), les 8 autres tuiles de bord
  3 (12 sommets), les 4 intérieures 4 (8 sommets) ; avec opacité 1 → toutes ont 24 sommets.
- Critère d'acceptation (auto) : même scène avec une tuile non pleine (un pixel vide) au milieu
  → cette tuile garde toutes ses faces, et ses voisines gardent la face qui la touche.
- Critère d'acceptation (auto) : deux objets de fond transparents différents côte à côte, tuiles
  pleines → la face commune est retirée des deux côtés ; un objet transparent contre un objet
  opaque → la face est gardée des deux côtés ; un objet `geo = CUBE` transparent → aucune face
  retirée ; un objet de sprites transparent de tuiles pleines → aucune face retirée ; la scène du
  premier critère avec une variante d'opacité 1 dont un script met `shape.Alpha = 0.7` → les faces
  internes sont retirées comme pour une variante d'opacité 0,7.
- Critère d'acceptation (revue, R5) : une surface d'eau ou de glace transparente de Super Mario
  Bros. 3 ou de Fire 'n Ice ne montre plus de grille, comme dans 3dSen 2.1.
- Notes : une implémentation PEUT préparer les 15 variantes de maillage d'une tuile pleine (une
  par combinaison de côtés retirés) ou retirer les faces au dessin ; le résultat dessiné est le
  même. S'il reste un écart visuel avec 3dSen 2.1 après ce lot, il sera mesuré en R5.

## 3. Lua

**REQ-LUA-118** — `CS.Nes3D.Info.Version` vaut `"2.1"` (amende AMD-040). Le critère d'AMD-040
(`tonumber(CS.Nes3D.Info.Version:match("^%d+")) >= 2` → vrai) reste vrai.
- Critère d'acceptation (auto) : `CS.Nes3D.Info.Version` → `"2.1"`.

**REQ-LUA-119** — `CS.Nes3D.CameraControl.RotOverride` : champ en lecture-écriture, `nil` par
défaut ou une valeur `Vector3` (degrés, mêmes axes que `Camera.Rot`, SPEC-05 `CameraAPI`).
- Tant qu'il n'est pas `nil`, à chaque mise à jour de la caméra (SPEC-06 REQ-SCN-007), la
  consigne de rotation du **mode de caméra courant** prend cette valeur, et la rotation affichée
  est cette valeur **immédiatement**, sans le lissage de REQ-SCN-019.
- Remis à `nil`, la caméra reprend son comportement normal à partir de la consigne courante (qui
  vaut la dernière valeur imposée).
- Lire le champ donne `nil` ou une copie `Vector3` de la valeur ; écrire autre chose qu'un
  `Vector3` ou `nil` est une erreur Lua.
- `RotOverride` est le **seul** champ de `CS.Nes3D.CameraControl` dont l'écriture ait un effet
  (amende AMD-059 REQ-LUA-117) ; la règle « tout autre champ vaut nil » reste vraie pour le reste.
- La valeur est une valeur de session : elle n'est pas persistée, ne fait pas partie de l'état
  de caméra des savestates (REQ-SCN-024 inchangée) et revient à `nil` au lancement d'un jeu et au
  changement de profil. Elle fait partie des **snapshots de rollback** : AMD-042 REQ-LOOP-069
  reçoit un quatrième élément, « la valeur de `CS.Nes3D.CameraControl.RotOverride` », et
  REQ-LOOP-070 réécrit de même cette valeur, avant le rétablissement de la caméra ; un rollback
  restaure donc cette valeur et la consigne de caméra du snapshot.
- Critère d'acceptation (auto) : `CS.Nes3D.CameraControl.RotOverride = Vector3(-60, 10, 0)` puis
  une frame → `Camera.Rot` vaut (−60, 10, 0) et la rotation rendue aussi, dès cette frame ;
  remis à `nil` → `CS.Nes3D.CameraControl.RotOverride == nil` et la caméra reste à (−60, 10, 0)
  jusqu'à une nouvelle consigne ; `CS.Nes3D.CameraControl.DistanceMax` → 512 et
  `CS.Nes3D.CameraControl.Autre` → nil (REQ-LUA-117 intacte) ; écrire `RotOverride` à la frame 50,
  revenir par rollback à la frame 45 → `RotOverride` vaut de nouveau `nil`.

**REQ-LUA-120** — `Pattern.HasPlaceholders` (propriété, lecture seule, booléen) : pour un pattern
du profil, vrai si et seulement si sa grille contient au moins une tuile **en attente** au sens
d'AMD-062 REQ-PROF-081 ; toujours faux pour un pattern d'un profil 3dn. Pour un **pattern de
frame** (SPEC-05 §4.2), la propriété rend la valeur du pattern de référence de sa shape
(`refPattern` de la shape, SPEC-05 §4.2).
- Critère d'acceptation (auto) : profil 3dh dont une tuile d'un pattern n'est pas encore
  résolue → `shape.refPattern.HasPlaceholders` et `shape.pattern.HasPlaceholders` valent vrai, et
  `Pattern.GetPatternWithTag(t).pattern.HasPlaceholders` (REQ-LUA-085) aussi ; après résolution
  de cette tuile, faux partout ; avec un profil 3dn, toujours faux.

**REQ-LUA-121** — `Instance:IsSolidTile(t)` (méthode) renvoie **toujours** faux, pour tout
argument, sans lever d'erreur.
- Critère d'acceptation (auto) : `shape:IsSolidTile(nil)` → faux, `shape:IsSolidTile(3)` → faux,
  aucune erreur, les hooks du script restent actifs (REQ-LUA-062).
- Notes : dans 3dSen 2.1, la méthode rend vrai quand `t` est une tuile pleine au sens de
  REQ-SCN-087. Zaxnes n'expose pas d'objet tuile (SPEC-05 §4.2) : la méthode existe pour qu'un
  script qui l'appelle ne perde pas ses hooks, comme `BuildExportMeshes()` (AMD-040 REQ-LUA-115).

## 4. Écarts assumés et sans objet

- **(D) `IsSolidTile`** : toujours faux faute d'objet tuile (REQ-LUA-121).
- **(D) Sprites** : 3dSen 2.1 applique aussi le retrait à des tuiles de sprites transparentes, en
  consultant la carte du fond avec les coordonnées de leur propre grille ; Zaxnes le réserve aux
  instances de fond (REQ-SCN-087 étape 2).
- **(D) Opacité au-delà de 1** : une instance d'`Alpha` > 1 est opaque pour Zaxnes partout dans
  REQ-SCN-087 (aucun profil livré ne le fait).
- Le correctif CPU de la 2.1 (écriture double des instructions lecture-modification-écriture,
  écriture du MMC1 au cycle suivant ignorée) et celui des maillages de plus de 65 536 sommets
  correspondent déjà au comportement de Zaxnes (SPEC-02, index 32 bits) : rien à faire.
- La boîte « Continue » de la 2.1 et le message de sortie concernent le point de reprise de la
  2.0, non retenu (rapport delta 2.0 §4.5) : rien à faire.

## Historique

- 1.0 (2026-11-09) : rédaction (R1) après l'étude du delta 2.0 → 2.1.
- 1.1 (2026-11-09) : relecture R2. Grille 35 × 33 (63-1) ; AMD-058 §2 amendé, REQ-SCN-040
  précisée, constat reformulé (63-2) ; tuile pleine par les trois fentes, `D > 0` (63-3) ;
  périmètre réservé au fond (63-6) ; renvoi `hidden` retiré (63-7) ; un seul prédicat d'opacité
  (63-4) ; carte construite sur les tuiles de chaque shape (63-5) ; REQ-E3D-425 et export par tuile
  dessinée (63-13) ; lissage REQ-SCN-019 (63-8) ; AMD-059 amendé (63-9) ; snapshot de rollback
  (63-10, option b) ; pattern de frame (63-11) ; énoncé de REQ-LUA-121 (63-12).
- 1.2 (2026-11-09) : relecture R2b. Opacité décidée par la seule instance dessinée, variante
  retirable sans condition d'opacité (N-63-1) ; plage d'opacité mesurée (N-63-2) ; renvois de
  REQ-LUA-120 (N-63-3) ; REQ-LOOP-070 (N-63-4) ; critères d'AMD-057 et de REQ-SCN-065 précisés
  (N-63-5) ; scène synthétique entièrement fixée (N-63-6).
