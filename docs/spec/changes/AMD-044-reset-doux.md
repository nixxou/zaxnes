# AMD-044 — Remise à zéro douce (bouton RESET de la console)

Version : 1.2 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Complète SPEC-02, SPEC-07 et SPEC-08 ; remplace la dernière phrase de REQ-LOOP-050 (« Il n'existe pas de reset doux… ») ; amende REQ-CORE-006. DEC-02-Q3 (« Reset = remise sous tension ») est **inchangée pour le reset dur**, et seulement complétée.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-039-045 : 44-1 à 44-7) ; 1.2 (seconde relecture R2-review-AMD-039-045-v1.1 : N-44-1 à N-44-3, T-1).
Priorités : Si une exigence P2 de cet amendement qui remplace une clause P1 n'est pas retenue, la clause d'origine reste valable (T-1).

## 1. Besoin

3dSen 2.0 distingue deux remises à zéro : le **reset dur** de SPEC-07 (comme couper et rallumer la
console) et un **reset doux**, qui imite l'appui sur le bouton RESET de la NES. Certains jeux s'en
servent : sauvegarde ou menu caché conservés en RAM, codes de triche, etc.

## 2. Exigences

**REQ-CORE-122** (P2) — **Reset doux du cœur** : le cœur DOIT offrir un reset doux conforme au
comportement documenté de la console (nesdev wiki, « CPU power up state » et « PPU power up
state », section *after reset*) :
- la RAM interne, la RAM de sauvegarde et la RAM de CHR sont **conservées** ;
- CPU : le drapeau I est mis à 1, S diminue de 3, PC est chargé depuis le vecteur de reset
  ($FFFC/$FFFD) ; A, X, Y et les autres drapeaux sont conservés ; aucune IRQ en attente ;
- PPU, APU et mapper suivent leur état documenté *after reset* (par exemple : PPUCTRL et PPUMASK
  à 0, canaux APU coupés, $4015 à 0) ;
- le verrou et les registres à décalage des ports de manette sont remis à zéro (choix de la
  réimplémentation, sans effet sur les boutons tenus par le joueur, relus à la frame suivante) ;
- le compteur de frames du cœur repart de 0 (**REQ-CORE-006 amendée** : il repart de 0 au
  chargement d'une ROM, au reset dur **et au reset doux**). Conséquences assumées, comme dans
  3dSen 2.0 : l'horloge des déformations animées (REQ-SCN-043) repart de 0, la cadence de
  capture du rollback repart avec le compteur, et les globales Lua `frame` et `time`
  (REQ-LUA-023, REQ-LUA-025) reviennent à 0 en cours de session, sans `End` ni `Start`.
- Critère d'acceptation (auto) : les ROM de test publiques `cpu_reset/ram_after_reset` et
  `cpu_reset/registers` (blargg), qui demandent un appui sur RESET, **passent** sous reset doux.
  La note de REQ-CORE-004 (`ram_after_reset` échoue volontairement) reste vraie pour le reset
  dur. Complément : une ROM maison qui compte ses resets en RAM affiche 3 après trois resets doux.
- **SPEC-02 §3.12 amendée** : `cpu_reset/ram_after_reset` et `cpu_reset/registers` s'ajoutent à la
  liste des ROM de test publiques exigées (REQ-CORE-110 à 116, REQ-ACC-001), sous reset doux.

**REQ-LOOP-072** (P2) [ROM] — **Commande** : le frontend DOIT proposer le reset doux à côté du reset dur
(menu de jeu, et l'action d'entrée de REQ-INP-125). Le reset doux se comporte comme le reset dur
de SPEC-07 pour le reste du programme (pause levée, horloge de cadence réinitialisée, historique de
rollback vidé, REQ-LOOP-031), mais NE DOIT PAS relancer le profil : pas de relecture de la ROM, pas
d'appel `End`/`Start` des scripts Lua, état appris de la conversion conservé.
- Critère d'acceptation (auto, harnais) : en jeu, reset doux → l'historique compte 0 snapshot ;
  les données `PData` ne sont pas remises à zéro ; aucun hook `End` ni `Start` n'est appelé.
- Critère d'acceptation [ROM] : « Super Mario Bros. » en cours de partie, reset doux → retour à
  l'écran titre sans écran de chargement.

**REQ-INP-125** (P3) — **Action d'entrée** : une action étendue « Reset doux » DEVRAIT exister dans
la table des actions étendues de SPEC-08 §4.3, avec un code libre (le suivant disponible), sans
association par défaut au clavier ni à la manette (comme dans 3dSen 2.0, où le reset doux passe
par le menu).
- Critère d'acceptation (auto) : l'action est associable dans l'écran des contrôles et déclenche
  REQ-LOOP-072.

## 3. Libellés (AMD-035 ; anglais par défaut / français)

| Élément | Anglais | Français |
|---|---|---|
| entrée de menu et action | `Soft Reset` | `Reset doux` |
| reset dur (libellé PEUT être précisé ainsi pour le distinguer ; sinon « Reset » inchangé) | `Hard Reset` | `Reset complet` |

## 4. Questions

- Aucune.
