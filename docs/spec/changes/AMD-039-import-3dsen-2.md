# AMD-039 — Import des profils de 3dSen 2.0 (bundle UnityFS format 8)

Version : 1.1 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Amende AMD-030 §3, §4 et §6.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-039-045 : 39-1 à 39-5).

## 1. Constat

3dSen 2.0 (moteur Unity 6000.3.21f1, Windows 64 bits) range ses profils intégrés dans un bundle
dont l'en-tête diffère de celui qu'AMD-030 a mesuré sur la 1.1.2 (Unity 2019.4.41f2). Mesuré
sur une installation réelle de la 2.0 : l'import AMD-030 échoue, avec une erreur de
décompression LZ4, alors que le même import réussit sur la 1.1.2 (101 profils).

Les profils eux-mêmes ne changent pas de format : ce sont toujours exactement des fichiers
`.3dn` (SPEC-03). La 2.0 en contient 101, sous les mêmes noms qu'en 1.1.2.

## 2. Ce qui change dans le format du bundle (format Unity public)

Valeurs mesurées sur la 2.0, entre crochets :

| Champ | 1.1.2 | 2.0 |
|---|---|---|
| version de format | 7 | [8] |
| version du moteur | `2019.4.41f2` | [`6000.3.21f1`] |
| drapeaux de l'en-tête | 0x43 | [0x243] |
| taille **compressée** du bloc d'information (champ de l'en-tête, AMD-030 §4.1) | 1438 | [1429] |
| plateforme du fichier sérialisé | 5 | [19] |
| version du fichier sérialisé | 21 | [22] |
| dossier de plateforme | `StandaloneWindows` | [`StandaloneWindows64`] |

- **Bit 0x200 des drapeaux de l'en-tête** (nouveau) : quand il est présent et que le bloc
  d'information suit l'en-tête (bit 0x80 absent), le flux des blocs de données commence à la
  **position suivante alignée sur 16 octets** (depuis le début du fichier) après la fin du bloc
  d'information, c'est-à-dire après ses octets **compressés**. Mesure : en-tête fini à 50, bloc
  d'information lu à 64 (alignement de format ≥ 7, déjà dans AMD-030), long de 1429 octets
  compressés, donc fini à 1493 ; premier bloc de données à **1504**.
- Avec le bit 0x80 (bloc d'information à la fin du fichier), le bit 0x200 est **sans effet** :
  les blocs de données suivent l'en-tête aligné (AMD-030 §4.2), position déjà multiple de 16.
- AMD-030 §4.1 : accepter les versions de format **6 à 8** (inchangé) ; les autres bits de
  drapeaux restent sans effet sur la lecture.
- AMD-030 §4.3 : la version 22 du fichier sérialisé (champs 64 bits) est déjà décrite ; elle
  est désormais rencontrée en pratique. La plateforme est ignorée.
- AMD-030 §3 : le **motif de chemin est inchangé** en 2.0. Mesuré :
  `3dSen_Data/StreamingAssets/aa/StandaloneWindows64/defaultprofiles_assets_all_<empreinte>.bundle`.

## 3. Ce qui ne change pas

- La structure du bloc d'information, des blocs de données, du fichier sérialisé et de l'objet
  TextAsset (AMD-030 §4.2 à §4.4).
- Les règles d'import (AMD-030 §5), de doublons et de noms, et AMD-037.
- Une installation 1.1.2 s'importe comme avant.

## 4. Critères d'acceptation (amendent AMD-030 §6)

- **Sans installation réelle** :
  - un bundle synthétique en format 8, drapeaux 0x243 (LZ4HC, liste des fichiers, bit 0x200),
    dont le bloc d'information compressé ne finit pas sur un multiple de 16, donne les
    TextAssets attendus ;
  - le même bundle dont on retire le bit 0x200 de l'en-tête (les octets de bourrage restent
    devant les données) **n'importe aucun profil et ne plante pas** (AMD-030 §5) ;
  - un bundle en format 8 avec les bits 0x80 et 0x200, dont les données suivent l'en-tête aligné
    sans bourrage supplémentaire, donne les TextAssets attendus.
- **Sur une installation réelle de 3dSen 2.0** (test facultatif) : 101 profils importés, 0 en
  échec ; un second import donne 101 déjà présents. Les empreintes ne sont pas comparées à
  l'inventaire de référence, qui décrit la 1.1.2 (§5).
- **Sur une installation réelle de 3dSen 1.1.2** (test facultatif) : inchangé, y compris la
  comparaison octet pour octet avec l'inventaire de référence (premier critère d'AMD-030 §6).

## 5. Données de référence

Dans la 2.0, 97 des 101 profils sont identiques octet pour octet à ceux de la 1.1.2. Les quatre
qui diffèrent (`DQ3_DW3`, `Final_Fantasy_USA`, `Super Mario Bros`, `punchout`) ne changent que
par leurs scripts Lua (voir AMD-040 §1).
