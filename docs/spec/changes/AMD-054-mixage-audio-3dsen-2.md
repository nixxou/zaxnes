# AMD-054 — Mixage APU non linéaire et audio du MMC5 (3dSen 2.0)

Version : 1.1 | 2026-10-31 | **Normatif** (R1, choix du propriétaire du 2026-10-31 : reprise de la piste §8 du rapport de comparaison avec 3dSen 2.0, en suivant la 2.0). **Amende SPEC-02 REQ-CORE-073** (mixage ; son filtre de composante continue est conservé) **et REQ-CORE-075** (audio du MMC5) ; ajoute REQ-CORE-076 (point de test du mixeur) ; clôt Q-084 ; voir le §3 pour les documents mis à jour.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-054 : 54-1 à 54-18 ; 54-1 : la constante 95,88 est celle du programme d'origine (forme analytique publiée par nesdev), la table de consultation à 95,52 n'est pas retenue ; 54-6 : option (b), point de test du harnais ; 54-7, 54-8 et 54-13 : comportement du programme d'origine, avec les écarts à la documentation publique déclarés ; 54-9 : critère [ROM] retiré ; 54-11 non retenu, la polarité n'ayant pas d'effet audible sur un mixage par addition).
Priorités : REQ-CORE-073 amendée (P2) et REQ-CORE-076 (P2) sont indépendantes de REQ-CORE-075 amendée (P2) ; si REQ-CORE-075 amendée n'est pas retenue, `M = 0` et la règle d'origine de REQ-CORE-075 (audio d'extension facultatif) reste valable.

## 1. Besoin

La spec actuelle, écrite d'après 3dSen 1.x, prescrit l'approximation **linéaire** du mixage de
l'APU et n'exige aucun audio d'extension. 3dSen 2.0 mixe les cinq canaux selon la forme **non
linéaire** publiée par la documentation publique de la NES (nesdev, « APU Mixer », forme
analytique), et émule l'**audio du MMC5** (deux voies carrées et une voie PCM), que n'utilisent que
les cartouches Famicom (la NES ne transmet pas l'audio des cartouches). Q-084 relève en outre que la
note « non linéaire toléré » de REQ-CORE-073 est incompatible avec son propre critère.

## 2. Exigences

**REQ-CORE-073 amendée** (P2) — **Mixage non linéaire.** Avec `p1, p2` les sorties des deux
pulses (0 à 15), `t` celle du triangle (0 à 15), `n` celle du bruit (0 à 15) et `d` celle du DMC (0 à
127), et les fonctions réelles (nulles pour `i = 0`) :

- `P(i) = 95,88 / (8128 / i + 100)`, `i` de 0 à 30 ;
- `T(i) = 163,67 / (24329 / i + 100)`, `i` de 0 à 202 ;

et la constante réelle `K = 32 767 / (P(30) + T(202))` (calculée sur les valeurs non arrondies, en
double précision), le mixeur DOIT produire l'entier
`A = r(K · P(p1 + p2)) + r(K · T(3t + 2n + d))`, où `r` arrondit à l'entier le plus proche, **au pair
en cas d'égalité** ; on peut précalculer les deux tables d'entiers `r(K · P(i))` et `r(K · T(i))`.
L'entier `S = min(A + M, 32 767)`, où `M` est la sortie de l'audio d'extension (REQ-CORE-075, 0 sans
audio d'extension), DOIT ensuite être converti en flottant par `S / 32 768`, puis passer par un
filtre passe-haut qui coupe la composante continue (fréquence de coupure de 10 à 40 Hz), avant le
rééchantillonnage de REQ-CORE-070 et la sortie de REQ-CORE-071.
- Critère d'acceptation (auto, au point de test de REQ-CORE-076) : un pulse seul au volume 15 →
  `A` = 4 890 ; les deux pulses au volume 15 → 8 462 (rapport 1,73, au lieu de 2 en linéaire) ; le
  triangle seul à 15 → `r(K · T(45))` = 8 363 ; tous les canaux au maximum → 32 767.
- Critère d'acceptation (auto, sortie de REQ-CORE-071, protocole de Q-039 confirmé par AMD-018 :
  pulse à 440 Hz, timer 253 en NTSC, différence des médianes des deux plateaux, première moitié des
  échantillons écartée) : un pulse seul au volume 15, rapport cyclique 50 % → crête à crête
  0,149 ± 0,002 ; un silence prolongé tend vers 0 en moins de 0,2 s.
- Notes : la note « Le mixage non linéaire de nesdev est toléré si les niveaux relatifs restent à
  ±10 % » et le critère « 0,172 ± 0,02 » de la version précédente sont retirés ; le niveau global
  baisse d'environ 13 % par rapport à l'approximation linéaire (gain 1,526), comme dans 3dSen 2.0.
  Le numérateur 95,88 est celui de la forme analytique de nesdev ; la table de consultation de la
  même page utilise 95,52 et donnerait 4 876 / 8 438 / 8 371 : elle n'est pas retenue.

**REQ-CORE-076** (P2) — **Point de test du mixeur.** Le harnais (SPEC-12 §2, catégorie « auto »)
DOIT pouvoir lire, pour un instant donné de l'émulation, les entiers `A`, `M` et `S` de
REQ-CORE-073 avant toute conversion ou filtrage. Les critères en unités entières de cet amendement
portent sur ce point de test.
- Critère d'acceptation (auto) : silence (tous les canaux à 0) → `A = M = S = 0`.

**REQ-CORE-075 amendée** (P2) — **Audio du MMC5.** L'audio du MMC5 (mapper 5) DOIT être émulé :

- **deux voies carrées** ($5000–$5003 et $5004–$5007), reprenant la forme d'onde, le rapport
  cyclique, le timer, l'enveloppe, le compteur de longueur et la coupure des périodes inférieures à
  8 des pulses de l'APU, **sans balayage** ; leur enveloppe est cadencée à une fréquence **fixe
  d'environ 240 Hz** (tous les 7 458 cycles CPU) et leur compteur de longueur à **120 Hz** (une
  fois sur deux), indépendamment de `$4017` **(D sur la cadence du compteur de longueur, que la
  documentation publique donne à 240 Hz)** ;
