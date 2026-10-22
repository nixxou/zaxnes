# Guide de rédaction et de lecture de la spécification

La spécification est **le seul pont** entre le côté sale (qui connaît le programme d'origine) et le côté propre (qui le reconstruit). Elle décrit **ce que** le programme fait, de façon observable et testable, jamais **comment** l'original est codé.

## Structure d'un document `SPEC-NN-<sujet>.md`

```
# SPEC-NN — <Titre>
Version : 1.0-draft | Statut : brouillon | relu R2 | gelé
Dépend de : SPEC-xx, SPEC-yy

## 1. Objet et périmètre
## 2. Concepts et vocabulaire (renvoie au glossaire de SPEC-00 si possible)
## 3. Exigences
### 3.1 <Thème>
**REQ-<MOD>-001** (P1) — <énoncé normatif, avec DOIT / DEVRAIT / PEUT>
- Critère d'acceptation : <test observable : entrée → sortie attendue, tolérance>
- Notes : <précisions, cas limites, justification d'une constante>
## 4. Formats et interfaces (si applicable)
## 5. Vecteurs de test (si applicable)
## 6. Questions ouvertes
```

- **Identifiants** : `REQ-<MOD>-NNN` avec MOD ∈ {CORE, PROF, E3D, LUA, SCN, LOOP, INP, PERS, UI, AUD, ARCH, ACC}. Un identifiant attribué n'est jamais réutilisé ; une exigence supprimée garde son numéro avec la mention « retirée ».
- **Priorités** : P1 bloquante (sans elle, un jeu ou un profil ne fonctionne pas), P2 importante (fonction visible), P3 confort.
- **Normatif** : DOIT = obligatoire, DEVRAIT = recommandé (un écart doit être justifié), PEUT = optionnel.
- Chaque exigence porte un **critère d'acceptation vérifiable** par un tiers qui ne connaît que la spec.

## Ce que la spec PEUT contenir
- Le comportement observable : entrées, sorties, états, transitions, invariants, valeurs par défaut, bornes, erreurs.
- Les **faits d'interopérabilité** nécessaires pour relire les données conservées : formats de fichiers (schéma protobuf avec numéros de champs, encodages), noms et signatures de l'API exposée aux scripts Lua des profils, clés de réglages persistés.
- Les constantes qui déterminent un résultat observable, avec leur justification (« la caméra par défaut est à N unités, parce que … »).
- Des vecteurs de test : entrée → résultat attendu.
- Des descriptions mathématiques d'un résultat (« le mesh d'une tile en mode X est un prisme de hauteur h, dont les faces … »).

- Des renvois au code public sous GPL v3 de `third_party/3DNes_Engine` (version antérieure du moteur, sous licence et réutilisable) : « même résultat que la méthode X de 3DNes, sauf … ». Ces renvois sont encouragés quand le comportement est identique, parce qu'ils réduisent les écarts.

## Ce que la spec NE DOIT PAS contenir
- Du code ou du pseudo-code qui suit la structure de l'original, ligne à ligne.
- Des noms internes de classes, méthodes, champs ou variables privés de l'original, sauf s'ils sont visibles depuis les données conservées (noms de champs Lua appelés par les profils, par exemple).
- L'architecture interne de l'original (découpage en classes, héritage, threads internes), sauf effet observable.
- Des références au code d'origine (« la méthode X fait… », numéros de ligne, noms de fichiers source).

## Liberté laissée aux implémenteurs
Tout ce que la spec ne fixe pas est un **choix d'implémentation libre** : structure du code, algorithmes, nommage, threads. Quand une ambiguïté bloque, l'implémenteur ouvre un ticket `spec/changes/Q-NNN.md` et ne devine pas en silence.

## Contraintes transverses (portabilité libretro, voir PLAN.md)
Toute exigence doit rester compatible avec : bibliothèques sans dépendance de plateforme pour le cœur, le format, le moteur 3D et le scripting ; un rendu dans une cible fournie par l'hôte ; aucun état global ; des savestates déterministes ; une couche d'entrées abstraite ; un modèle de réglages indépendant de l'UI.
