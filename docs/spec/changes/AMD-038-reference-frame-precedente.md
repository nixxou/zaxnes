# AMD-038 — Référence conservée d'une frame à la suivante

Version : 1.0 | 2026-10-24 | **Normatif** (R0, constat de l'utilisateur). Précise SPEC-05 REQ-LUA-028 et corrige sa note.

## 1. Constat

- Profil « Zelda II - The Adventure of Link (U) », début de partie (palais du Nord) : le lit de Zelda et les deux torches apparaissent devant la marche supérieure au lieu d'être posés sur la plateforme, contrairement à 3dSen (captures de l'utilisateur, plusieurs angles de caméra).
- Cause : le script `pf` remplit sa liste `pfList` dans `LateUpdate` avec les shapes de la frame ; le script `char` lit cette liste dans le `UpdateS` de la frame **suivante** pour trouver la plateforme sous chaque personnage et ajoute son `Offset.z` (48 pour la marche supérieure). Le script lit donc des références conservées d'une frame à la suivante.
- La note de REQ-LUA-028 (« le corpus remet ses listes à zéro à chaque `Update`, il ne dépend pas de ce comportement ») est fausse pour ce profil. Une implémentation qui lit ces références comme vides (valeurs nulles) ne trouve aucune plateforme.
- La réimplémentation C# garde l'objet de chaque référence et lit ses dernières valeurs : son rendu de la scène est conforme. Le portage Rust lisait ces références comme vides.

## 2. Règle

- Une référence d'instance ou de shape obtenue pendant la frame N et lue pendant les hooks de la frame N+1 (`Update`, `UpdateS`, `LateUpdate`) DOIT donner les valeurs qu'avait l'objet à la fin de la frame N (frame publiée).
- L'écrire n'a aucun effet visible (inchangé).
- Au-delà, REQ-LUA-028 est inchangée : la lecture ne provoque ni plantage ni erreur Lua et DEVRAIT donner les valeurs de la dernière frame de l'objet. Une implémentation qui ne garde que la frame publiée peut rendre des valeurs nulles pour une référence plus ancienne, ou lue hors des hooks de la frame N+1.

## 3. Note de REQ-LUA-028

Remplacer la dernière phrase de la note par : « Le corpus en dépend au moins d'une frame à la suivante : « Zelda II - The Adventure of Link (U) » (scripts `pf` et `char`, AMD-038). »

## 4. Critères d'acceptation

- Test synthétique : un script garde `kept = shape` dans `UpdateS` à la frame 1 et lit `kept.sizeZ` dans `Update` à la frame 2 → la valeur de la frame 1 ; `kept.Alpha = 0.25` puis relecture → la valeur de la frame 1.
- « Zelda II », palais du Nord au début de la partie : le lit et les torches sont posés sur la plateforme supérieure (rendu identique à celui de la réimplémentation C#).
