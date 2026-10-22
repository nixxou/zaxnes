# AMD-025 — Boîtier TV : un modèle sous CC BY 4.0

Version : 1.1 | 2026-10-21 | **Normatif** (remplace REQ-PUB-004 d'AMD-024 pour le boîtier TV ; décision de l'utilisateur)

- Le boîtier TV propose **un seul modèle** (plus « aucun »). Le réglage TV devient : 0 aucun, 1 TV. Toute valeur héritée 2 ou 3 est ramenée à 1. Les trois modèles procéduraux de P7a sont retirés.
- **Modèle** : « Retro Crt Tv », par **meipal** (https://sketchfab.com/ashabbugaev12), licence **CC BY 4.0**, source https://sketchfab.com/3d-models/retro-crt-tv-d3025abdf2b64882bf60f207b2042a5c. Fichier fourni par l'utilisateur : `assets-free/tv/retro-crt-tv/retro_crt_tv.glb` (glTF 2.0 binaire, 4 maillages, 3 matériaux PBR, 6 textures : 5 PNG et 1 JPEG). Aucune mention NoAI : le fichier peut être lu par les agents pour l'intégration.
- **Le fichier GLB est livré tel quel**, sans modification. Les adaptations sont faites à l'exécution, et `ASSETS-LICENSES.md` les décrit (attribution CC BY : auteur, titre, lien, licence, liste des adaptations).
- **Adaptations** :
  - **l'écran n'est pas dessiné** : les deux plaques superposées de l'écran (la vitre à couleur unie et l'image texturée, les maillages de 49 sommets) sont masquées ; l'image du jeu NES apparaît à leur place. Option P3 : garder la vitre en léger reflet transparent ;
  - **échelle, orientation et position** calculées pour que l'ouverture de l'écran encadre exactement le rectangle visible de SPEC-06, avec le tunnel de SPEC-06 derrière ;
  - **ombres** : le caisson ne projette ni ne reçoit d'ombre ; les boutons projettent et reçoivent (AMD-021).
- **Rendu** : chargeur glTF 2.0 binaire générique (maillages, matériaux PBR métal-rugosité, normal map, émission), décodeurs PNG et JPEG de base écrits d'après les normes publiques (pas de nouvelle dépendance réseau).
- L'écart d'aspect avec les modèles TV de l'original est une divergence **D**.