- **une voie PCM** en mode écriture : une écriture non nulle en $5011 fixe le niveau `pcm` (0 à
  255) ; une écriture de 0 ne change pas le niveau et lève le drapeau d'IRQ PCM ; le mode lecture
  de $5010 (bit 0 = 1) n'est pas émulé (**D**) ;
- **$5010** : en écriture, bit 0 = mode (0 écriture, 1 lecture), bit 7 = autorisation de l'IRQ
  PCM ; en lecture, renvoie le drapeau d'IRQ PCM en bit 7 (les autres bits à 0) et **efface** le
  drapeau. Le drapeau **n'active pas** la ligne d'IRQ du CPU **(D)** ;
- **$5015** : bits 0 et 1 activent les deux voies carrées (une voie désactivée a son compteur de
  longueur mis à 0) ; en lecture, bits 0 et 1 = compteurs de longueur non nuls.

Sa sortie entière vaut `M = r(Kₘ · P(q1 + q2)) + 31 · pcm`, avec `q1, q2` les sorties des deux
voies carrées (0 à 15), `P` la fonction ci-dessus, `Kₘ = 8 000 / P(30)` et `r` l'arrondi de
REQ-CORE-073 ; elle s'ajoute à `A` et la somme est écrêtée à 32 767 (REQ-CORE-073). Les autres
audios d'extension (VRC6, VRC7, Namco 163, Sunsoft 5B, FDS) PEUVENT être émulés ; ils ne sont pas
exigés.
- Critère d'acceptation (auto, au point de test) : une voie carrée MMC5 seule au volume 15 →
  `M` = 4 623 ; les deux au volume 15 → 8 000 ; `pcm` à 128 → 3 968 ; une écriture de 0 en $5011
  laisse `pcm` inchangé et une lecture de $5010 renvoie alors $80, la suivante $00.
- Critère d'acceptation (auto) : une voie MMC5 à enveloppe décroissante (volume de départ 15, période
  d'enveloppe 0) passe de 15 à 0 en 15 × 1/240 s ± 5 %, que `$4017` vaille $00, $80 ou $C0.
- Notes : les constantes 8 000 (plein volume des deux voies carrées) et 31 (pas de la voie PCM) sont
  celles du programme d'origine ; elles placent la voie PCM sensiblement plus bas que la
  documentation publique. Aucun des 101 profils intégrés ne vise un jeu à audio MMC5.

## 3. Changements documentaires

1. SPEC-02 REQ-CORE-073 : corps, critère et notes remplacés par REQ-CORE-073 amendée ;
   REQ-CORE-075 : remplacée par REQ-CORE-075 amendée ; REQ-CORE-076 ajoutée.
2. SPEC-02 REQ-CORE-100 (liste des mappers, ligne du mapper 5) : « audio non émulé » devient
   « audio émulé (REQ-CORE-075) ».
3. SPEC-02 §6, question Q5, et DEC-02-Q5 (« l'audio d'extension n'est pas émulé ») : dépassées par
   REQ-CORE-075 amendée pour le MMC5.
4. Q-084 : close ; son §5 (choix provisoire du mixage linéaire) est dépassé.
5. SPEC-11 §1 et SPEC-v1.2-INDEX : le mixage est le mixage non linéaire de REQ-CORE-073 amendée,
   l'audio du MMC5 est émulé.

## 4. Questions

- Aucune.
