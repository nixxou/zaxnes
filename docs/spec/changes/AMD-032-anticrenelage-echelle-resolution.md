# AMD-032 — Anticrénelage, échelle de résolution, et deux corrections de démarrage

Version : 1.0 | 2026-10-23 | **Normatif** (lot FIX-4, demande de l'utilisateur). Complète SPEC-06 REQ-SCN-063, SPEC-09 REQ-PERS-017 et REQ-PERS-067, AMD-010 REQ-UI-207/209.

## 1. Objet

Trois réglages vidéo de SPEC-09 étaient **stockés, lus depuis les profils et exposés à Lua, sans
que le rendu ne s'en serve** : `video.anti_alias`, `video.res_scale` et `video.render_method`.
L'original lui-même force son anticrénelage à « désactivé », de sorte que son réglage n'a pas
d'effet visible non plus (REQ-PERS-017 l'enregistre à ce titre). Le présent amendement les rend
effectifs dans la réimplémentation, et corrige deux défauts de démarrage signalés par
l'utilisateur.

Les §2 et §3 sont des **divergences de type D** (améliorations assumées par rapport à l'original),
autorisées respectivement par REQ-PERS-017 (« elle **PEUT** appliquer un anticrénelage plein
écran ») et par REQ-SCN-063 (« la réimplémentation **PEUT** proposer un rendu à résolution
réduite […] ; sa prise en compte par le rendu est facultative »). Les §5 et §6 sont des
corrections de régression.

## 2. Anticrénelage — `video.anti_alias` (divergence D)

| Valeur | Libellé SPEC-09 | Effet dans Zaxnes |
|---|---|---|
| 0 | `None` | aucun |
| 1 | `FFAA` | **MSAA 4×** |
| 2 | `SMAA` | **MSAA 8×** |

- Les libellés et le domaine de SPEC-09 §4.1.1 sont **inchangés** : la persistance, l'aller-retour
  des profils (champ 32), la table libretro et l'API Lua ne bougent pas.
- REQ-PERS-017 nomme FXAA et SMAA. Zaxnes prend du **multi-échantillonnage** à la place : la scène
  est faite de faces plates à arêtes franches texturées au plus proche voisin (REQ-SCN-034), et un
  filtre de post-traitement flouterait précisément le pixel art qu'il s'agit de garder net, alors
  que le multi-échantillonnage ne touche que les arêtes géométriques. C'est la divergence.
- La cible multi-échantillonnée est une copie du tampon linéaire d'AMD-021 R2-009 (RGBA16F +
  profondeur), **résolue avant que quoi que ce soit n'échantillonne la texture** — donc avant la
  passe d'encodage sRGB. Le mélange de REQ-SCN-040 continue de se faire en linéaire.
- **La carte d'ombre n'est pas multi-échantillonnée** et ne change pas (REQ-SCN-030, DEC-R2-D4).
- Le nombre d'échantillons demandé est ramené à la plus grande puissance de deux que
  `GL_MAX_SAMPLES` autorise, puis divisé par deux tant que le framebuffer n'est pas complet, et
  en dernier ressort à 1 : une machine qui ne sait pas faire 8× rend en 4×, et une qui ne sait
  rien faire rend sans anticrénelage, sans erreur. Aucune extension n'est requise
  (REQ-SCN-056 : tout ceci est du GL 3.0 core).
- **Effet immédiat, sans redémarrage** : le réglage est appliqué à la frame rendue suivante, le
  rendu réallouant sa cible de lui-même. C'est l'effet « imm. » que SPEC-09 §4.1.1 lui donne.

Critères d'acceptation :

- `Zax.Harness render --anti-alias 0|1|2` rapporte respectivement 1, 4 et 8 échantillons accordés,
  et le nombre de couleurs distinctes de la capture croît strictement de 0 à 1.
- Changer le réglage entre deux frames, sur un même contexte, suffit : aucune réouverture de
  fenêtre.

## 3. Échelle de résolution — `video.res_scale` et `video.render_method` (divergence D)

