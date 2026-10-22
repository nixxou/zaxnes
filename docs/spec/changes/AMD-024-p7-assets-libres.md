# AMD-024 — Assets libres et version publiable (phase P7)

Version : 1.0 | 2026-10-21 | **Normatif** pour la version publiable. Origine : décision de l'utilisateur (feuille de route post-P6, PLAN.md §4bis) et inventaire `legal/P7-ASSET-INVENTORY.md`.

## Principe
La version publiable ne contient **aucun fichier issu du build d'origine** : ni profil, ni texture, ni cubemap, ni modèle, ni son, ni police. Tout asset nécessaire est **généré par le code** ou **lu sur le système de l'utilisateur à l'exécution, sans redistribution**. Les exigences dont le critère dépendait de l'aspect exact d'un asset d'origine sont reclassées en divergences **D** pour cette version ; leurs fonctions restent exigées.

## Exigences
**REQ-PUB-001 (P1)** — L'application DOIT démarrer et être pleinement utilisable sans le dossier `assets-neutral` : écran d'accueil, ouverture de ROM, mode 2D, 3D avec la géométrie par défaut (sans profil), import de profils `.3dn`, savestates, options, touches. Critère : lancement du build publié avec un dossier d'assets vide ou absent ; Elevator Action, sans profil, se joue 60 s en 3D ; l'import d'un `.3dn` fonctionne.

**REQ-PUB-002 (P1)** — Le dépôt et le build publiés NE DOIVENT contenir aucun fichier dont le contenu proviendrait du build d'origine. Critère : aucune empreinte SHA-256 d'un fichier publié ne figure dans `assets-neutral/inventory.json` ; aucun fichier `.bytes`, `.3dn`, `.png`, `.obj`, `.wav` ni `.ttf` n'est présent, hormis ceux générés par le projet et listés dans `ASSETS-LICENSES.md`.

**REQ-PUB-003 (P1)** — Les skyboxes « Gradient » et « Fantasy » sont **procédurales** (shader), avec les paramètres fonctionnels de SPEC-06 (couleurs, rotation, soleil et lune, nuages, étoiles). L'aspect exact des cubemaps d'origine n'est plus exigé **(D)**. Critère : chaque skybox se rend sans aucun fichier image ; changer ses paramètres change l'image.

**REQ-PUB-004 (P2)** — Les trois modèles de boîtier TV sont **construits par le code** : géométrie paramétrique et matériaux procéduraux. Ils respectent la géométrie fonctionnelle de SPEC-06 (tunnel, ouverture de l'écran sur le rectangle visible, ombres des touches et absence d'ombre du corps selon AMD-021). L'aspect exact des modèles d'origine n'est plus exigé **(D)**. Critère : les trois TV se rendent sans fichier ; le rectangle visible n'est pas masqué.

**REQ-PUB-005 (P2)** — *Révisée le 2026-10-21 (décision de l'utilisateur)* : **aucune musique de menu**. Les sons d'interface sont **synthétisés par le code**, un son distinct par événement de SPEC-11. La musique de menu de SPEC-11 (pistes, tirage aléatoire, reprise, fondus de la musique) est **retirée** **(D)**. Le réglage de musique de fond reste dans le modèle de réglages pour la compatibilité des données, sans effet, et n'est plus affiché dans les options. L'audio du jeu émulé n'est pas concerné. Critère : chaque événement de SPEC-11 produit un son non silencieux ; aucune musique ne joue sur l'écran d'accueil ni dans les menus.

**REQ-PUB-006 (P2)** — Police : l'application charge à l'exécution une police installée sur le système (Windows : Segoe UI ou Arial), **sans la redistribuer**, avec repli sur la police intégrée d'ImGui. Les textes français accentués et le glyphe « ► » restent lisibles avec la police système ; le repli peut les remplacer par des équivalents ASCII. Critère : capture de l'écran d'accueil avec la police système, et de l'écran d'accueil avec le repli forcé.

**REQ-PUB-007 (P1)** — Les données d'interopérabilité nécessaires à la lecture des profils `.3dn` importés (clé d'encodage des tiles de SPEC-03) sont intégrées au code ou à un fichier de données du projet.

**REQ-PUB-008 (P1)** — Un fichier `ASSETS-LICENSES.md` à la racine liste chaque asset livré ou généré, son origine (« généré par le code du projet, GPL-3.0 »), et les composants tiers avec leur licence (ImGui MIT, Silk.NET MIT, KeraLua et Lua MIT, 3DNes_Engine GPL-3.0, etc.). Le README explique que **les profils de jeux et les ROMs ne sont pas fournis**, comment importer un profil `.3dn` dont on dispose légalement, et que le projet est **une réimplémentation indépendante, sans lien avec Geod Studio ni 3dSen**.

**REQ-PUB-009 (P2)** — Le nom du produit, de l'exécutable et du dépôt ne contient pas la marque « 3dSen ». « Sen » reste le nom de travail ; le nom définitif est confirmé par l'utilisateur avant la mise en ligne.

## Exigences reclassées (D pour la version publiable)
Les critères d'aspect qui reposaient sur les assets d'origine : SPEC-06 REQ-SCN-050/051 (aspect des cubemaps), REQ-SCN-053/054 et AMD-021 R2-014 (aspect, textures et albédo 0,208 des modèles TV d'origine), SPEC-11 (clips audio précis), SPEC-10 et AMD-010 (police). Leurs **fonctions** restent exigées, comme indiqué ci-dessus.

## Mode « assets d'origine » (local, facultatif)
L'option `--assets <dossier>` existante PEUT continuer à charger un dossier d'assets fourni par l'utilisateur, pour un usage local avec ses propres fichiers. Elle n'est jamais alimentée par la version publiée.

## Règle de création des assets de remplacement (décision de l'utilisateur, 2026-10-21, normative)
Les auteurs des assets de remplacement **peuvent consulter directement les assets d'origine** (`assets-neutral/`) **et s'en inspirer, y compris pour l'aspect** : formes, proportions, palette, ambiance, composition générale, caractère des sons.
Restent **interdits** :
- la copie de fichiers, de pixels, d'échantillons audio ou de géométrie (aucun contenu d'origine, même transformé mécaniquement : recolorisé, redimensionné, filtré, ré-échantillonné) ;
- une **composition strictement identique** : le résultat doit être une création nouvelle, reconnaissable comme telle au premier coup d'œil (ou à la première écoute) quand on la compare à l'original.
REQ-PUB-002 (aucun fichier d'origine, vérifié par empreintes) reste inchangée.
