# AMD-033 — Dossier `profiles` à côté de l'exécutable

Version : 1.0 | 2026-10-23 | **Normatif** (R0, demande de l'utilisateur). Complète SPEC-09 REQ-PERS-040 et REQ-PERS-071.

## 1. Règle

- En plus du dossier de profils de l'utilisateur (`<racine utilisateur>/profiles`, REQ-PERS-040), Zaxnes parcourt **toujours** un second dossier : **`profiles` à côté de l'exécutable**.
- Ce dossier est parcouru **dans tous les modes** : normal, portable (`portable.txt`) et avec `--userdata`. En mode portable, il s'ajoute à `userdata/profiles`, qui reste le dossier de l'utilisateur.
- Le dossier de l'exécutable est **en lecture seule** pour Zaxnes : rien n'y est jamais écrit, déplacé ni supprimé. L'import (fichier `.3dn` ou depuis 3dSen, AMD-030) écrit toujours dans le dossier de l'utilisateur. Le programme doit fonctionner même si ce dossier n'est pas accessible en écriture, par exemple sous `Program Files`.
- Si le dossier n'existe pas, il n'y a ni erreur ni création automatique. Seule la version publiée le fournit (§3).

## 2. Fusion des deux dossiers

- La bibliothèque montre les profils **des deux dossiers**, avec les mêmes règles de lecture et de rejet des fichiers invalides (REQ-PERS-040 et suivantes) et une rescanne dans les mêmes cas.
- **Même nom de fichier** : si le dossier de l'utilisateur contient un fichier du même nom (comparaison sans tenir compte de la casse, sous-dossiers relatifs compris), **seul celui de l'utilisateur est pris en compte**. Celui du dossier du programme est ignoré, sans même être lu.
- Deux fichiers au contenu **identique, octet pour octet**, n'apparaissent qu'une fois.
- Deux fichiers différents qui désignent **le même profil** (même identifiant de profil, SPEC-03) : celui du **dossier de l'utilisateur l'emporte**, et l'autre est signalé une fois dans le journal.
- L'origine du profil (« utilisateur » ou « dossier du programme ») est visible dans l'interface, là où le dossier source d'un profil est déjà affiché, sinon en info-bulle.

## 3. Version publiée

- `publish.ps1` crée un dossier `profiles/` vide à côté de `Zaxnes.exe`, avec un court `README.txt` : « Déposez ici vos profils .3dn ; ils sont chargés au démarrage. Aucun profil n'est fourni. »
- Ce dossier ne contient aucun profil, et REQ-PUB-002 reste vraie.

## 4. Critères d'acceptation

- Un `.3dn` déposé dans `profiles` à côté de l'exécutable apparaît dans la bibliothèque, en mode normal, en mode portable et avec `--userdata`.
- Les règles du §2 sont vérifiées : à nom de fichier égal, celui de l'utilisateur seul ; à contenu identique, affiché une fois ; à identifiant égal, la version de l'utilisateur gagne.
- Le programme démarre normalement si ce dossier est absent ou en lecture seule.
- L'import écrit toujours dans le dossier de l'utilisateur.