- `video.render_method` = **0 (« Direct »)** : la scène est rendue à la résolution native de la
  vue. `video.res_scale` n'a **aucun effet**, et la fenêtre d'options affiche la ligne **grisée**
  avec la mention « sans effet en rendu direct : résolution native ». C'est le comportement de
  l'original, qui n'offre que cette méthode sous Windows.
- `video.render_method` = **1 (« Render Tex »)** : la scène est rendue dans un tampon de taille
  vue × facteur, puis **agrandi en filtrage bilinéaire**. Les facteurs sont ceux de REQ-SCN-063 et
  de REQ-PERS-013 point 6 : 1, 2/3, 1/2, 1/3, 1/4 pour les indices 0 à 4. L'arrondi est au plus
  proche, de sorte que 2/3 de 1080 donne 720.
- Le tampon ne descend jamais sous 1 × 1 pixel.
- Effet immédiat dans les deux sens, sans redémarrage.

Critères d'acceptation : pour une vue de 1920 × 1080 et la méthode 1, les cinq indices donnent
1920×1080, 1280×720, 960×540, 640×360 et 480×270 ; avec la méthode 0, les cinq donnent 1920×1080.

## 4. Ombrage — rien ne change

La note fonctionnelle du lot proposait de porter l'ambiante à « 0,3 ». La mesure des captures de
référence 1920 × 1080 montre que l'ambiante effective de l'original est **0,183** en linéaire,
soit la valeur de REQ-SCN-027 (0,1812) à 1 % près, et que Zaxnes produit déjà 0,179.
**REQ-SCN-027 est donc conservée telle quelle.** Le détail de la mesure, la cause réelle de
l'écart de lisibilité (l'ombre portée des glyphes extrudés, perdue par la carte d'ombre de 1024
texels) et la proposition qui en découle sont dans **Q-072**.

## 5. Console parente — régression du lot P8

L'exécutable est de sous-système GUI. Il ne **DOIT** emprunter la console du terminal qui l'a
lancé que pour les options qui écrivent et s'arrêtent sans jamais ouvrir de fenêtre :
`--help`, `--list-profiles` et `--import-3dsen` (AMD-030 §2), plus l'erreur d'usage, qui est du
même ordre. Un lancement normal **NE DOIT PAS** emprunter la console : sinon la fenêtre partage
le terminal, les lignes de journal s'écrivent par-dessus l'invite et un Ctrl+C destiné au shell
tue l'émulateur.

La vérification du descripteur standard déjà redirigé et la sortie en UTF-8 sont conservées pour
ces options.

Critère d'acceptation : la décision est une fonction pure du résultat de l'analyse de la ligne de
commande, testée option par option.

## 6. Plein écran au démarrage — état de fenêtre et état d'interface

- **REQ-PERS-067** est appliquée **sur la fenêtre** : quand les réglages persistés portent
  `ui.fullscreen` vrai, la fenêtre est **créée** en plein écran. Auparavant seule l'interface
  passait en mode plein écran (barre de menus masquée) tandis que la fenêtre s'ouvrait en
  fenêtré, ce qui laissait l'utilisateur devant une fenêtre noire sans menu.
- Exception : `--smoke-test` et `--ui-shots` ouvrent toujours en fenêtré, leurs captures devant
  garder la taille demandée en ligne de commande.
- Le retour reste Alt+Entrée (REQ-UI-209) ; et sans ROM chargée, l'écran d'accueil de REQ-UI-207
  reste affiché même en plein écran, de sorte que le moyen d'en sortir est toujours visible
  (**Q-073**).

Critère d'acceptation : avec un dossier de données dont `settings.json` porte
`"ui.fullscreen": true`, le lancement donne une fenêtre réellement plein écran, l'écran d'accueil
visible s'il n'y a pas de jeu, et Alt+Entrée ramène la barre de menus.

## 7. Ce qui ne change pas

Persistance, domaines, libellés, champs de profil, table libretro et API Lua des trois réglages :
**inchangés**. Carte d'ombre : inchangée. Ambiante et ombrage : inchangés (§4).
