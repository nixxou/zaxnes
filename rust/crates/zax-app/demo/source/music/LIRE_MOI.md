# Saut de puce — musique pour jeu NES

Choix validé : proposition 1 « Saut de puce ». Ambiance joyeuse et rebondissante, accompagnement léger. Conserver cette mélodie et éviter de densifier l'arrangement.

## Fichiers
- Saut_de_puce.mp3 : référence sonore à écouter.
- Saut_de_puce.wav : référence PCM mono, 44 100 Hz, 16 bits.
- partition.json : partition structurée de tous les canaux, percussions incluses.
- notes.csv : les mêmes événements sous forme de tableau.
- regenerer_apercu.py : synthèse de référence autonome, Python 3 sans dépendance. Lancer depuis un dossier de travail ; crée outputs/ et work/.

## Structure et temps
136 BPM, 4/4, 8 mesures, 128 doubles croches, boucle de 14,117647 secondes. Une unité start_step vaut une double croche, soit 60/136/4 seconde. MIDI 60 = do central (C4). Tous les débuts sont absolus depuis le début de la boucle. Les durées des notes sont en doubles croches ; celles du bruit sont en secondes. Les petits écarts de millième dans duration_steps viennent de l'arrondi de l'export de référence.

Canaux : pulse1 = mélodie, pulse2 = ponctuations discrètes, triangle = basse, noise = percussions. Aucun échantillon DPCM, aucune extension audio. Pulse1 et pulse2 utilisent un rapport cyclique de 25 %. Chaque canal joue une seule note à la fois. Répéter à la position 128, sans ajouter de mesure ni de fondu. Les silences entre notes sont intentionnels.

## Intégration par l'agent du jeu
1. Identifier le moteur sonore et la chaîne de compilation existants avant de choisir le format final. Le WAV et le MP3 sont des références d'écoute ; ils ne sont pas directement jouables par l'APU NES.
2. Convertir partition.json dans le format source du moteur déjà utilisé par le jeu. Ne pas remplacer son moteur audio sans nécessité. Si un tracker est utilisé, reporter les 128 positions et les quatre voix, puis exporter avec le pilote compatible choisi par le projet.
3. Programmer une boucle sur 8 mesures ; initialiser le morceau au début d'une partie et l'arrêter à la fin selon les règles du jeu. Ne pas le réinitialiser à chaque battement d'aile.
4. Faire passer les bruitages par l'arbitrage audio existant. Pulse2 et noise sont les premières voix à interrompre temporairement pour un bruitage. Restaurer ensuite l'état musical courant ; éviter que musique et bruitages écrivent simultanément sur un canal.
5. Valider plusieurs boucles dans l'émulateur cible, puis sur matériel si disponible. Vérifier tempo, absence de notes bloquées, transitions, et priorité des bruitages.

## Timbres et différences attendues
Le rendu fourni est une simulation logicielle des formes d'onde NES, pas un enregistrement de console. preview_gain est un coefficient de mixage de référence, pas un volume matériel ou une valeur de registre. La mélodie domine, la basse la soutient, pulse2 et les percussions restent discrètes. Refaire cette balance à l'oreille avec les enveloppes du moteur.

La synthèse pulse utilise une attaque très courte et une décroissance douce sur chaque note ; la basse triangle est coupée entre les notes. Le triangle matériel a une amplitude fixe : son preview_gain ne se traduit donc pas par un réglage de volume direct. Les percussions sont des bruits brefs avec décroissance ; le bruit logiciel de référence actualise son LFSR tous les 12 échantillons à 44,1 kHz. Choisir à l'écoute la période de bruit matérielle la plus proche : ce paramètre logiciel n'est pas un indice de période APU.

La référence quantifie les hauteurs avec une horloge de 1 789 773 Hz. Pour une autre région, utiliser les tables de hauteurs et le séquenceur régional du moteur. Ne pas reprendre un tempo en images fixe pour PAL et NTSC : conserver 136 BPM avec l'accumulateur temporel du moteur. Une conversion sur des rangées de tracker peut quantifier les durées ; préserver d'abord les débuts des notes et le caractère détaché.

## Livrable restant côté jeu
Ce paquet contient la composition et sa source de reproduction. Il ne contient pas de ROM, NSF, projet tracker ni données compilées pour un pilote précis : le moteur et la chaîne du jeu ne sont pas connus. L'agent doit produire l'export compatible avec ce projet et en vérifier la lecture. L'audio de référence a été généré ; aucune exécution sur console n'a été vérifiée.
