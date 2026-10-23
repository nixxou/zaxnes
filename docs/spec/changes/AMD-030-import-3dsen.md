# AMD-030 — Importer les profils d'une installation de 3dSen (P8, « bring your own 3dSen »)

Version : 1.0 | 2026-10-22 | **Normatif** (R0, décision de l'utilisateur)

## 1. Besoin

Zaxnes ne livre aucun profil commercial. Un utilisateur qui possède 3dSen doit pouvoir importer en une action les profils intégrés à **sa propre installation** dans sa bibliothèque de profils (SPEC-09 et SPEC-10, profils importés `.3dn`). Rien n'est téléchargé ni redistribué : seule l'installation locale de l'utilisateur est lue.

## 2. Interface

- **Fichier → Importer depuis 3dSen…** ouvre un sélecteur de dossier. L'utilisateur désigne le dossier d'installation de 3dSen (celui qui contient `3dSen.exe`) ou directement le fichier bundle (§3). La même action existe en ligne de commande : `--import-3dsen <dossier|fichier>`, qui importe puis quitte avec le code 0 si tout s'est bien passé.
- **Fin de l'import** : un message donne le nombre de profils importés, déjà présents (ignorés) ou en échec. Les profils importés apparaissent aussitôt dans la bibliothèque.
- **Doublons** : un profil dont le contenu est identique, octet pour octet, à un profil déjà importé est ignoré. Si le contenu diffère mais que le nom de fichier est le même, le nouveau profil reçoit un suffixe ` (2)`, ` (3)`…

## 3. Localisation du fichier

Depuis le dossier d'installation, on cherche dans l'ordre :
1. `<dossier>/<n'importe quel nom>_Data/StreamingAssets/aa/<plateforme>/defaultprofiles_assets_all_*.bundle`, où `<plateforme>` vaut `StandaloneWindows`, `StandaloneWindows64`, `StandaloneLinux64` ou `StandaloneOSX` (le premier trouvé gagne) ;
2. sous macOS, le même chemin dans `<dossier>/*.app/Contents/Resources/Data/StreamingAssets/aa/<plateforme>/`.

La partie `*` est une empreinte qui change selon la version : on n'en suppose jamais la valeur. Si rien n'est trouvé, l'interface le dit clairement et rappelle le chemin attendu.

## 4. Format du bundle (format Unity public, mesuré sur une installation réelle : Unity 2019.4.41f2)

Le format est celui des AssetBundles Unity « UnityFS », documenté publiquement par des projets libres de lecture d'assets Unity. Les valeurs entre crochets sont celles mesurées ; le lecteur doit accepter toute valeur valide décrite ici, et refuser le reste avec un message clair.

### 4.1 En-tête (gros-boutiste)

| Champ | Type | Valeur mesurée |
|---|---|---|
| signature | chaîne terminée par 0 | `UnityFS` |
| version de format | u32 | [7] (accepter 6 à 8) |
| version du lecteur | chaîne terminée par 0 | [`5.x.x`] |
| version du moteur | chaîne terminée par 0 | [`2019.4.41f2`] |
| taille totale | i64 | taille du fichier |
| taille compressée du bloc d'information | u32 | [1438] |
| taille décompressée du bloc d'information | u32 | [3341] |
| drapeaux | u32 | [0x43] |

- Bits 0 à 5 des drapeaux : compression du bloc d'information (0 aucune, 1 LZMA, 2 LZ4, 3 LZ4HC ; LZ4 et LZ4HC se décodent pareil).
- Bit 0x40 : le bloc d'information contient aussi la liste des fichiers.
- Bit 0x80 : le bloc d'information est à la **fin** du fichier, au lieu de suivre l'en-tête.
- Si la version de format est au moins 7, la position qui suit l'en-tête est **alignée sur 16 octets** avant de lire le bloc d'information [l'en-tête finit à 50, lecture à 64].
- LZMA n'est pas à prendre en charge (refus clair) : seuls aucune compression, LZ4 et LZ4HC sont exigés.

### 4.2 Bloc d'information (gros-boutiste, après décompression)

- 16 octets d'empreinte (ignorés), puis un i32 donnant le nombre de blocs de données [326].
- Pour chaque bloc : taille décompressée u32 [131072 sauf le dernier], taille compressée u32, drapeaux u16 (bits 0 à 5 : compression, mêmes codes) [3].
- Un i32 donnant le nombre de fichiers [1].
- Pour chaque fichier : décalage i64 et taille i64 dans le flux décompressé, drapeaux u32 [4], chemin en chaîne terminée par 0 [`CAB-` suivi de 32 chiffres hexadécimaux].

Les blocs de données se suivent juste après le bloc d'information (ou après l'en-tête aligné si le bloc d'information est à la fin). Les décompresser à la suite donne un flux continu, dans lequel chaque fichier se lit par son décalage et sa taille.

### 4.3 Fichier sérialisé (le fichier `CAB-…`)

- **En-tête** (gros-boutiste) : taille des métadonnées u32 [4537], taille du fichier u32, version u32 [21], décalage des données u32 [4560], puis un octet d'ordre des octets (0 petit-boutiste [0], 1 gros-boutiste) et 3 octets réservés.
  - Pour une version au moins 22 : ces tailles et décalages sont repris en champs 64 bits juste après.
  - Accepter les versions 17 à 22.
- **Métadonnées**, dans l'ordre des octets annoncé :
  - version Unity (chaîne terminée par 0), plateforme i32, arbre de types présent (octet) ;
  - nombre de types i32, puis pour chaque type :
    - identifiant de classe i32 ;
    - octet « allégé » ;
    - index de script i16 ;
    - 16 octets supplémentaires si l'identifiant de classe vaut 114 ;
    - 16 octets d'empreinte ;
    - si l'arbre de types est présent : nombre de nœuds i32 et taille du tampon de chaînes i32, puis les nœuds (32 octets chacun pour une version au moins 19, 24 octets sinon), puis le tampon de chaînes ;
    - pour une version au moins 21 : nombre de dépendances i32, puis autant d'i32.
  - [Types mesurés : 49 (TextAsset) et 142 (AssetBundle).]
- **Table des objets** : nombre d'objets i32 [102], puis alignement sur 4 octets (position relative au début du fichier sérialisé). Pour chaque objet :
  - identifiant i64 ;
  - début u32 (i64 pour une version au moins 22), relatif au décalage des données ;
  - taille u32 ;
  - index de type i32, qui désigne un type de la liste ci-dessus.

### 4.4 Objet TextAsset (classe 49)

Les données de l'objet commencent par :
1. **m_Name** : longueur i32 puis autant d'octets UTF-8, puis alignement sur 4 octets relatif au début de l'objet [`Pinball`, `Blaster Master (U)`, `pacman`…] ;
2. **m_Script** : longueur i32 puis autant d'octets. **Ces octets sont exactement un profil**, au format de SPEC-03, identique à un fichier `.3dn`.

L'objet AssetBundle (classe 142) peut être ignoré. Les autres classes éventuelles sont ignorées.

## 5. Ce que fait l'import

- **Pour chaque TextAsset** :
  - le contenu de m_Script est vérifié avec le décodeur de profils de SPEC-03 ; un profil qui ne se décode pas est compté en échec, sans arrêter l'import ;
  - un profil valide est écrit **sans aucune modification** dans le dossier des profils importés de l'utilisateur (SPEC-09), sous le nom `<m_Name>.3dn`, où les caractères interdits par le système de fichiers sont remplacés par `_`.
- **Liste des jeux** : le titre et l'identifiant affichés dans la bibliothèque sont ceux que contient le profil lui-même (SPEC-03). Le lien entre une ROM et un profil importé suit les règles existantes de résolution de profil.
- **Fichiers externes** : le bundle n'est jamais modifié, déplacé ni copié tel quel. Seuls les profils extraits sont écrits.
- **Robustesse** : un fichier tronqué, une signature inconnue, une compression non prise en charge ou une table incohérente produit un message d'erreur clair, jamais un plantage, et n'écrit aucun fichier partiel.

## 6. Critères d'acceptation

- **Sur une installation réelle de 3dSen** (test optionnel, ignoré si l'installation est absente) : 101 profils importés ; chaque fichier produit est identique, octet pour octet, au profil de même contenu de l'inventaire de référence (empreintes SHA-256 de `assets-neutral/inventory.json`, type `Profile`).
- **Un second import** de la même installation n'ajoute rien : 101 déjà présents.
- **Sans installation réelle** :
  - des bundles synthétiques construits par les tests (sans compression, en LZ4, avec un ou plusieurs blocs, bloc d'information au début ou à la fin, ordre petit- ou gros-boutiste du fichier sérialisé) donnent les TextAssets attendus ;
  - des bundles corrompus (tronqués, mauvaise signature, compression LZMA, décalages hors limites) donnent une erreur claire.
- **Licences** : aucun fichier de 3dSen n'entre dans le dépôt ni dans la version publiée (REQ-PUB-002 reste vraie).
