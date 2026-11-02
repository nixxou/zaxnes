# AMD-051 — Robustesse de la conversion, du chargement et des écritures de réglages

Version : 1.1 | 2026-10-31 | **Normatif** (R1, choix du propriétaire du 2026-10-31 : reprise des pistes du rapport de comparaison avec 3dSen 2.0, §7.2, 7.3, 7.5 et §2). Précise SPEC-04B REQ-E3D-564 (« le comportement au-delà n'est pas spécifié ») et REQ-E3D-601 (objets pas encore prêts) ; **amende SPEC-07 §4** (un message ajouté) ; **étend le périmètre de l'écriture atomique de SPEC-09 REQ-PERS-021** (par REQ-PERS-088, REQ-PERS-021 restant inchangée) ; ajoute une option au harnais.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-050-053 : 51-1 à 51-12, tous retenus ; **REQ-E3D-645 retirée** au titre de 51-1, son numéro restant attribué avec la mention « retirée » ; REQ-E3D-644 et REQ-E3D-646 réécrites par leurs effets, sans « file d'envoi au GPU » (51-2, 51-11) ; texte exact et état cible de la notification de REQ-LOOP-073 (51-5)). ; 1.2 (R2-review-AMD-050-053-v1.1 : 51-N1, seconde branche restreinte aux étapes sans mesure d'avancement).
Priorités : les quatre exigences sont indépendantes les unes des autres ; si l'une n'est pas retenue, les autres restent valables. Aucune ne lève une exigence en vigueur : REQ-E3D-524, REQ-E3D-601 et REQ-E3D-603 continuent de s'appliquer intégralement.

## 1. Besoin

3dSen 2.0 remplace plusieurs pannes franches par des dégradations contrôlées, observables comme
suit : les tuiles d'un pattern dont la texture ne peut pas être allouée ne sont simplement pas
dessinées (le reste de la scène l'est) ; une tuile apparaît dès que son maillage et sa texture sont
disponibles et n'est jamais perdue ; un chargement qui n'avance plus est signalé à l'utilisateur au
lieu de figer le programme ; et un fichier persistant ne peut plus être laissé à moitié écrit. Ces
règles n'ont pas d'effet visible tant que tout va bien ; elles évitent un plantage ou une scène
corrompue sinon.

## 2. Exigences

**REQ-E3D-644** (P2) — **Texture d'un objet non allouable.** (Précise la phrase « Le comportement
au-delà n'est pas spécifié mais NE DOIT PAS interrompre l'émulation » de REQ-E3D-564, sans rouvrir
REQ-E3D-603 : cette exigence n'autorise pas à plafonner le nombre d'objets ou de tuiles dessinés par
frame.) Quand la texture d'un objet 3D d'un pattern ne peut pas être allouée — capacités de
REQ-E3D-564 dépassées, ou échec d'allocation côté pilote —, le moteur NE DOIT PAS lever d'exception
ni interrompre la frame : les tuiles de ce pattern ne sont pas dessinées tant que la texture n'est
pas disponible, les autres patterns sont dessinés normalement, et la frame suivante est produite.
Une erreur est journalisée au plus une fois par pattern et au plus 100 fois par partie ; au-delà,
une ligne de résumé périodique (au plus une par minute de jeu) donne le nombre cumulé de patterns
concernés.
- Critère d'acceptation (auto) : avec un budget total d'atlas réduit à C texels par le harnais
  (`Zax.Harness`, option `--atlas-budget <texels>`), le chargement d'un profil dont les objets
  demandent plus de C texels rend les objets logés, journalise une erreur par objet non logé, et la
  frame suivante est produite ; aucune exception n'est levée et l'émulation continue.
- Notes : la capacité de l'atlas est exprimée en **texels** (REQ-E3D-564 : jusqu'à 512 × 512 texels
  par objet, au moins 64 × 512 × 512 texels au total pour le profil chargé), parce qu'un pattern
  porte un nombre variable d'objets de tailles variables ; « N patterns » ne serait pas une grandeur
  mesurable. Le plafond de 100 lignes de journal par partie est nécessaire parce que les patterns
  sont aussi créés **pendant** le jeu (patterns temporaires, REQ-E3D-008, REQ-E3D-453) et que rien
  ne borne leur nombre sur une partie : une règle « une fois par pattern » seule produirait des
  milliers de lignes sur une ROM pathologique, ce qui est en soi une dégradation.

