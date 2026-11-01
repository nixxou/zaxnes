# AMD-056 — Import d'un profil : textes et dialogues de 3dSen 2.0

Version : 1.0 | 2026-11-01 | **Normatif** (R1, choix du propriétaire du 2026-11-01 : aligner l'import d'un profil sur 3dSen 2.0, suite de Q-089 et de la vérification R5). **Amende SPEC-10 REQ-UI-084** (textes 2 et 3), **SPEC-09 REQ-PERS-041** (points 1, 3 et 4), **AMD-053 REQ-UI-212** (table des causes et succès) ; **remplace** la réponse de Q-089 sur la forme de `<v>` et sur le profil embarqué. Les entrées anglaises de la table des textes d'AMD-036 suivent ; leurs traductions françaises sont à refaire sur les nouveaux textes.
Priorité : P2, comme REQ-UI-212.

## 1. Constat

La lecture du programme de référence 2.0 (passe R5, 2026-11-01) montre trois écarts avec AMD-053 :

1. le texte de version refusée affiche la version **assainie** (seuls chiffres, `.` et `$` gardés, REQ-PROF-021), non la chaîne brute du fichier ;
2. le texte « fichier illisible » est corrigé : une espace après le chemin, `obsolete` au lieu de `obsolate` ;
3. l'import passe par des **boîtes de dialogue** et non par une notification : un dialogue d'erreur, un dialogue d'information pour un profil embarqué, une **confirmation** avant de remplacer un profil utilisateur ; un import réussi n'affiche **rien** et montre l'entrée importée.

AMD-053 reprenait les fonctions de 2.0 sans sa présentation. Le choix du propriétaire est d'en reprendre aussi ces textes et ces dialogues ; la **forme** des dialogues reste celle de l'interface de Zaxnes (AMD-010), libre.

## 2. Exigences

**REQ-UI-084** (amendée, P2) — Textes des causes de refus, entrées anglaises de la table des textes :
- `This file is not a 3dn file` (inchangé) ;
- `3dn format version <v>  is not supported`, deux espaces avant « is », où `<v>` est la version **assainie** au sens de REQ-PROF-021 (un fichier qui écrit `v1` donne `version 1`) ;
- `<chemin> has an obsolete or wrong profile format. This profile is not compatible with the current version of Zaxnes.` — une espace après le chemin complet du fichier, `obsolete`.

Ces textes servent aussi hors de l'import (chargement d'un profil, REQ-PERS-040) ; la gravité et la durée des notifications qui les portent en dehors de l'import restent celles de SPEC-10.

**REQ-UI-214** (P2) — **Dialogues d'import de profil** (remplace la notification de REQ-PERS-041 point 1 et le succès de REQ-UI-212). Après le choix d'un fichier par « Importer un profil… », le programme lit le fichier comme le prévoit REQ-UI-212 (SPEC-03, champs racine seulement), puis selon l'issue :

| Issue | Titre du dialogue | Corps | Boutons |
|---|---|---|---|
| Refus (une cause de la table de REQ-UI-212) | `Cannot import this file` | `<fichier>: <texte de la cause>` | `OK` |
| `Id` d'un profil **embarqué** | `Already in Zaxnes` | `<titre> is a built-in profile, so <fichier> was not imported.` | `OK` |
| `Id` d'un profil **utilisateur** existant | `Replace <nom>?` | `The profile will be replaced with the one in <fichier>. The ROM, cover and save states will be kept.` | `Replace` (action destructive) et `Cancel` |
| `Id` inconnu | aucun dialogue | — | — |

- `<fichier>` est le **nom** du fichier choisi, avec son extension, sans dossier ; `<texte de la cause>` est le texte de REQ-UI-084 correspondant, qui contient lui-même le chemin complet pour la troisième cause.
- `<titre>` est le `Title` du fichier importé, à défaut `<fichier>`.
- `<nom>` est le titre **affiché** de l'entrée de bibliothèque existante (titre modifié par l'utilisateur compris).
- `Replace` effectue le remplacement de REQ-PERS-041 point 3, puis se comporte comme un import réussi ; `Cancel`, ou la fermeture du dialogue, ne change rien.
- Si l'écriture échoue après la confirmation, le dialogue de refus s'affiche avec le message de l'erreur.
- Un import réussi (nouvel identifiant, ou remplacement confirmé) NE DOIT afficher **ni notification ni dialogue** ; l'entrée importée DEVRAIT devenir l'entrée sélectionnée de la boîte de sélection de jeu/profil de REQ-UI-201 (forme libre).
- Le titre et le texte des boutons sont des entrées de la table des textes (AMD-036) ; les textes ci-dessus en sont les entrées anglaises.
- Critère d'acceptation (auto) : importer un fichier texte quelconque `junk.3dn` → dialogue `Cannot import this file`, corps `junk.3dn: <chemin complet>/junk.3dn has an obsolete or wrong profile format. …`, bibliothèque inchangée.
- Critère d'acceptation (auto) : importer un profil dont `Ver` = `"v1"` → corps se terminant par `3dn format version 1  is not supported`.
- Critère d'acceptation (auto) : importer un fichier portant l'`Id` d'un profil embarqué de `Title` « Foo » → `Already in Zaxnes`, corps `Foo is a built-in profile, so x.3dn was not imported.` ; rien n'est copié.
- Critère d'acceptation (auto) : importer une nouvelle version d'un profil utilisateur existant → dialogue `Replace <nom>?` ; `Cancel` laisse fichier et entrée intacts ; `Replace` remplace le fichier et garde ROM, jaquette et slots (critère de REQ-PERS-041).
- Critère d'acceptation (auto) : importer un profil d'`Id` inconnu → aucune notification ni dialogue, une entrée de plus dans la bibliothèque.

**REQ-PERS-041** (amendée) — Les points 1 et 4 renvoient à REQ-UI-214 au lieu d'une notification et d'un « aucun changement » silencieux ; le point 3 n'est exécuté qu'après la confirmation de REQ-UI-214. Le fait reste journalisé dans tous les cas.

**REQ-UI-212** (amendée) — La table des causes et la règle « protobuf mal formé = troisième texte » restent ; seule la forme d'affichage change (REQ-UI-214). Le paragraphe « Un profil accepté DOIT afficher dans la notification d'import son titre et son auteur » est **supprimé**, ainsi que son critère d'acceptation.

## 3. Écarts assumés avec 3dSen 2.0

- **(D) Version refusée dès l'import.** La 2.0 ne lit pas le champ `Ver` à l'import : un profil `Ver` = `"1"` est copié dans la bibliothèque et n'échoue qu'au lancement du jeu. Zaxnes garde le refus à l'import de REQ-UI-212, avec le texte de la 2.0 : refuser tôt évite une entrée inutilisable.
- **(D) Protobuf mal formé.** La 2.0 affiche alors le message brut de sa bibliothèque protobuf, qui dépend de cette bibliothèque. Zaxnes garde le troisième texte de REQ-UI-084, comme REQ-UI-212.
- **Libellé « Zaxnes ».** `Already in 3dSen` devient `Already in Zaxnes`, comme AMD-029 l'a fait pour REQ-UI-084.
- La présentation des dialogues (taille, style, bouton destructif rouge, focus) reste libre (AMD-010 §4).

## Historique

- 1.0 (2026-11-01) : rédaction après Q-089 et la vérification R5 du code d'import de 3dSen 2.0.
