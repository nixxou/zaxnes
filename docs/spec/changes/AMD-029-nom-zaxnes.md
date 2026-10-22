# AMD-029 — Nom du produit « Zaxnes » et réponse à Q-068

Version : 1.0 | 2026-10-22 | **Normatif** (R0, décision de l'utilisateur)

## 1. Noms

- **Produit, dépôt, exécutable, titre de fenêtre** : **Zaxnes** (construction : « Z-axis » + « NES »). L'exécutable s'appelle `Zaxnes.exe`. Le dépôt et l'archive de publication s'appellent `zaxnes`.
- **Code** : les espaces de noms, assemblies, projets, dossiers et la solution passent de `Sen.*` / `Sen.sln` à `Zax.*` / `Zax.sln` (par exemple `Zax.Nes`, `Zax.Engine3D`, `Zax.App`, `Zax.Harness`). Les types préfixés `Sen` (`SenApplication`, `SenUi`…) prennent le préfixe `Zax`.
- **Journal et messages** : le préfixe « Sen: » devient « Zaxnes: ».
- « Sen » reste le **nom de travail historique** dans la spec, les tickets, le journal de salle blanche et les rapports de vérification. On ne les réécrit pas.

## 2. Compatibilité (obligatoire)

- **Données utilisateur** : la racine utilisateur passe de `%LOCALAPPDATA%\Sen` à `%LOCALAPPDATA%\Zaxnes`. Le mode portable reste `userdata\` à côté de l'exécutable. Au premier lancement, si la nouvelle racine n'existe pas et que l'ancienne existe, son contenu est **copié** (pas déplacé) dans la nouvelle racine, puis un journal le signale. L'ancienne racine n'est jamais modifiée ni supprimée.
- **Savestates** : l'écriture utilise une nouvelle signature propre au produit. La lecture accepte la nouvelle signature **et** l'ancienne (`SEN3DSAV`).
- **Jaquettes** : l'infixe `_3dsen_` (REQ-UI-200/201) est **conservé**. C'est une donnée de format qui permet de relire les jaquettes existantes.

## 3. Réponse à Q-068

- **REQ-UI-084** (échec d'import) : « …the current version of 3dSen. » devient « …the current version of Zaxnes. ». C'est la seule modification du texte exact.
- **REQ-UI-076** (À propos) : le bloc présente Zaxnes et garde une **attribution factuelle** : « Projet indépendant inspiré de 3dSen (Geod Studio) ; contient du code adapté de 3DNes_Engine (Geod Studio, GPL v3). Sans lien avec Geod Studio ni Nintendo. »
- **REQ-PERS-083** : le préfixe des clés d'option libretro devient `zaxnes_`.
- **Jaquettes** : voir §2.
- Q-068 est clos.