**REQ-E3D-645** — **retirée** (SPEC-GUIDE : un identifiant attribué n'est jamais réutilisé). La
v1.0 exigeait d'abandonner un maillage de tuile de plus de 65 536 sommets. Elle contredisait
frontalement REQ-E3D-524 (P1) — « Le moteur DOIT accepter des maillages de tuile de plus de 65 535
sommets (index 32 bits ou découpage) », critère : damier 3D de 8 × 8 × 96, 18 432 rectangles
(73 728 sommets), **tous présents** — et sa clause de sortie (« une implémentation à indices 32 bits
n'a pas cette limite ») la rendait vide pour toute implémentation conforme. La garantie voulue est
déjà portée par REQ-E3D-524, dont les Notes classent en **D** les faces erronées que le dépassement
produit dans l'original : aucun maillage de tuile, quelle que soit sa taille, ne doit lever
d'exception ni produire de géométrie corrompue ; une implémentation à indices de 16 bits découpe le
maillage. Le vecteur du damier de REQ-E3D-524 reste le seul critère.

**REQ-E3D-646** (P2) — **Objet pas encore prêt : pas oublié, et diagnostic.** Qu'un objet 3D dont
les maillages ne sont pas prêts ne soit pas dessiné et que sa shape reste reconnue est déjà exigé
par REQ-E3D-601 (P1), qui fait foi pour la mise en attente. S'y ajoutent deux clauses :
- un tel objet DOIT être dessiné à la **première frame rendue** qui suit la disponibilité de ses
  maillages et de sa texture, **sans être oublié** ;
- si un objet reste en attente pendant **600 frames émulées** consécutives, un diagnostic DOIT être
  journalisé, au plus une fois par objet (et, au-delà de 100 lignes par partie, sous la forme de
  résumé de REQ-E3D-644).
- Critère d'acceptation (auto) : un objet dont la construction est retardée de 10 frames par le
  harnais est dessiné à la première frame rendue qui suit la fin de sa construction, et aucun
  diagnostic n'est journalisé ; un objet dont la construction est retardée de 700 frames produit
  exactement une ligne de diagnostic, à la 600ᵉ frame ± 1.
- Notes : le retard de 10 frames est un **cas de test**, pas un comportement visé : REQ-E3D-601
  recommande (DEVRAIT) des maillages prêts au plus tard 2 frames émulées après la création d'un
  pattern. Justification de la constante : 600 frames émulées valent 10 s en NTSC (60 images/s,
  SPEC-07 §2), même ordre de grandeur que les 10 s de REQ-LOOP-073.

**REQ-LOOP-073** (P2) — **Chargement bloqué.** Pendant l'indicateur Chargement (REQ-LOOP-001), si
l'avancement exposé à l'indicateur de chargement (REQ-E3D-600) ne change pas pendant **10 s**, ou si
une étape de REQ-LOOP-021 (ou de REQ-LOOP-044) **qui n'expose aucune mesure d'avancement** (donc
autre que l'attente des maillages) ne se termine pas dans les 10 s, le programme DOIT :
1. afficher la notification d'erreur `Loading timed out.` (gravité normale, 4 s, SPEC-10
   REQ-UI-080/081 ; traduction française « Délai de chargement dépassé. », AMD-035/AMD-036) **(D)** ;
2. effacer le jeu courant, remettre l'indicateur Chargement à faux et passer à l'état **Arrêté**,
   comme REQ-LOOP-023 le fait pour un échec de lancement — l'écran d'accueil de AMD-010 REQ-UI-207
   est alors affiché, aucune ROM n'étant chargée.
- Critère d'acceptation (auto) : le harnais bloque l'étape 6 de REQ-LOOP-021 (attente des
  maillages) → la notification apparaît au bout de 10 s ± 1 s.
- Critère d'acceptation (auto) : après la notification, l'état est Arrêté, l'indicateur Chargement
  est faux, et une commande du modèle de commandes (AMD-010 REQ-UI-205) s'exécute dans la seconde
  qui suit.
- Notes : la durée totale d'une construction de masse n'est pas bornée (REQ-ACC-031) : tant que
  l'avancement change, le chargement n'est jamais interrompu. « n'avance plus » est défini par la seule mesure d'avancement observable de la spec
  (REQ-E3D-600), celle que REQ-LOOP-021 point 6 impose de consulter « au moins toutes les 100 ms ».
  Le message est une divergence **(D)** : le programme d'origine n'a pas de message pour ce cas ;
  il entre dans la table des textes d'AMD-035 comme entrée anglaise, avec sa traduction française
  (AMD-036), et s'ajoute à la table des messages de SPEC-07 §4.

**REQ-PERS-088** (P1) — **Périmètre de l'écriture atomique.** L'écriture atomique de REQ-PERS-021
(« écriture d'un fichier temporaire dans le même dossier, puis remplacement ») DOIT s'appliquer de
la même façon à **tout fichier persistant au sens de REQ-PERS-024** — réglages, bibliothèque, index
des slots, bindings — **et** aux fichiers de savestate (REQ-PERS-056). Une interruption (arrêt
brutal, coupure) laisse donc soit l'ancien fichier intact, soit le nouveau complet, jamais un
fichier tronqué. La copie `<nom>.v<N>.bak` de REQ-PERS-024 DOIT être écrite avant la première
réécriture migrée, et le renommage `*.corrupt-<horodatage>` de REQ-PERS-022 et REQ-PERS-037 DOIT
avoir lieu avant toute écriture du fichier de remplacement : ces deux opérations portent sur
l'ancien fichier et ne sont jamais la cible du remplacement atomique.
- Critère d'acceptation (auto) : pour le fichier de bibliothèque, l'index des slots, le fichier de
  bindings et un fichier de savestate, une écriture interrompue après la moitié des octets laisse
  l'ancien fichier lisible et inchangé, et aucun fichier temporaire n'est pris pour le fichier final
  au démarrage suivant.
- Notes : le fichier de réglages est déjà couvert par REQ-PERS-021 (P1), avec son propre critère
  (« tuer le processus pendant une écriture → le fichier reste lisible ») ; il n'est pas réénoncé
  ici, pour qu'il n'existe qu'une source normative par comportement. L'index des slots est le
  fichier dont la corruption coûte le plus cher (REQ-PERS-055 : « à défaut, tous les slots sont
  vides »), et c'est précisément lui que la v1.0 omettait.

## 3. Changements documentaires

1. SPEC-04B §3.6 : REQ-E3D-644 ajoutée après REQ-E3D-564 (dont elle précise la dernière phrase).
2. SPEC-04B §3.4 REQ-E3D-524 : inchangée. Son numéro et son critère restent la seule référence pour
   les maillages de plus de 65 535 sommets.
3. SPEC-04B §3.7 : REQ-E3D-646 ajoutée après REQ-E3D-601 ; REQ-E3D-645 inscrite comme **retirée**.
4. SPEC-07 §3.3 : REQ-LOOP-073 ajoutée après REQ-LOOP-024.
5. SPEC-07 §4, table des messages : ligne ajoutée
   « | chargement bloqué (REQ-LOOP-073) | `Loading timed out.` | erreur | 4 s | ».
6. AMD-035, table des textes (anglais par défaut / français) : `Loading timed out.` / « Délai de
   chargement dépassé. ».
7. SPEC-09 §3.2 : REQ-PERS-088 ajoutée après REQ-PERS-024 ; REQ-PERS-021 est inchangée, son
   périmètre étant étendu par REQ-PERS-088.
8. Harnais (PLAN, `Zax.Harness`) : option `--atlas-budget <texels>` ajoutée à la liste des moyens de
   vérification, pour le critère de REQ-E3D-644.
9. SPEC-v1.2-INDEX : REQ-E3D-644 et REQ-E3D-646 ajoutées à la liste de SPEC-04B, REQ-E3D-645 avec
   la mention « retirée », REQ-LOOP-073 à celle de SPEC-07, REQ-PERS-088 à celle de SPEC-09.

## 4. Questions

- Aucune.
