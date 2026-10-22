# AMD-020 — Réponses aux tickets du round 1, partie C

Version : 1.0 | 2026-10-21 | **Normatif**

| Ticket | Exigence | Amendement |
|---|---|---|
| Q-048 | SPEC-08 REQ-INP-03x | Le seuil numérique de 0,34 s'applique à la **course brute** de l'axe (valeur 0 à 1 lue sur le périphérique, avant zone morte et saturation). |
| Q-049 | SPEC-08 REQ-INP-106, AMD-010 REQ-UI-204 | AMD-010 et AMD-013 priment (décision postérieure) : raccourcis de menu de REQ-UI-204, chargement d'état par Ctrl+Fn ; les touches nues de la table §4.2 de SPEC-08 gardent leur rôle de jeu et de caméra. |
| Q-050 | SPEC-09 REQ-PERS-036, REQ-PERS-074 | Jaquettes : exigences ramenées en **P3** (D). Sans bibliothèque à tuiles (AMD-010), l'import et la capture de jaquettes ne sont pas exigés en v1 ; la sélection de profil PEUT afficher une jaquette si elle existe. |
| Q-051 | SPEC-10 REQ-UI-036, AMD-010 REQ-UI-201, AMD-013 | Le filtre du choix de ROM propose `.nes`, `.zip` et « Tous les fichiers » ; `.7z` n'est pas proposé (pas de décodeur, AMD-013). |
| Q-052 | SPEC-09 §4.1.10, SPEC-05 §4 | `EngineMode`, `IsProfileMode` et `IsRomMode` sont **requis** (lecture seule) en Lua, avec la sémantique de SPEC-09 §4.1.10 ; ils complètent les tables de SPEC-05 §4. |
| Q-053 | SPEC-08 REQ-INP-123 | Le critère de REQ-INP-123 suit la relation amendée de REQ-INP-063 (DEC-08-4 : pixel affiché, `py = tronc(248 − (y − Oy)/e)`) ; les valeurs attendues sont recalculées en conséquence. |
