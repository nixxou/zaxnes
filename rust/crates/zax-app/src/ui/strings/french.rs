//! The French table of AMD-035.

use crate::ui::keys;

pub(super) const FRENCH: &[(&str, &str)] = &[
    // --------------------------------------------------------- menu bar
    (keys::MENU_FILE, "Fichier"),
    (keys::MENU_EMULATION, "Émulation"),
    (keys::MENU_VIDEO, "Vidéo"),
    (keys::MENU_AUDIO, "Audio"),
    (keys::MENU_TOOLS, "Outils"),
    (keys::MENU_RECENT_ROMS, "ROMs récentes"),
    (keys::MENU_LOAD_STATE, "Charger un état"),
    (keys::MENU_SAVE_STATE, "Sauvegarder l'état"),
    (keys::MENU_DELETE_STATE, "Supprimer un état"),
    (keys::MENU_SPEED, "Vitesse"),
    (keys::MENU_REGION, "Région"),
    (keys::MENU_CONSOLE_MODEL, "Modèle de console"),
    (keys::MENU_PORTS, "Ports"),
    (keys::MENU_CAMERA_MODE, "Mode caméra"),
    (keys::MENU_VOLUME, "Volume"),
    (keys::MENU_NOTHING_HERE, "(vide)"),
    (keys::MENU_REGION_AUTO, "Auto"),
    (keys::MENU_PORT, "Port"),
    (keys::MENU_PORT_CONTROLLER, "Manette"),
    (keys::MENU_PORT_ZAPPER, "Zapper"),
    (keys::MENU_CAMERA_ORBITAL, "Orbitale (TPV)"),
    (keys::MENU_CAMERA_FIRST_PERSON, "Première personne (FPV)"),
    (keys::SLOT_EMPTY, "vide"),
    (keys::SLOT_OTHER_PATCH, "faite avec les patchs intégrés réglés autrement"),
    (keys::SLOT_OTHER_HACK_VERSION, "faite avec une autre version du hack intégré"),
    (keys::SLOT_ENGINE, "moteur : {engine}"),
    (keys::SLOT_OTHER_ENGINE, "faite avec le moteur {engine} : choisissez le moteur {engine} pour ce jeu pour la charger"),
    (keys::MENU_MOVIES, "Lire un film"),
    (keys::WINDOW_MOVIE_NOTE, "Note sur cette frame"),
    (keys::MOVIE_NOTE_WRITE, "Écrire"),
    (keys::WINDOW_ROMS_FOLDER, "Dossier des ROM du mode console"),
    (keys::BUTTON_BROWSE, "Parcourir…"),
    (keys::ROMS_FOLDER_HINT, "Les jeux que liste le mode console. Vide : {0}"),
    (keys::SLOT_QUICK, "État rapide"),
    (keys::SLOT_AUTO, "Sauvegarde auto"),
    (keys::SLOT_PREFIX, "Emplacement"),
    // ----------------------------------------------------- window titles
    (keys::WINDOW_OPTIONS, "Options"),
    (keys::WINDOW_KEY_CONFIG, "Configuration des touches"),
    (keys::WINDOW_ABOUT, "À propos"),
    (keys::WINDOW_PROFILE_CHOOSER, "Choisir le profil 3D"),
    (keys::WINDOW_OPEN_ROM, "Ouvrir une ROM"),
    (keys::WINDOW_OPEN_MOVIE, "Ouvrir un film"),
    (keys::WINDOW_IMPORT_PROFILE, "Importer un profil 3D"),
    (keys::WINDOW_IMPORT_COVER, "Importer une jaquette"),
    (keys::WINDOW_IMPORT_THREE_DSEN, "Importer depuis 3dSen"),
    // ----------------------------------------------------------- buttons
    (keys::BUTTON_CLOSE, "Fermer"),
    (keys::BUTTON_CANCEL, "Annuler"),
    (keys::BUTTON_OPEN, "Ouvrir"),
    (keys::BUTTON_LAUNCH, "Lancer"),
    (keys::BUTTON_PARENT_FOLDER, "Dossier parent"),
    (keys::BUTTON_RESET_DEFAULTS, "Valeurs par défaut"),
    (keys::BUTTON_RESET_BINDINGS, "Réinitialiser toutes les touches"),
    (keys::BUTTON_STOP_BINDING, "Arrêter la capture"),
    (keys::BUTTON_YES, "Oui"),
    (keys::BUTTON_NO, "Non"),
    // AMD-056 REQ-UI-214 : les boutons des trois dialogues d'import. « OK » ne se traduit pas ;
    // « Replace » nomme l'action, donc « Remplacer » et non « Oui ».
    (keys::BUTTON_OK, "OK"),
    (keys::BUTTON_REPLACE, "Remplacer"),
    // ------------------- les dialogues d'import d'AMD-056 REQ-UI-214. Rien pour un import
    //                     réussi : il n'affiche ni notification ni dialogue, et l'entrée
    //                     « Profil importé : … » de REQ-UI-212 a disparu.
    (keys::DIALOG_IMPORT_REFUSED_TITLE, "Impossible d'importer ce fichier"),
    (keys::DIALOG_IMPORT_REFUSED_BODY, "{0} : {1}"),
    (keys::DIALOG_IMPORT_BUILT_IN_TITLE, "Déjà dans Zaxnes"),
    (keys::DIALOG_IMPORT_BUILT_IN_BODY, "{0} est un profil embarqué ; {1} n'a pas été importé."),
    (keys::DIALOG_IMPORT_REPLACE_TITLE, "Remplacer {0} ?"),
    (keys::DIALOG_IMPORT_REPLACE_BODY, "Le profil sera remplacé par celui de {0}. La ROM, la jaquette et les sauvegardes d'état seront conservées."),
    // ------------------------ notifications de SPEC-10 REQ-UI-082 à REQ-UI-084
    (keys::NOTIFY_ROLL_BACK, "Retour arrière"),
    (keys::NOTIFY_FAST_FORWARD, "Avance rapide"),
    (keys::NOTIFY_QUICK_SAVE, "Sauvegarde rapide"),
    (keys::NOTIFY_QUICK_LOAD, "Chargement rapide"),
    (keys::NOTIFY_CORRUPTED_ZIP, "Archive zip corrompue"),
    (keys::NOTIFY_STATE_OTHER_PATCH, "Cette sauvegarde a été faite avec les patchs de ROM intégrés réglés autrement."),
    (keys::NOTIFY_STATE_OTHER_HACK_VERSION, "Cette sauvegarde a été faite avec une autre version du hack intégré : elle ne peut pas être chargée."),
    (keys::CONFIRM_TURN_HACK_ON, "Pour jouer à cette sauvegarde, le hack intégré va être activé : le jeu sera rechargé, puis la sauvegarde chargée. Continuer ?"),
    (keys::CONFIRM_TURN_HACK_OFF, "Pour jouer à cette sauvegarde, le hack intégré va être désactivé : le jeu sera rechargé, puis la sauvegarde chargée. Continuer ?"),
    (keys::CONFIRM_OTHER_COMPONENTS, "Cette sauvegarde a été faite avec d'autres éléments du hack intégré (le widescreen, les aides) que ceux réglés maintenant. La charger avec les éléments actuels, ou régler ceux de la sauvegarde (le jeu sera rechargé) ?"),
    (keys::BUTTON_LOAD_AS_SET, "Éléments actuels"),
    (keys::BUTTON_LOAD_ALIGNED, "Ceux de la sauvegarde"),
    (keys::NOTIFY_UNCOMPRESSION_FAILED, "La décompression a échoué"),
    (keys::NOTIFY_INVALID_ROM, "Fichier ROM invalide."),
    (keys::NOTIFY_GAME_MISSING, "Le profil du jeu n'existe plus."),
    (keys::NOTIFY_LOADING_TIMED_OUT, "Délai de chargement dépassé."),
    (keys::NOTIFY_FOREIGN_CORE, "La sauvegarde d'état vient d'un autre cœur."),
    (keys::NOTIFY_NOT_A3DN_FILE, "Ce fichier n'est pas un fichier 3dn"),
    (keys::NOTIFY_UNSUPPORTED3DN_VERSION, "La version {0} du format 3dn n'est pas prise en charge"),
    (keys::NOTIFY_OBSOLETE_PROFILE_FORMAT, "{0} a un format de profil obsolète ou incorrect. Ce profil n'est pas compatible avec la version actuelle de Zaxnes."),
    // ------------- notifications des fichiers persistants (REQ-PERS-022, REQ-PERS-024)
    (keys::NOTIFY_SETTINGS_UNREADABLE, "Le fichier de réglages était illisible et a été réinitialisé."),
    (keys::NOTIFY_SETTINGS_NEWER_VERSION, "Les réglages ont été créés par une version plus récente."),
    (keys::NOTIFY_GAME_LIST_UNREADABLE, "La liste des jeux était illisible et a été réinitialisée."),
    (keys::NOTIFY_GAME_LIST_NEWER_VERSION, "La liste des jeux a été créée par une version plus récente."),
    (keys::NOTIFY_BINDINGS_UNREADABLE, "Les affectations de touches étaient illisibles et ont été réinitialisées."),
    (keys::NOTIFY_BINDINGS_NEWER_VERSION, "Les affectations de touches ont été créées par une version plus récente."),
    (keys::NOTIFY_THREE_DSEN_SUMMARY, "3dSen : {0} profil(s) importé(s), {1} déjà présent(s), {2} en échec."),
    // ------------------- confirmations de REQ-PERS-042 et REQ-PERS-054
    (keys::CONFIRM_DELETE_GAME, "Voulez-vous supprimer le jeu et ses sauvegardes d'état ?"),
    (keys::CONFIRM_DELETE_SLOT, "Voulez-vous supprimer l'emplacement {0} ?"),
    // ------------------------------------------------------------- hints
    (keys::HINT_IMPORT_THREE_DSEN, "Choisissez le dossier d'installation de 3dSen (celui qui contient 3dSen.exe), ou directement le fichier .bundle des profils."),
    (keys::HINT_RES_SCALE_DIRECT, "(sans effet en rendu direct : résolution native)"),
    (keys::HINT_RUN_AHEAD_FORCED, "(forcé à 0 : les scripts de ce jeu écrivent dans le cœur)"),
    (keys::HINT_PRESS_ACONTROL, "Appuyez sur une touche ou un bouton…"),
    (keys::HINT_CLICK_ACELL, "Cliquez sur une case pour réassigner."),
    (keys::HINT_AT_NEXT_START, "(au prochain démarrage)"),
    (keys::HINT_AT_NEXT_LOAD, "(au prochain chargement)"),
    (keys::HINT_PROFILE_SETTING, "(réglage de profil)"),
    // ---------------------------- RO-1 : les groupes de la fenêtre d'options
    (keys::GROUP_RESOLUTION, "Résolution"),
    (keys::GROUP_PERFORMANCE, "Performances"),
    (keys::GROUP_OVERSCAN, "Overscan"),
    (keys::GROUP_WIDESCREEN, "Écran large"),
    (keys::GROUP_REGION, "Région vidéo"),
    (keys::GROUP_ADVANCED, "Avancé"),
    // ---------------- RO-3 : les pages et les titres de groupe de l'arbre d'options console
    (keys::PAGE_ENHANCED_EFFECTS, "Effets améliorés"),
    (keys::PAGE_GEOMETRY, "Géométrie (courbure)"),
    (keys::GROUP_REFLECTIONS, "Réflexions"),
    (keys::GROUP_AMBIENT_OCCLUSION, "Occlusion ambiante"),
    // -------------- RO-1 point 6 : les deux lignes compactes des marges d'overscan, et la légende
    //                de chacune des quatre valeurs qu'une ligne compacte porte côte à côte.
    (keys::COMPACT_OVERSCAN_NTSC, "Overscan NTSC"),
    (keys::COMPACT_OVERSCAN_PAL, "Overscan PAL"),
    (keys::CAPTION_TOP, "haut"),
    (keys::CAPTION_BOTTOM, "bas"),
    (keys::CAPTION_LEFT, "gauche"),
    (keys::CAPTION_RIGHT, "droite"),
    // ------------- RO-1 point 1 : pourquoi une ligne est grisée. Chacune nomme le réglage qu'il
    //               faut changer pour que la ligne agisse de nouveau ; la valeur est conservée.
    (keys::HINT_RES_SCALE_AUTOMATIC, "(sans effet : la résolution automatique la choisit)"),
    (keys::HINT_NEEDS_SHADOWS, "(sans effet : les ombres sont désactivées)"),
    (keys::HINT_NEEDS_LUA_OPTIMIZED, "(sans effet : le Lua optimisé est désactivé)"),
    (keys::HINT_BG_COLOR_UNUSED, "(sans effet : ce fond est tiré des couleurs du jeu)"),
    (keys::HINT_NEEDS_ARC, "(sans effet : Arc horizontal est à 0)"),
    (keys::HINT_NEEDS_CURVATURE, "(sans effet : aucune courbure n'est réglée)"),
    (keys::HINT_REGION_AUTOMATIC, "(fixée par la détection au prochain chargement de ROM)"),
    (keys::HINT_SINGLE_CORE, "(cette version n'a qu'un cœur : la valeur est toujours 0)"),
    (keys::HINT_FCEUX_NO_WIDESCREEN, "(ce moteur n'a pas de panoramique : choisissez le moteur Zaxnes pour ce jeu)"),
    (keys::HINT_FCEUX_NO_HACKS, "(ce moteur n'applique aucun pack de hack : choisissez le moteur Zaxnes pour ce jeu)"),
    (keys::HINT_FCEUX_NO_SPRITE_LIMIT, "(ce moteur garde les 8 sprites par ligne : choisissez le moteur Zaxnes pour ce jeu)"),
    (keys::HINT_FCEUX_NO_RUN_AHEAD, "(l'avance de trames n'est pas implémentée sur ce moteur)"),
    (keys::STATUS_ENGINE_FCEUX, "moteur FCEUX"),
    (keys::STATUS_ENGINE_NESHAWK, "moteur NesHawk"),
    (keys::HINT_FCEUX_VALIDATED, "(cette ROM est validée pour la 3D sur le moteur FCEUX)"),
    (keys::HINT_FCEUX_NOT_VALIDATED, "(cette ROM n'est pas validée pour la 3D sur le moteur FCEUX : la 3D peut différer de celle de Zaxnes)"),
    (keys::HINT_CLASSIC_AUDIO, "(sans effet : la sortie audio classique a sa propre réserve fixe)"),
    (keys::HINT_NEEDS_ACHIEVEMENTS, "(sans effet : RetroAchievements est désactivé)"),
    (keys::HINT_STYLE_CLASSIC, "Style = Classic : ces effets sont désactivés."),
    (keys::HINT_MENU_BUTTON_UNUSED, "(conservé pour 3dSen : Zaxnes n'en tient pas compte)"),
    // ---------------------------------------------------- fields, columns
    (keys::FIELD_SEARCH, "Rechercher"),
    (keys::COLUMN_ACTION, "Action"),
    (keys::COLUMN_KEYBOARD, "Clavier"),
    (keys::COLUMN_GAMEPAD, "Manette"),
    (keys::VALUE_NO_GAMEPAD, "(aucune manette)"),
    // ---------------------------------------------------- welcome screen
    (keys::WELCOME_HINT, "Fichier ▸ Ouvrir une ROM"),
    (keys::WELCOME_RECENT, "ROMs récentes"),
    (keys::WELCOME_NO_RECENT, "Aucune ROM n'a encore été ouverte."),
    // -------------------------------------------------------- about block
    (keys::ABOUT_LICENCE, "GPL v3"),
    (keys::ABOUT_CREDIT_INSPIRED, "Projet indépendant inspiré de 3dSen (Geod Studio) ;"),
    (keys::ABOUT_CREDIT_ADAPTED, "contient du code adapté de 3DNes_Engine (Geod Studio, GPL v3)."),
    (keys::ABOUT_CREDIT_DISCLAIMER, "Sans lien avec Geod Studio ni Nintendo."),
    (keys::ABOUT_VERSION_PREFIX, "Version"),
    // -------------------------------------------------- options window tabs
    (keys::TAB_DISPLAY, "Affichage"),
    (keys::TAB_NES_PICTURE, "Image NES"),
    (keys::TAB_RENDER_3D, "Rendu 3D"),
    (keys::TAB_CAMERA, "Caméra"),
    (keys::TAB_LIGHT, "Lumière et ombres"),
    (keys::TAB_AUDIO, "Audio"),
    (keys::TAB_EMULATION, "Émulation"),
    (keys::TAB_INTERFACE, "Interface"),
    // ---------------------------------------- key configuration sections
    (keys::KEYS_PLAYER_ONE, "Joueur 1 — boutons NES"),
    (keys::KEYS_PLAYER_TWO, "Joueur 2 — boutons NES"),
    (keys::KEYS_CAMERA, "Caméra"),
    (keys::KEYS_HOTKEYS, "Raccourcis système"),
    (keys::KEYS_COMMANDS, "Commandes de menu"),
    // ------------------------------------------------------ file browser
    (keys::BROWSER_COMPUTER, "Poste de travail"),
    (keys::BROWSER_FILTER_NES_ROMS, "ROMs NES (*.nes, *.unf, *.studybox, *.fds, *.zip, *.7z)"),
    (keys::BROWSER_FILTER_NES_FILES, "Fichiers .nes, .unf, .studybox et .fds"),
    (keys::BROWSER_FILTER_MOVIES, "Films (.fm2, .fcm, .bk2, .mv)"),
    (keys::MOVIE_BAR_PLAY, "Lecture"),
    (keys::MOVIE_BAR_PAUSE, "Pause"),
    (keys::MOVIE_BAR_SEEKING, "vers"),
    (keys::PENS_HAND, "Main : le pointeur tourne la caméra"),
    (keys::PENS_UNDO, "Annuler"),
    (keys::PENS_CLEAR, "Effacer"),
    (keys::PENS_RED, "Stylo rouge"),
    (keys::PENS_BLUE, "Stylo bleu"),
    (keys::PENS_BLACK, "Stylo noir"),
    (keys::PENS_WHITE, "Stylo blanc"),
    (keys::PENS_VIEW_2D, "2D : l'image NES"),
    (keys::PENS_VIEW_ZAXNES, "3D telle que Zaxnes la dessine"),
    (keys::PENS_VIEW_3DSEN, "3D telle que 3dSen la dessine (sans les blocs Zaxnes des scripts)"),
    (keys::PENS_VIEW_HANDHELD, "3D telle que Zaxnes la dessine sur une console portable"),
    (keys::BROWSER_FILTER_ZIP, "Archives .zip et .7z"),
    (keys::BROWSER_FILTER_PROFILES, "Profils 3D (*.3dn, *.3dh)"),
    (keys::BROWSER_FILTER_BUNDLE, "Bundle de profils 3dSen (*.bundle)"),
    (keys::BROWSER_FILTER_PNG, "Images PNG (*.png)"),
    (keys::BROWSER_FILTER_ALL, "Tous les fichiers"),
    // ---------------------------------------------------- profile chooser
    (keys::PROFILE_TWO_DIMENSIONAL, "2D Mode"),
    (keys::PROFILE_SOURCE_TWO_DIMENSIONAL, "sans conversion 3D"),
    (keys::PROFILE_SOURCE_DEFAULT_THREE_D, "3D par défaut, démarre en 2D"),
    (keys::PROFILE_NONE, "Aucun profil"),
    (keys::PROFILE_SOURCE_BUILT_IN, "profil intégré"),
    (keys::PROFILE_SOURCE_PROGRAM_FOLDER, "profil du dossier du programme"),
    (keys::PROFILE_SOURCE_USER, "profil utilisateur"),
    // ---------------------------------------------------------- shortcuts
    (keys::SHORTCUT_ALT_ENTER, "Alt+Entrée"),
    (keys::SHORTCUT_SHIFT_PREFIX, "Maj"),
    (keys::SHORTCUT_QUICK_SAVE, "raccourci QuickSave"),
    (keys::SHORTCUT_QUICK_LOAD, "raccourci QuickLoad"),
    (keys::SHORTCUT_FAST_FORWARD, "."),
    (keys::SHORTCUT_ROLLBACK, ","),
    (keys::SHORTCUT_SAVE_SLOTS, "Maj+F1…F10"),
    (keys::SHORTCUT_LOAD_SLOTS, "Ctrl+F1…F10"),
    (keys::SHORTCUT_LONG_PRESS_MENU, "Échap maintenu"),
    // ------------------------------------------- commands of REQ-UI-205
    ("cmd.file.open_rom", "Ouvrir une ROM…"),
    ("cmd.file.open_recent_rom", "ROMs récentes"),
    ("cmd.file.choose_profile", "Choisir le profil 3D…"),
    ("cmd.file.import_profile", "Importer un profil…"),
    ("cmd.file.import_3dsen", "Depuis un dossier de 3dSen…"),
    ("cmd.file.import_3dsen_steam", "Depuis 3dSen installé par Steam"),
    ("cmd.file.import_3dsen_bundle", "Depuis le fichier de profils (.bundle)…"),
    ("cmd.file.import_3dsen_archive", "Depuis une archive (.zip, .7z)…"),
    ("menu.import_3dsen", "Importer depuis 3dSen"),
    ("import_3dsen.steam_missing", "3dSen n'a pas été trouvé dans les bibliothèques de Steam."),
    ("import_3dsen.bundle_title", "Fichier de profils de 3dSen"),
    ("import_3dsen.archive_title", "Archive contenant 3dSen ou ses profils"),
    ("cmd.file.launch_game", "Lancer le jeu"),
    ("cmd.file.delete_game", "Supprimer le jeu…"),
    ("cmd.file.create_game_art", "Créer la jaquette"),
    ("cmd.file.import_cover", "Importer une jaquette…"),
    // AMD-053 §4 : les quatre commandes de REQ-UI-210, REQ-UI-211 et REQ-LOOP-076.
    ("cmd.resume.last", "Reprendre"),
    ("cmd.favorite.toggle", "Favori"),
    ("cmd.library.filter_favorites", "Favoris seulement"),
    ("cmd.library.search", "Rechercher"),
    ("cmd.app.quit", "Quitter"),
    ("cmd.state.save_slot", "Sauvegarder l'état"),
    ("cmd.state.load_slot", "Charger un état"),
    ("cmd.state.quick_save", "État rapide : sauvegarder"),
    ("cmd.state.quick_load", "État rapide : charger"),
    ("cmd.state.delete_slot", "Supprimer l'état"),
    ("cmd.state.save_current_slot", "Sauvegarder sur le slot courant"),
    ("cmd.state.load_current_slot", "Charger le slot courant"),
    ("cmd.state.previous_slot", "Slot précédent"),
    ("cmd.state.next_slot", "Slot suivant"),
    ("cmd.video.next_shader", "Shader suivant"),
    ("cmd.cheats.previous", "Triche précédente"),
    ("cmd.cheats.next", "Triche suivante"),
    ("cmd.cheats.toggle", "Activer / désactiver la triche"),
    ("cmd.video.previous_shader", "Shader précédent"),
    ("cmd.video.screenshot", "Capture d'écran"),
    ("cmd.emulation.pause_resume", "Pause / Reprise"),
    ("cmd.emulation.pause", "Pause"),
    ("cmd.emulation.resume", "Reprise"),
    // AMD-044 §3 : les deux resets voisinent, le dur reçoit donc son nom.
    ("cmd.emulation.reset", "Reset complet"),
    ("cmd.emulation.soft_reset", "Reset doux"),
    ("cmd.emulation.insert_coin_1", "Insérer une pièce (fente 1)"),
    ("cmd.emulation.insert_coin_2", "Insérer une pièce (fente 2)"),
    ("cmd.emulation.vs_service", "Bouton service (VS. System)"),
    ("cmd.emulation.switch_disk_side", "Changer de face (Disk System)"),
    ("cmd.emulation.insert_next_disk", "Disque suivant (Disk System)"),
    ("cmd.emulation.eject_disk", "Éjecter le disque (Disk System)"),
    ("cmd.emulation.stop", "Arrêter"),
    ("cmd.emulation.fast_forward", "Avance rapide"),
    ("cmd.emulation.rollback", "Rollback"),
    ("cmd.emulation.frame_back", "Image précédente"),
    ("cmd.emulation.frame_advance", "Image suivante"),
    ("cmd.emulation.set_speed", "Vitesse"),
    ("cmd.emulation.speed_up", "Plus vite"),
    ("cmd.emulation.speed_down", "Moins vite"),
    ("cmd.emulation.set_region", "Région"),
    ("cmd.emulation.set_console_model", "Modèle de console"),
    ("cmd.emulation.auto_region", "Région : Auto"),
    ("cmd.emulation.set_port_type", "Type de port"),
    ("cmd.video.toggle_2d", "Mode 3D / Mode 2D"),
    ("cmd.video.reset_view", "Réinitialiser la vue"),
    ("cmd.video.set_camera_mode", "Mode caméra"),
    ("cmd.video.toggle_local_camera", "Vue de dessus"),
    ("cmd.video.set_skybox", "Skybox"),
    ("cmd.video.toggle_shadows", "Ombres"),
    ("cmd.video.toggle_reflections", "Reflets"),
    ("cmd.video.toggle_hack_option", "Option du hack"),
    ("cmd.video.toggle_fullscreen", "Plein écran"),
    ("cmd.video.toggle_fps", "Afficher les FPS"),
    ("cmd.audio.set_volume", "Volume"),
    ("cmd.audio.volume_up", "Volume +"),
    ("cmd.audio.volume_down", "Volume −"),
    ("cmd.audio.toggle_mute", "Muet"),
    ("cmd.audio.toggle_menu_music", "Musique de menu"),
    ("cmd.tools.options", "Options…"),
    ("cmd.tools.key_config", "Configuration des touches…"),
    ("cmd.tools.data_folder", "Dossier des données"),
    ("cmd.tools.about", "À propos"),
    ("cmd.tools.record_movie", "Enregistrer un film"),
    ("cmd.tools.play_movie", "Lire un film"),
    ("cmd.tools.stop_movie", "Arrêter le film"),
    ("cmd.tools.seek_movie", "Aller à une frame du film"),
    ("cmd.tools.reload_script", "Recharger les scripts du profil"),
    ("cmd.tools.open_movie", "Ouvrir un fichier de film..."),
    ("cmd.tools.annotate_movie", "Note sur cette frame…"),
    ("cmd.tools.editor_mode", "Mode éditeur"),
    ("cmd.tools.new_lua_profile", "Nouveau profil Lua pour ce jeu"),
    ("cmd.tools.copy_agent_prompt", "Copier le prompt pour l'agent IA"),
    ("cmd.ui.cancel_modal", "Fermer la fenêtre modale"),
    ("cmd.ui.toggle_debug_overlay", "Panneau de statistiques"),
    ("cmd.ui.close_windows", "Fermer les fenêtres"),
    ("cmd.ui.toggle_console", "Mode console"),
    ("cmd.ui.save_states", "Sauvegarder un état…"),
    ("cmd.ui.load_states", "Charger un état…"),
    // RO-2 part 1 : l'entrée nomme la fonction, le menu qui la porte ne la nomme plus.
    ("cmd.ui.achievements_account", "Compte et options RetroAchievements…"),
    // ------------------------------------------- labels of SPEC-09 settings
    ("setting.video.frame_rate", "Fréquence de rendu"),
    // AMD-052 §3 point 10 : juste après « Fréquence de rendu », dans le groupe Vidéo.
    ("setting.video.max_queued_frames", "Frames en file GPU"),
    ("setting.video.skip_frame", "Rendu accéléré (saut d'images)"),
    ("setting.video.vsync", "Synchronisation verticale"),
    ("setting.video.render_method", "Méthode de rendu"),
    ("setting.video.res_scale", "Échelle de résolution"),
    ("setting.video.anti_alias", "Anticrénelage"),
    ("setting.render.mesh_algo", "Algorithme de maillage"),
    ("setting.render.shadow", "Ombres"),
    // AMD-049 REQ-SCN-075, un réglage propre à Zaxnes : le backdrop que le script d'un profil
    // place derrière la scène est éclairé avec le modèle presque mat de l'original, sauf si ce
    // réglage est désactivé.
    ("setting.render.backdrop_lighting", "Éclairage du backdrop"),
    (
        "help.render.backdrop_lighting",
        "Éclaire le fond placé derrière la scène avec un léger reflet, comme 3dSen. Désactivez-le \
         pour un éclairage diffus simple, un peu moins coûteux sur une petite machine. Seuls \
         certains profils affichent un fond.",
    ),
    ("setting.camera.fov", "Champ de vision (degrés)"),
    ("setting.camera.frame_picture", "Cadrage 3D comme l'image 2D"),
    ("setting.light.intensity", "Intensité lumineuse"),
    ("setting.scene.skybox", "Fond (skybox)"),
    ("setting.scene.custom_bg_color", "Couleur de fond"),
    // Le groupe « Style » d'AMD-055 §3 point 6 : les treize libellés et les treize aides de son
    // §4, tels que l'amendement les écrit. Le groupe vient juste après la ligne Skybox, où
    // SPEC-10 REQ-UI-061 amendée le place.
    ("group.style", "Style"),
    ("setting.render.visual_style", "Style"),
    (
        "help.render.visual_style",
        "Classic garde l'aspect d'origine. Enhanced ajoute des effets optionnels.",
    ),
    ("setting.render.style.ao", "Occlusion ambiante"),
    (
        "help.render.style.ao",
        "Ombre douce dans les coins et les creux.",
    ),
    ("option.MSVO", "MSVO"),
    ("option.Depth", "Depth"),
    ("setting.render.style.ao_strength", "Force"),
    (
        "help.render.style.ao_strength",
        "Intensité de l'assombrissement.",
    ),
    ("setting.render.style.ao_radius", "Rayon"),
    (
        "help.render.style.ao_radius",
        "Portée de l'assombrissement.",
    ),
    ("setting.render.style.ao_thickness", "Épaisseur"),
    (
        "help.render.style.ao_thickness",
        "Sans effet avec l'occlusion Depth.",
    ),
    (
        "hint.style.thickness_no_effect",
        "sans effet avec l'occlusion Depth",
    ),
    ("setting.render.style.ao_quality", "Qualité de l'occlusion"),
    (
        "help.render.style.ao_quality",
        "Plus d'échantillons, ombre plus lisse.",
    ),
    ("option.Low", "Faible"),
    ("option.Medium", "Moyenne"),
    ("option.High", "Élevée"),
    ("option.Ultra", "Ultra"),
    ("setting.render.style.pixel_shadow", "Ombres pixel"),
    (
        "help.render.style.pixel_shadow",
        "Des ombres qui suivent la grille de pixels du jeu.",
    ),
    ("setting.render.style.voxel_grid", "Grille de voxels"),
    (
        "help.render.style.voxel_grid",
        "Des rainures tous les 2, 4 ou 8 voxels, comme des briques de jouet.",
    ),
    ("option.Every 2 voxels", "Tous les 2 voxels"),
    ("option.Every 4 voxels", "Tous les 4 voxels"),
    ("option.Every 8 voxels", "Tous les 8 voxels"),
    ("setting.render.style.reflections", "Reflets"),
    (
        "help.render.style.reflections",
        "Le sol reflète la scène comme une surface polie.",
    ),
    ("setting.render.style.ssr_shine", "Brillance du sol"),
    ("help.render.style.ssr_shine", "Intensité du reflet."),
    ("setting.render.style.ssr_clarity", "Netteté"),
    (
        "help.render.style.ssr_clarity",
        "Une valeur élevée garde le reflet net.",
    ),
    ("setting.render.style.ssr_length", "Longueur"),
    ("help.render.style.ssr_length", "Portée du reflet."),
    ("setting.render.style.ssr_quality", "Qualité des reflets"),
    (
        "help.render.style.ssr_quality",
        "Plus de pas, moins de trous dans le reflet.",
    ),
    // Le groupe « Géométrie » d'AMD-043 §6 et d'AMD-046 §3 : les libellés et les aides, tels que
    // les deux amendements les écrivent.
    ("group.geometry", "Géométrie"),
    ("setting.scene.bend_lr", "Courbure gauche-droite"),
    (
        "help.scene.bend_lr",
        "Courbe la scène horizontalement. Une valeur positive la bombe comme un écran \
         cathodique ; une valeur négative l'enroule autour du joueur.",
    ),
    ("setting.scene.bend_tb", "Courbure haut-bas"),
    (
        "help.scene.bend_tb",
        "Courbe la scène verticalement. Une valeur positive la bombe ; une valeur négative \
         l'enroule autour du joueur.",
    ),
    ("setting.scene.arc_h", "Arc horizontal"),
    (
        "help.scene.arc_h",
        "Pose le niveau sur un arc, comme sur une toute petite planète. Idéal pour les jeux à \
         défilement horizontal.",
    ),
    ("setting.scene.arc_center", "Centre de l'arc"),
    (
        "help.scene.arc_center",
        // La phrase d'AMD-043 §6, à laquelle RO-5 partie 2 ajoute les quarts sans la remplacer.
        "Autour de quoi l'arc se courbe : le centre de l'écran, son bord bas, ou l'un des quarts \
         entre les deux.",
    ),
    ("option.Screen Center", "Centre de l'écran"),
    ("option.Screen Edge", "Bord de l'écran"),
    // RO-5 partie 2 : les trois quarts entre ces deux hauteurs.
    ("option.25%", "25 %"),
    ("option.50%", "50 %"),
    ("option.75%", "75 %"),
    // AMD-046 §3, réglage propre à Zaxnes : l'image plate du mode 2D suit la courbure
    // d'AMD-043 sauf si on le désactive.
    ("setting.scene.bend_2d_mode", "Courber le mode 2D"),
    (
        "help.scene.bend_2d_mode",
        "Applique aussi la courbure à l'image plate du mode 2D. Désactivez-le pour garder \
         l'image plate.",
    ),
    ("setting.emu.game_speed", "Vitesse de jeu"),
    ("setting.emu.auto_tv_mode", "Région détectée automatiquement"),
    ("setting.emu.catch_up", "Rattrapage en cas de retard (frames)"),
    // AMD-041 §3 : le titre et les quatre valeurs de la liste Run-ahead.
    ("setting.emu.run_ahead", "Run-ahead"),
    ("option.Run-ahead off", "Désactivé"),
    ("option.1 frame (Recommended)", "1 frame (recommandé)"),
    ("option.2 frames", "2 frames"),
    ("option.3 frames", "3 frames"),
    ("option.10 frames", "10 frames"),
    ("option.9 frames", "9 frames"),
    ("option.8 frames", "8 frames"),
    ("option.7 frames", "7 frames"),
    ("option.6 frames", "6 frames"),
    ("option.5 frames", "5 frames"),
    ("option.4 frames", "4 frames"),
    ("setting.emu.lua_smooth_gc", "Nettoyage mémoire Lua lissé"),
    ("setting.emu.lua_optimized", "Lua optimisé (unsafe)"),
    (
        "setting.emu.lua_experimental",
        "Optimisation Lua expérimentale (nécessite Lua optimisé)",
    ),
    ("setting.emu.pipeline", "Scripts en parallèle du dessin"),
    ("setting.emu.pause_3d_in_2d", "3D en pause en 2D"),
    ("setting.emu.catch_up_last_3d", "Rattrapage : 3D de l'image affichée seulement"),
    ("setting.video.half_rate_3d", "3D à 30 i/s (jeu à pleine vitesse)"),
    ("setting.video.dynamic_resolution", "Résolution automatique"),
    ("setting.video.shader_resolution", "Résolution du shader"),
    (
        "detail.video.shader_resolution",
        "La part de l'écran à laquelle les shaders slang dessinent, étirée ensuite sur l'écran : \
         50 % coûte environ quatre fois moins au processeur graphique (Mega Bezel fluide sur un \
         téléphone), pour un masque CRT et des lignes de balayage plus doux. Par défaut : 100 %.",
    ),
    ("setting.video.backend", "API graphique (au prochain démarrage)"),
    ("setting.video.widescreen", "Écran large (ROM prévues pour)"),
    ("setting.video.widescreen_extra", "Largeur supplémentaire de l'écran large"),
    (
        "setting.video.widescreen_strict_edges",
        "Écran large : attendre les objets entiers aux bords dessinés",
    ),
    (
        "setting.emu.smooth_scroll_ahead",
        "Défilement lissé (interpolé par run-ahead)",
    ),
    ("setting.emu.smooth_scroll_3d", "Défilement lissé aussi en 3D"),
    (
        "setting.emu.smooth_scroll_view",
        "Afficher la zone du défilement lissé (rouge, sprites fixes en vert)",
    ),
    (
        "setting.video.remove_sprite_limit",
        "Supprimer la limite de 8 sprites par ligne",
    ),
    ("setting.video.overscan_ntsc_top", "Overscan NTSC : haut"),
    ("setting.video.overscan_ntsc_bottom", "Overscan NTSC : bas"),
    ("setting.video.overscan_ntsc_left", "Overscan NTSC : gauche"),
    ("setting.video.overscan_ntsc_right", "Overscan NTSC : droite"),
    ("setting.video.overscan_pal_top", "Overscan PAL : haut"),
    ("setting.video.overscan_pal_bottom", "Overscan PAL : bas"),
    ("setting.video.overscan_pal_left", "Overscan PAL : gauche"),
    ("setting.video.overscan_pal_right", "Overscan PAL : droite"),
    ("setting.video.overscan_top", "Overscan : haut"),
    ("setting.video.overscan_bottom", "Overscan : bas"),
    ("setting.video.overscan_left", "Overscan : gauche"),
    ("setting.video.overscan_right", "Overscan : droite"),
    ("setting.video.overscan_mode", "Overscan : fit (bords masqués) ou resize (agrandi)"),
    ("setting.video.nes_palette", "Palette de couleurs NES"),
    ("setting.video.composite_signal", "Signal composite"),
    ("setting.video.emphasis_3d", "Accentuation des couleurs en 3D"),
    ("setting.emu.builtin_patches", "Patchs de ROM intégrés (au prochain chargement)"),
    ("setting.emu.tv_mode", "Région vidéo"),
    ("setting.emu.core", "Cœur d'émulation"),
    ("setting.emu.engine", "Moteur d'émulation (ce jeu)"),
    ("setting.emu.cartridge_db", "Base de cartouches"),
    ("setting.emu.oam_corruption", "Corruption OAM"),
    ("setting.emu.ppu_bus_conflicts", "Conflits de bus PPU"),
    ("setting.emu.oamaddr_bug", "Défaut OAMADDR (2C02G)"),
    ("setting.emu.oam_decay", "Effacement de l'OAM"),
    ("setting.emu.rainbow_network", "Réseau de la carte Rainbow"),
    ("setting.emu.vs_dip_1", "DIP switch 1 du VS. System"),
    ("setting.emu.vs_dip_2", "DIP switch 2 du VS. System"),
    ("setting.emu.vs_dip_3", "DIP switch 3 du VS. System"),
    ("setting.emu.vs_dip_4", "DIP switch 4 du VS. System"),
    ("setting.emu.vs_dip_5", "DIP switch 5 du VS. System"),
    ("setting.emu.vs_dip_6", "DIP switch 6 du VS. System"),
    ("setting.emu.vs_dip_7", "DIP switch 7 du VS. System"),
    ("setting.emu.vs_dip_8", "DIP switch 8 du VS. System"),
    ("setting.emu.power_on_top", "Allumage en haut de l'image"),
    ("setting.emu.ppu_2006_delay", "Délai de chargement $2006"),
    ("setting.emu.ppu_2006_glitch", "Glitch de défilement $2006"),
    ("setting.emu.ppu_2000_glitch", "Glitch bus ouvert $2000/$2005"),
    ("setting.emu.palette_corruption", "Corruption de palette"),
    ("setting.emu.ppu_reset_line", "Reset du PPU"),
    ("setting.emu.cpu_ppu_alignment", "Alignement CPU/PPU"),
    ("setting.emu.cpu_apu_parity", "Parité CPU/APU"),
    ("option.Zaxnes", "Zaxnes"),
    ("option.RetroArch", "RetroArch"),
    ("option.2A03 netlist", "Netlist du 2A03"),
    ("setting.emu.fidelity_plus", "Fidélité+"),
    ("options.new.to_new", "Nouvelle fenêtre d'options"),
    ("options.new.to_classic", "Ancienne fenêtre"),
    ("options.new.advanced", "Mode avancé"),
    ("options.new.simple_line", "Mode simple : l'essentiel"),
    ("options.new.advanced_line", "Mode avancé : tous les réglages"),
    ("options.new.tab.essentials", "Essentiel"),
    ("options.new.tab.controllers", "Manettes"),
    ("options.new.controllers_line", "Les boutons de chaque manette et du clavier, pour les deux joueurs."),
    ("options.new.controllers_open", "Configurer les manettes..."),
    ("options.new.preset_title", "Performance ou fidélité :"),
    ("options.new.modified", "(modifié)"),
    ("options.new.faster", "Plus rapide (consoles portables)"),
    ("options.new.truer", "Plus fidèle (machine de bureau)"),
    ("options.new.preset_options", "Options de ce préréglage"),
    ("options.new.essentials", "L'essentiel"),
    ("options.new.next_game", "au prochain jeu"),
    ("options.new.shaders_open", "Configurer les shaders..."),
    ("options.new.next_start", "au prochain démarrage"),
    ("options.new.tab.widescreen", "Écran large et hacks"),
    ("options.new.game_hacks_line", "Le jeu en cours a ses propres réglages :"),
    ("options.new.game_widescreen", "Écran large de ce jeu..."),
    ("options.new.game_hacks", "Hacks de ce jeu..."),
    ("options.new.no_game", "Lancez un jeu pour régler les siens."),
    ("setting.video.signal_kind", "Type de signal"),
    ("setting.video.tv_model", "Téléviseur"),
    ("setting.video.decoder_sharpness", "Netteté du décodeur"),
    ("setting.video.comb_filter", "Filtre en peigne (3 lignes)"),
    ("setting.video.frozen_dot_crawl", "Dot crawl figé"),
    ("setting.video.rf_noise", "RF : neige"),
    ("setting.video.rf_ghost", "RF : image fantôme"),
    ("setting.video.tv_hue", "Teinte (degrés)"),
    ("setting.video.tv_saturation", "Saturation (%)"),
    ("setting.video.tv_contrast", "Contraste (%)"),
    ("setting.video.tv_brightness", "Luminosité"),
    ("detail.video.signal_kind", "Ce que le téléviseur reçoit quand le signal est actif. Composite : le câble jaune, luma et couleur sur un seul fil, d'où les franges et le dot crawl. S-Vidéo : luma et couleur séparées, comme une console modifiée sur une entrée S-Vidéo ; plus de franges, les couleurs du signal. RF : la prise antenne, plus floue, avec de la neige et une faible image fantôme. Par défaut : composite."),
    ("detail.video.tv_model", "Le téléviseur sur lequel l'image est montrée, aussi bien l'image plate (palette ou signal) que la vue 3D. Écran moderne : les couleurs telles quelles. Un tube applique sa courbe gamma mesurée et ses propres phosphores, convertis pour un écran sRGB, comme BeesNES : tube NTSC (RCA ColorTrak de 1989), tube japonais (Toshiba CF2005), tube PAL (Sony Trinitron). Le blanc garde la teinte du tube, un peu bleutée pour les postes américains et japonais. Ne coûte rien. Par défaut : écran moderne."),
    ("detail.video.decoder_sharpness", "La largeur des filtres du décodeur du signal. Douce : bords plus doux, moins de franges. Normale : le décodeur de référence (luma sur un cycle de sous-porteuse, couleur sur deux). Nette : bords plus nets, un léger motif de couleur reste dans les aplats. Par défaut : normale."),
    ("detail.video.comb_filter", "Le filtre en peigne des téléviseurs des années 90, en NTSC : la sous-porteuse de la console avance d'un tiers de cycle par ligne, donc la couleur de trois lignes s'annule là où l'image ne change pas verticalement, ce qui retire la plupart des franges sans flouter. Sans effet en S-Vidéo (déjà séparé) et en PAL (la ligne à retard du poste fait ce travail). Par défaut : activé."),
    ("detail.video.frozen_dot_crawl", "Garde la phase du signal identique d'une image à l'autre : les franges restent fidèles mais ne scintillent plus. Utile pour une capture d'écran ou si le scintillement gêne. Par défaut : désactivé."),
    ("detail.video.rf_noise", "La neige du signal RF, de 0 à 100 : le grain qu'une prise antenne ajoute à l'image, différent à chaque image. Seulement avec le signal RF. Par défaut : 30."),
    ("detail.video.rf_ghost", "L'image fantôme du signal RF, de 0 à 100 : un faible écho de l'image deux pixels et demi à droite, comme le donne un signal d'antenne réfléchi. Seulement avec le signal RF. Par défaut : 15."),
    ("detail.video.tv_hue", "Le bouton de teinte du téléviseur, de -30 à 30 degrés, sur le signal comme sur la palette. Par défaut : 0."),
    ("detail.video.tv_saturation", "Le bouton de couleur du téléviseur, de 0 (noir et blanc) à 200 %. Par défaut : 100 %."),
    ("detail.video.tv_contrast", "Le bouton de contraste (image) du téléviseur, de 50 à 150 %. Par défaut : 100 %."),
    ("detail.video.tv_brightness", "Le bouton de luminosité du téléviseur, de -50 à 50 (centièmes de l'écart noir-blanc). Par défaut : 0."),
    ("option.Composite", "Composite"),
    ("option.S-Video", "S-Vidéo"),
    ("option.RF", "RF (antenne)"),
    ("option.Modern screen", "Écran moderne"),
    ("option.NTSC tube (RCA ColorTrak)", "Tube NTSC (RCA ColorTrak)"),
    ("option.Japanese tube (Toshiba)", "Tube japonais (Toshiba)"),
    ("option.PAL tube (Trinitron)", "Tube PAL (Trinitron)"),
    ("option.Soft", "Douce"),
    ("option.Normal", "Normale"),
    ("option.Sharp", "Nette"),
    ("options.new.signal", "Signal vidéo"),
    ("options.new.signal.help", "Désactivé : les couleurs de la palette. Sinon l'image plate est décodée du signal de la console comme un téléviseur le fait (Fidélité+, un peu de temps processeur par image)."),
    ("options.new.signal.off", "Désactivé (palette)"),
    ("options.new.signal.decoder", "Décodeur"),
    ("options.new.signal.knobs", "Réglages du téléviseur"),
    ("options.new.signal.neutral", "Neutre"),
    ("options.new.signal.palette_note", "Avec le signal actif, l'image plate prend les couleurs du signal ; la palette reste celle de la vue 3D."),
    ("hint.needs_signal", "(sans effet : le signal vidéo est désactivé)"),
    ("hint.comb_svideo", "(sans effet en S-Vidéo : luma et couleur sont déjà séparées)"),
    ("hint.needs_rf", "(sans effet : seulement avec le signal RF)"),
    ("shaders.look.guest", "CRT Guest"),
    ("shaders.look.guest.help", "Le tube de référence : net, lumineux, très réglable."),
    ("shaders.look.mega_bezel", "Mega Bezel (TV)"),
    ("shaders.look.mega_bezel.help", "Une télévision autour de l'image, ou son seul écran."),
    ("shaders.look.crt_light", "CRT léger"),
    ("shaders.look.crt_light.help", "Des tubes légers, pour une petite carte graphique ou une console portable."),
    ("shaders.look.crt_classic", "CRT classique"),
    ("shaders.look.crt_classic.help", "Les tubes classiques : Lottes, Hyllian, Geom, Mattias."),
    ("shaders.look.royale", "CRT Royale"),
    ("shaders.look.royale.help", "Le tube le plus détaillé, et le plus lourd."),
    ("shaders.look.composite", "Signal composite"),
    ("shaders.look.composite.help", "Les couleurs baveuses de la sortie de la console, sans tube."),
    ("shaders.look.lcd", "LCD"),
    ("shaders.look.lcd.help", "L'écran d'une console portable."),
    ("shaders.look.smoothing", "Lissage"),
    ("shaders.look.smoothing.help", "Les pixels lissés, sans tube (devant un tube, c'est le pré-filtre)."),
    ("shaders.look.custom", "Personnalisé"),
    ("shaders.look.choose", "Choisir un style..."),
    ("shaders.look.title", "Style"),
    ("shaders.look.all_presets", "Tous les presets du pack"),
    ("shaders.look.options", "Options du style"),
    ("shaders.option.signal", "Signal"),
    ("shaders.option.frame", "Cadre TV"),
    ("shaders.option.quality", "Qualité"),
    ("shaders.option.model", "Modèle"),
    ("shaders.option.method", "Méthode"),
    ("shaders.option.curvature", "Courbure"),
    ("shaders.option.scanlines", "Lignes de balayage"),
    ("shaders.option.mask", "Grille (masque)"),
    ("shaders.option.brightness", "Luminosité"),
    ("shaders.option.reflections", "Reflets"),
    ("shaders.option.artifacts", "Artefacts couleur"),
    ("shaders.option.sharpness", "Netteté"),
    ("shaders.option.black_level", "Noirs relevés"),
    ("shaders.option.subpixels", "Ordre des sous-pixels"),
    ("shaders.choice.yes", "Oui"),
    ("shaders.choice.no", "Non"),
    ("shaders.choice.light", "Léger"),
    ("shaders.choice.standard", "Standard"),
    ("shaders.choice.advanced", "Avancé"),
    ("shaders.choice.rgb", "RGB"),
    ("shaders.choice.svideo", "S-Vidéo"),
    ("shaders.choice.composite", "Composite"),
    ("shaders.choice.rf", "RF (antenne)"),
    ("shaders.choice.zfast", "zfast"),
    ("shaders.choice.crt_pi", "CRT-Pi"),
    ("shaders.choice.easymode", "Easymode"),
    ("shaders.choice.nes_mini", "NES Mini"),
    ("shaders.choice.fakelottes", "Fake Lottes"),
    ("shaders.choice.lottes", "Lottes"),
    ("shaders.choice.hyllian", "Hyllian"),
    ("shaders.choice.geom", "Geom"),
    ("shaders.choice.mattias", "Mattias"),
    ("shaders.choice.ntsc", "NTSC 256 px"),
    ("shaders.choice.blargg", "Blargg"),
    ("shaders.choice.sharp", "Net"),
    ("shaders.choice.xbrz", "xBRZ"),
    ("shaders.choice.scalefx", "ScaleFX"),
    ("shaders.step.none", "Aucune"),
    ("shaders.step.light", "Légère"),
    ("shaders.step.normal", "Normale"),
    ("shaders.step.strong", "Forte"),
    ("shaders.step.darker", "Plus sombre"),
    ("shaders.step.brighter", "Plus lumineuse"),
    ("shaders.step.fine", "Fine"),
    ("shaders.step.trinitron", "Trinitron"),
    ("shaders.step.slot", "Fente (slot)"),
    ("shaders.step.soft", "Douce"),
    ("shaders.step.sharp", "Nette"),
    ("shaders.step.rgb", "RGB"),
    ("shaders.step.bgr", "BGR"),
    ("shaders.step.custom", "Personnalisée"),
    ("options.new.mode", "Mode des options"),
    ("options.new.mode.simple", "Simple"),
    ("options.new.mode.advanced", "Avancé"),
    ("options.new.widescreen_default", "Écran large activé par défaut"),
    ("options.new.widescreen_ratio", "Format"),
    ("options.new.hack.no_slowdown", "Pas de ralentissements"),
    ("options.new.hack.invincible", "Invincible"),
    ("options.new.hack.level_select", "Sélection du niveau"),
    ("options.new.hack.margin_enemies", "Ennemis dans les marges"),
    ("options.new.hack.margin_freeze", "Ennemis figés dans les marges"),
    ("options.new.hack.margin_freeze_tint", "Ennemis figés en gris"),
    ("options.new.widescreen_line", "Valeurs par défaut des jeux ayant un patch écran large ; les choix faits pour un jeu dans sa fenêtre passent avant."),
    ("options.preset.fastest.name", "Rapidité max"),
    ("options.preset.fastest.summary", "Pour les consoles portables et les machines modestes. La même console, avec beaucoup moins de calcul : les images et les sons ne changent pas."),
    ("options.preset.fastest.acts", "Cœur rapide (2,5 à 3,5 fois moins de calcul sur les cartes courantes)\nLua optimisé et expérimental\nPas d'anticipation (run-ahead 0)\nScripts en parallèle du dessin (Auto)\nRésolution automatique, pas d'ombres\nRattrapage : seule la dernière image en retard passe en 3D\n3D en pause quand la 2D est affichée\nStyle Classique"),
    ("options.preset.fast.name", "Rapide"),
    ("options.preset.fast.summary", "Le cœur rapide, le reste comme d'habitude."),
    ("options.preset.fast.acts", "Cœur rapide\nRésolution automatique\nRattrapage : seule la dernière image en retard passe en 3D\n3D en pause quand la 2D est affichée\nStyle Classique"),
    ("options.preset.balanced.name", "Équilibré"),
    ("options.preset.balanced.summary", "Les réglages par défaut : le cœur standard, la manette lue en temps réel."),
    ("options.preset.balanced.acts", "Cœur standard\nManette lue en temps réel\nRattrapage : seule la dernière image en retard passe en 3D\nStyle Amélioré, sans reflets ni occlusion"),
    ("options.preset.faithful.name", "Fidèle"),
    ("options.preset.faithful.summary", "Chaque image passe en 3D ; la cadence exacte de la console est proposée."),
    ("options.preset.faithful.acts", "Manette lue en temps réel\nDéfauts matériels courants reproduits\nChaque image convertie en 3D, même en rattrapage\nStyle Amélioré : occlusion, ombres pixel (reflets proposés)"),
    ("options.preset.most_faithful.name", "Fidélité max"),
    ("options.preset.most_faithful.summary", "L'émulation la plus fidèle, sans compromis. Pour une machine de bureau."),
    ("options.preset.most_faithful.acts", "Fidélité+ (CPU et PPU sur les fronts d'horloge)\nCadence exacte de la console\nCorruption à l'écriture d'OAMADDR (certains jeux rares en ont besoin)\nChaque image convertie en 3D, même en rattrapage\nStyle Amélioré : occlusion haute qualité, ombres pixel (reflets proposés)"),
    ("setting.emu.fast_core", "Cœur rapide"),
    ("setting.emu.exact_rate", "Cadence de la console"),
    ("setting.emu.realtime_input", "Manette en temps réel"),
    ("setting.emu.late_present", "Présentation tardive"),
    ("option.Alignment 0", "Alignement 0"),
    ("option.Alignment 1", "Alignement 1"),
    ("option.Alignment 2", "Alignement 2"),
    ("option.Alignment 3", "Alignement 3"),
    ("option.Blargg's console", "Console de blargg"),
    ("option.2C02G", "2C02G"),
    ("option.2C02 before G", "2C02 avant G"),
    ("setting.emu.mmc3_revision", "Révision du MMC3"),
    ("option.The cartridge's", "Celle de la cartouche"),
    ("option.MMC3A / MMC6", "MMC3A / MMC6"),
    ("option.MMC3B / MMC3C", "MMC3B / MMC3C"),
    ("setting.emu.cpu_revision", "Révision du processeur"),
    ("option.RP2A03G and later", "RP2A03G et suivants"),
    ("option.Letterless RP2A03", "RP2A03 sans lettre"),
    ("setting.emu.power_on_ram", "RAM à l'allumage"),
    ("setting.emu.power_on_ppu_ram", "Mémoire vidéo à l'allumage"),
    ("setting.emu.ppu_revision", "Révision de la puce vidéo"),
    ("setting.emu.oamaddr_write_corruption", "Corruption par OAMADDR"),
    ("option.Zeros", "Zéros"),
    ("option.FCEUX pattern", "Motif FCEUX"),
    ("option.All $FF", "Tout à $FF"),
    ("option.Random", "Aléatoire"),
    ("setting.audio.volume", "Volume général"),
    ("setting.audio.bg_music", "Musique de menu"),
    ("setting.audio.mute", "Muet"),
    ("setting.audio.latency_ms", "Latence de sortie (ms)"),
    ("setting.audio.classic_output", "Sortie audio classique (v0.4.2)"),
    ("setting.audio.console_profile", "Profil audio de console"),
    (
        "detail.audio.console_profile",
        "Fait passer le son par les filtres d'une vraie console, mesurés par l'émulateur BeesNES : chaque modèle de NES ou de Famicom, et chaque sortie vidéo, adoucit différemment les aigus et les graves et équilibre un peu différemment les cinq voies. La plus ancienne NES (N0260073) n'a pas non plus le mode de bruit court. Aucun garde le son habituel de Zaxnes. Prend effet immédiatement. Par défaut : aucun.",
    ),
    ("option.NES Front-Loading N0260073", "NES Front-Loading N0260073"),
    ("option.NES Front-Loading N1151667", "NES Front-Loading N1151667"),
    ("option.NES Front-Loading N13721267", "NES Front-Loading N13721267"),
    ("option.NES Front-Loading N20914398, Sunsoft 5B", "NES Front-Loading N20914398, Sunsoft 5B"),
    ("option.NES Front-Loading N32670105", "NES Front-Loading N32670105"),
    ("option.NES Front-Loading N34169630", "NES Front-Loading N34169630"),
    ("option.NES Front-Loading N8869725, NESRGB", "NES Front-Loading N8869725, NESRGB"),
    ("option.NES Front-Loading N8869725, NESRGB, stock RCA", "NES Front-Loading N8869725, NESRGB, stock RCA"),
    ("option.NES Top-Loading NN101255729, NESRGB", "NES Top-Loading NN101255729, NESRGB"),
    ("option.Twin Famicom 441850, stock RCA", "Twin Famicom 441850, stock RCA"),
    ("option.Twin Famicom 475711, NESRGB, stock RCA", "Twin Famicom 475711, NESRGB, stock RCA"),
    ("option.Twin Famicom 475711, NESRGB, SCART", "Twin Famicom 475711, NESRGB, SCART"),
    ("option.PAL NES NESE-001 (AUS) PM313006", "PAL NES NESE-001 (AUS) PM313006"),
    ("option.PAL NES NESE-001 (GBR) PM608755", "PAL NES NESE-001 (GBR) PM608755"),
    ("help.audio.classic_output", "À utiliser si la nouvelle sortie audio se comporte mal sur cet appareil ; elle a l'ancienne réserve, plus courte, et peut craquer."),
    ("setting.input.emulated_menu_button", "Menu = Select + Start"),
    ("setting.input.touch_pad", "Pad tactile"),
    ("setting.input.touch_scale", "Taille du pad tactile"),
    ("setting.input.touch_haptic_dpad", "Vibration de la croix"),
    ("setting.input.touch_haptic_buttons", "Vibration des boutons"),
    ("setting.input.touch_extra", "Retour arrière et avance rapide"),
    ("setting.ui.orientation", "Orientation"),
    ("setting.ui.fullscreen", "Plein écran"),
    ("setting.ui.profiler", "Mesures de performance (journal)"),
    ("setting.ui.editor_mode", "Mode éditeur (serveur MCP, stylos, dossier editor)"),
    ("setting.ui.roms_folder", "Dossier des ROM"),
    ("setting.ui.show_fps", "Afficher les FPS"),
    ("setting.ui.sort_option", "Tri de la bibliothèque"),
    ("setting.ui.key_layout", "Disposition des touches"),
    ("setting.ui.escape_quits", "Échap deux fois pour quitter"),
    ("setting.ui.language", "Langue"),
    // ----------------------------------- labels of the enumerated values
    ("option.Off", "Désactivée"),
    ("option.Auto", "Auto"),
    ("option.Light", "Légère"),
    ("option.Strong", "Forte"),
    ("option.Landscape", "Paysage"),
    ("option.Portrait", "Portrait"),
    ("option.Always", "Toujours"),
    ("option.Disabled", "Désactivé"),
    ("option.Smart", "Intelligent"),
    ("option.Every refresh", "À chaque rafraîchissement"),
    ("option.Every other refresh", "Un rafraîchissement sur deux"),
    ("option.Global", "Global"),
    ("option.Fit", "Fit (bords masqués)"),
    ("option.Resize", "Resize (agrandi)"),
    ("option.Default", "Par défaut"),
    ("option.Color", "Couleur"),
    ("option.Gradient", "Dégradé"),
    ("option.Fantasy", "Fantaisie"),
    ("option.None", "Aucun"),
    // RO-5 partie 1 : voir le commentaire de `english.rs`. Les deux noms sont les mêmes dans les
    // deux langues, un niveau de multi-échantillonnage n'étant pas un mot. Q-107 §7.
    ("option.FFAA", "MSAA 4×"),
    ("option.SMAA", "MSAA 8×"),
    // AMD-060 §2 : le quatrième niveau, « sous un libellé propre […] sans toucher aux trois
    // libellés existants ». C'est le seul des quatre qui soit vraiment du SMAA, donc le libellé dit
    // de quel SMAA il s'agit plutôt que combien d'échantillons il prend — il en prend un.
    ("option.SMAA_DEPTH", "SMAA (profondeur)"),
    ("option.Native", "Native"),
    ("option.enabled", "activées"),
    ("option.disabled", "désactivées"),
    ("option.A-Z", "A-Z"),
    ("option.Recent", "Récent"),
    ("option.en", "English"),
    ("option.fr", "Français"),
    // ================================================================= RO-1 point 3
    // L'infobulle détaillée de chaque ligne de la fenêtre d'options du bureau : ce que fait le
    // réglage, ce que valent ses valeurs quand ce n'est pas évident, sa valeur par défaut (et celle
    // des consoles portables quand elle diffère), ce qu'il coûte et de quoi il dépend.
    // `ui::options::tooltip_of` place d'abord la phrase courte `help.*` de l'amendement, quand il y
    // en a une, puis ce paragraphe.
    //
    // Bureau seulement : les pages des consoles et Android n'ont pas de survol.

    // ------------------------------------------------------ onglet Affichage
    (
        "detail.ui.fullscreen",
        "Dessine la fenêtre sur tout l'écran, sans bordure. Alt+Entrée fait la même chose à tout \
         moment, ainsi que Vidéo ▸ Plein écran. Par défaut : désactivé.",
    ),
    (
        "detail.video.vsync",
        "Lie l'image au rafraîchissement de l'écran. Désactivée, chaque image part dès qu'elle est \
         dessinée : le délai le plus court, et l'image peut se déchirer. À chaque rafraîchissement \
         suit l'écran, ce qui supprime le déchirement et plafonne la cadence de rendu à celle de \
         l'écran (60 images par seconde sur une dalle 60 Hz). Un rafraîchissement sur deux divise \
         encore ce chiffre par deux, pour un écran dont le rafraîchissement dépasse largement ce \
         que la machine peut dessiner. Par défaut : désactivée sur le bureau, à chaque \
         rafraîchissement sur une console portable.",
    ),
    (
        "detail.video.frame_rate",
        "Combien de fois par seconde la scène 3D est redessinée. Ce n'est pas la vitesse du jeu : \
         l'émulation garde la cadence de la console (environ 60 images par seconde pour un jeu \
         NTSC, 50 pour un jeu PAL) quel que soit ce choix. Une cadence plus élevée rend la caméra \
         et le défilement plus fluides et coûte du temps GPU en proportion. La synchronisation \
         verticale réglée sur un rafraîchissement la plafonne à la cadence de l'écran. \
         Par défaut : 90.",
    ),
    (
        "detail.video.render_method",
        "Direct dessine la scène 3D directement dans la fenêtre, toujours à la résolution de \
         celle-ci. Render Tex la dessine d'abord dans une texture puis affiche la texture, ce dont \
         l'échelle de résolution et la résolution automatique ont besoin ; cela coûte une copie \
         plein écran par image. Par défaut : Direct, sur les deux plates-formes.",
    ),
    (
        "detail.video.res_scale",
        "Dessine la scène 3D plus petite que la fenêtre puis l'étire : une façon simple d'acheter \
         de la cadence avec de la netteté. 1/2 représente un quart des pixels, 1/4 un seizième. \
         Nécessite la méthode Render Tex — en Direct la scène est toujours dessinée à la résolution \
         de la fenêtre — et la résolution automatique prend le choix à sa place quand elle est \
         active. Par défaut : Native.",
    ),
    (
        "detail.video.dynamic_resolution",
        "Laisse l'échelle de la scène suivre le temps que prend le GPU, entre native et 1/2, au \
         lieu de l'échelle fixe ci-dessus : l'image s'adoucit là où une scène est lourde et \
         redevient nette ailleurs. Nécessite la méthode Render Tex, le moteur OpenGL et un budget \
         d'image connu, c'est-à-dire la synchronisation verticale ou un plafond de cadence. \
         Par défaut : désactivée, sur les deux plates-formes.",
    ),
    (
        "detail.video.anti_alias",
        "Adoucit l'escalier sur les arêtes obliques des formes 3D. Les deux premiers niveaux sont \
         du multi-échantillonnage — 4 et 8 échantillons par pixel — afin que les textures en pixel \
         art restent nettes. 3dSen les nomme FFAA et SMAA, et c'est aussi ce que contiennent le \
         fichier de réglages et l'option libretro ; Zaxnes les affiche pour ce qu'ils sont, MSAA 4× \
         et MSAA 8×. SMAA (profondeur) est un filtre et non des échantillons supplémentaires : la \
         scène est rendue une seule fois et les contours sont cherchés dans la profondeur de \
         l'image, de sorte que les formes sont adoucies sans qu'une texture soit jamais touchée, \
         pour environ le tiers de la mémoire du MSAA 4×. Il laisse le contour de la barre d'état \
         tel quel. Le coût des trois croît avec la résolution. Par défaut : MSAA 4× sur le bureau, \
         Aucun sur une console portable.",
    ),
    (
        "detail.video.skip_frame",
        "Quand l'émulation n'a produit aucune nouvelle image depuis le dernier rendu de la scène, \
         l'image précédente est conservée au lieu de redessiner la même scène. Rien ne change à \
         l'écran — la caméra est la seule exception, aussi désactivez-le si un mouvement de caméra \
         lent doit être plus fluide que la cadence du jeu. Par défaut : activé, sur les deux \
         plates-formes.",
    ),
    (
        "detail.emu.catch_up",
        "Combien d'images émulées une image affichée peut exécuter quand le jeu a pris du retard, \
         afin qu'il rattrape au lieu de ralentir. Aucune ne laisse une image affichée exécuter que \
         les images que le rapport des cadences impose, si bien qu'une machine qui ne suit pas \
         ralentit le jeu. Plus la limite est haute, plus une image en retard peut coûter d'un coup. \
         Par défaut : 2 sur le bureau, 5 sur une console portable.",
    ),
    (
        "detail.emu.catch_up_last_3d",
        "Quand une image affichée exécute plusieurs images émulées, seule la dernière est convertie \
         en 3D : les autres sont bien émulées en entier — le jeu et son son restent exacts — mais \
         elles ne sont pas converties et leurs hooks d'image de script ne sont pas appelés. Cela \
         économise la conversion et le Lua de chaque image ainsi sautée ; un profil dont le script \
         anime quelque chose à chaque image peut saccader. Un profil dont les scripts écrivent dans \
         la console est laissé tel quel : chacune de ses images est convertie. Par défaut : \
         désactivé sur le bureau, activé sur une console portable.",
    ),
    (
        "detail.emu.pause_3d_in_2d",
        "Pendant que l'image 2D plate est à l'écran, rien de la scène 3D ne l'est, donc ni la \
         conversion ni les hooks d'image de script ne sont exécutés ; l'image montrée au retour de \
         la 3D est convertie d'abord. Un profil dont les scripts écrivent dans la console est \
         laissé tel quel. Par défaut : désactivé sur le bureau, activé sur une console portable.",
    ),
    (
        "detail.emu.lua_smooth_gc",
        "Règle le ramasse-miettes des scripts de profil pour qu'il travaille un peu à chaque image \
         au lieu de s'arrêter de temps en temps pour une longue collecte — des collectes de 100 à \
         300 ms ont été mesurées sans cela. La seule différence qu'un script puisse voir est le \
         chiffre que renvoie collectgarbage(\"count\"). Par défaut : activé, sur les deux \
         plates-formes.",
    ),
    (
        "detail.emu.lua_optimized",
        "Fait passer les liaisons de script par des chemins optimisés qui emploient du code unsafe : \
         les valeurs de l'hôte sont atteintes par des métaméthodes C au lieu des enveloppes sûres, \
         pour les mêmes résultats et moins de temps par accès. Désactivé, seuls les chemins sûrs \
         sont employés. Les images en pipeline et l'optimisation expérimentale ci-dessous en ont \
         besoin toutes les deux. Prend effet au prochain chargement de jeu. Par défaut : désactivé \
         sur le bureau, activé sur une console portable.",
    ),
    (
        "detail.emu.pipeline",
        "Exécute les scripts et la conversion 3D d'une image sur un fil d'exécution propre pendant \
         que l'image précédente est dessinée, ce qui affiche l'image un rendu plus tard. Désactivé \
         ne le fait jamais ; Auto ne le fait que lorsque les images sont en retard ; Toujours le \
         fait dès que le jeu le permet. Nécessite le Lua optimisé — une image en pipeline confie le \
         moteur de script à un autre fil — et reste simplement désactivé sans lui. Par défaut : \
         désactivé sur le bureau, Auto sur une console portable.",
    ),
    (
        "detail.emu.lua_experimental",
        "Par-dessus le Lua optimisé : un hook UpdateS dont la source ne lit que des champs de shape \
         reçoit un shape allégé et pas de shape2D. Quels hooks sont dans ce cas est lu dans le \
         script au chargement du profil, et cette lecture est une heuristique — un profil qu'elle \
         juge mal peut mal se comporter, et c'est ce qui rend l'option expérimentale. Prend effet au \
         prochain chargement de jeu. Nécessite le Lua optimisé. Par défaut : désactivé sur le \
         bureau, activé sur une console portable.",
    ),
    (
        "detail.video.backend",
        "Avec quelle API graphique la fenêtre dessine, lue une seule fois au démarrage de Zaxnes. \
         OpenGL 3.3 est la valeur par défaut et le seul choix sur une console portable. Vulkan \
         permet à des outils de post-traitement comme ReShade de lire la profondeur de la scène \
         3D ; DirectX 12 est réservé à Windows et ne fonctionne pas encore avec ReShade. Ni Vulkan \
         ni DirectX 12 ne dessine les effets du style Enhanced, et aucun des deux n'applique la \
         résolution automatique. Par défaut : OpenGL.",
    ),
    (
        "detail.video.max_queued_frames",
        "Combien d'images dessinées le pilote graphique peut encore avoir en chantier avant que \
         Zaxnes attende la plus ancienne. 2 laisse le processeur prendre une image d'avance sur la \
         carte graphique, ce qui est le plus fluide ; 1 attend chaque image, ce qui coûte des \
         images par seconde et raccourcit le délai entre l'appui sur un bouton et l'image. Cela \
         concerne le moteur OpenGL — Vulkan et DirectX 12 ont leur propre file — et sur une console \
         portable rien n'est attendu à 2. Par défaut : 2.",
    ),

    // ----------------------------------------------------- onglet Image NES
    (
        "detail.video.emphasis_3d",
        "Certains jeux teintent ou assombrissent tout l'écran grâce à l'accentuation des couleurs de la console (une pause, un fondu, un éclair, un niveau sous l'eau) : la console affaiblit son signal vidéo sur deux des trois couleurs. L'image plate l'affiche toujours ; ceci l'affiche aussi dans la vue 3D, toute la scène prenant la teinte que le jeu demande. Un jeu qui change l'accentuation au milieu de l'écran donne sa teinte la plus fréquente à toute la scène. Par défaut : désactivé.",
    ),
    (
        "detail.video.composite_signal",
        "Fidélité+ : l'image plate faite comme une télévision la faisait à partir du signal composite de la console, généré pixel par pixel comme la puce vidéo le génère, puis décodé : les couleurs bavent un peu au bord des objets et les franges ondulent d'une image à l'autre, comme sur un téléviseur branché avec le câble jaune. Les couleurs des grandes surfaces sont celles du signal de la console, pas celles de la palette. Coûte un peu de temps processeur par image. La vue 3D n'est pas concernée. Par défaut : désactivé.",
    ),
    (
        "detail.video.nes_palette",
        "Dans quelle table de couleurs RVB les 64 couleurs de la console sont montrées, aussi bien \
         dans l'image plate que dans la vue 3D. La console émet un signal composite, que chaque \
         émulateur décode en une table qui lui est propre ; la première entrée est la table de \
         3dSen, les autres celles des principaux émulateurs et de quelques téléviseurs mesurés. \
         Rien d'autre que les couleurs ne change. Par défaut : Mesen.",
    ),
    (
        "detail.video.remove_sprite_limit",
        "La console ne pouvait dessiner que huit sprites sur une même ligne de balayage, ce qui \
         explique le clignotement de certains jeux : ils montrent leurs sprites une image sur deux \
         pour se partager la ligne. Désactivé garde la limite et le clignotement, Toujours les \
         dessine tous, et Intelligent garde la limite sur une ligne où un jeu empile volontairement \
         huit sprites pour cacher ceux qui sont derrière — ce que fait Mesen. Cela ne change que \
         l'image 2D plate : la vue 3D a tous les sprites quel que soit ce choix. Par défaut : \
         Intelligent.",
    ),
    (
        "detail.video.overscan_mode",
        "Ce que devient la place libérée par les marges d'overscan. Fit laisse à l'image la taille \
         qu'elle avait et ne dessine simplement pas les bords coupés, si bien que le fond de la \
         scène apparaît là. Resize agrandit ce qui reste jusqu'à remplir de nouveau la vue, en 2D. \
         Par défaut : Resize.",
    ),
    (
        "detail.video.overscan_ntsc_top",
        "Combien de rangées de pixels sont coupées en haut de l'image d'un jeu NTSC. Un téléviseur \
         masquait les rangées extrêmes, et beaucoup de jeux y laissent des déchets de défilement. \
         De 0 à 32 pixels ; 8 est la valeur usuelle pour le haut et le bas. Ce qui arrive à la place \
         libérée est réglé par la ligne Overscan ci-dessus. Un jeu peut régler ses quatre marges \
         dans ses options, qui l'emportent alors sur celles-ci. Par défaut : 0 — toute l'image.",
    ),
    (
        "detail.video.overscan_ntsc_bottom",
        "Combien de rangées de pixels sont coupées en bas de l'image d'un jeu NTSC. Un téléviseur \
         masquait les rangées extrêmes, et beaucoup de jeux y laissent des déchets de défilement. \
         De 0 à 32 pixels ; 8 est la valeur usuelle pour le haut et le bas. Un jeu peut régler ses \
         quatre marges dans ses options, qui l'emportent alors sur celles-ci. Par défaut : 0 — \
         toute l'image.",
    ),
    (
        "detail.video.overscan_ntsc_left",
        "Combien de colonnes de pixels sont coupées à gauche de l'image d'un jeu NTSC. De 0 à 32 \
         pixels ; les côtés demandent en général moins que le haut et le bas. Une marge d'écran \
         large n'est jamais coupée : l'image supplémentaire continue au-delà de ce bord. Un jeu \
         peut régler ses quatre marges dans ses options, qui l'emportent alors sur celles-ci. \
         Par défaut : 0 — toute l'image.",
    ),
    (
        "detail.video.overscan_ntsc_right",
        "Combien de colonnes de pixels sont coupées à droite de l'image d'un jeu NTSC. De 0 à 32 \
         pixels ; les côtés demandent en général moins que le haut et le bas. Une marge d'écran \
         large n'est jamais coupée : l'image supplémentaire continue au-delà de ce bord. Un jeu \
         peut régler ses quatre marges dans ses options, qui l'emportent alors sur celles-ci. \
         Par défaut : 0 — toute l'image.",
    ),
    (
        "detail.video.overscan_pal_top",
        "Combien de rangées de pixels sont coupées en haut de l'image d'un jeu PAL. Un téléviseur \
         PAL montrait davantage de l'image qu'un téléviseur NTSC, aussi ces quatre marges sont-elles \
         tenues à part de celles du NTSC ; laquelle des deux paires sert suit la région vidéo de \
         l'onglet Émulation. De 0 à 32 pixels. Par défaut : 0 — toute l'image.",
    ),
    (
        "detail.video.overscan_pal_bottom",
        "Combien de rangées de pixels sont coupées en bas de l'image d'un jeu PAL. Un téléviseur \
         PAL montrait davantage de l'image qu'un téléviseur NTSC, aussi ces quatre marges sont-elles \
         tenues à part de celles du NTSC ; laquelle des deux paires sert suit la région vidéo de \
         l'onglet Émulation. De 0 à 32 pixels. Par défaut : 0 — toute l'image.",
    ),
    (
        "detail.video.overscan_pal_left",
        "Combien de colonnes de pixels sont coupées à gauche de l'image d'un jeu PAL. Laquelle des \
         deux paires de marges sert suit la région vidéo de l'onglet Émulation. Une marge d'écran \
         large n'est jamais coupée : l'image supplémentaire continue au-delà de ce bord. De 0 à 32 \
         pixels. Par défaut : 0 — toute l'image.",
    ),
    (
        "detail.video.overscan_pal_right",
        "Combien de colonnes de pixels sont coupées à droite de l'image d'un jeu PAL. Laquelle des \
         deux paires de marges sert suit la région vidéo de l'onglet Émulation. Une marge d'écran \
         large n'est jamais coupée : l'image supplémentaire continue au-delà de ce bord. De 0 à 32 \
         pixels. Par défaut : 0 — toute l'image.",
    ),
    (
        "detail.video.widescreen_strict_edges",
        "Réservé à une ROM corrigée pour l'extension d'écran large. Là où le jeu a cessé de \
         dessiner une bande de l'image élargie, les formes coupées y sont laissées inachevées — \
         dessinées avec le motif temporaire — jusqu'au retour de toute la bande, afin que rien \
         n'apparaisse à moitié construit. C'est désactivé par défaut parce qu'un objet d'une seule \
         tuile posé dans une telle bande resterait temporaire tant qu'il y est. Cela ne fait rien \
         tant que le jeu ne produit pas de marge d'écran large ; l'écran large lui-même se règle \
         pour chaque jeu, dans les options de ce jeu. Par défaut : désactivé.",
    ),

    // ------------------------------------------------------ onglet Rendu 3D
    (
        "detail.scene.skybox",
        "Ce qu'il y a derrière la scène 3D. Default prend la couleur de fond du jeu, et se rabat \
         sur la couleur ci-dessous pour un profil qui la désactive. Color remplit le fond avec la \
         couleur ci-dessous. Gradient et Fantasy sont tirés de la couleur de fond et de la palette \
         du jeu, si bien que la couleur ci-dessous ne les atteint pas. Par défaut : Default.",
    ),
    (
        "detail.scene.custom_bg_color",
        "La couleur derrière la scène lorsque le fond ci-dessus est réglé sur Color, et sous le \
         fond Default pour un profil qui désactive la couleur propre du jeu. Les ciels Gradient et \
         Fantasy sont tirés des couleurs du jeu et l'ignorent. Par défaut : noir.",
    ),
    (
        "detail.render.visual_style",
        "Enhanced est ce dont les groupes ci-dessous ont besoin : la grille de voxels et les reflets \
         ici, les ombres pixel et l'occlusion ambiante dans l'onglet Lumière et ombres. À Classic \
         aucun d'eux n'est compilé, aucune passe supplémentaire ne tourne et aucun tampon \
         supplémentaire n'est alloué : toute la famille ne coûte exactement rien. Les moteurs \
         Vulkan et DirectX 12 ne dessinent pas du tout les effets Enhanced. Par défaut : Classic, \
         sur les deux plates-formes.",
    ),
    (
        "detail.render.style.voxel_grid",
        "Creuse un sillon dans les faces de la scène tous les 2, 4 ou 8 voxels, de sorte qu'une \
         forme se lise comme un empilement de blocs et non comme un bloc plein. Plus l'écart est \
         petit, plus il y a de sillons. Un coût par pixel sur toute l'image. Nécessite le style \
         Enhanced. Par défaut : désactivée.",
    ),
    (
        "detail.render.style.reflections",
        "Reflète la scène sur le sol en marchant dans le tampon de profondeur. C'est l'effet le plus \
         coûteux du style Enhanced, et les quatre lignes en dessous décident à quel point. \
         Nécessite le style Enhanced. Par défaut : désactivés.",
    ),
    (
        "detail.render.style.ssr_shine",
        "De 1 à 5 : quelle part de la scène le sol renvoie, du sol à peine humide au miroir. Cela \
         change l'aspect, pas le coût. Nécessite les reflets. Par défaut : 3.",
    ),
    (
        "detail.render.style.ssr_clarity",
        "De 1 à 5 : plus la valeur est haute, plus un reflet reste net en s'éloignant de l'objet \
         auquel il appartient ; une valeur basse le fond dans le sol. Nécessite les reflets. \
         Par défaut : 3.",
    ),
    (
        "detail.render.style.ssr_length",
        "De 1 à 5 : jusqu'où un reflet s'étend sur le sol. Un reflet plus long demande plus de pas \
         de marche, donc celui-ci coûte. Nécessite les reflets. Par défaut : 3.",
    ),
    (
        "detail.render.style.ssr_quality",
        "Combien de pas la marche fait et combien de fois elle affine ce qu'elle trouve, ce qui \
         décide du nombre de trous dans le reflet : Low vaut 12 pas et 2 affinages, High 48 et 5, \
         Ultra davantage encore. C'est la ligne à baisser en premier sur une petite machine. \
         Nécessite les reflets. Par défaut : High sur le bureau, Low sur une console portable.",
    ),
    (
        "detail.scene.bend_lr",
        "En degrés ; 0 donne une scène plate. Les pas sont de 5 degrés entre −30 et 30, là où une \
         légère courbure vaut d'être réglée finement, et de 30 degrés au-delà, jusqu'à ±180. Tout \
         le niveau est courbé, et la caméra comme la visée du Zapper le suivent. Cela coûte un peu \
         par sommet dès qu'un des trois angles n'est pas 0, et rien du tout quand les trois sont 0. \
         Un script y accède sous le nom spl.bend_zx, et l'écriture d'un script tombe sur les mêmes \
         crans que ceux du menu. Par défaut : 0.",
    ),
    (
        "detail.scene.bend_tb",
        "En degrés ; 0 donne une scène plate. Les pas sont de 5 degrés entre −30 et 30 et de 30 \
         degrés au-delà, jusqu'à ±180. Cela coûte un peu par sommet dès qu'un des trois angles \
         n'est pas 0, et rien du tout quand les trois sont 0. Un script y accède sous le nom \
         spl.bend_zy, et l'écriture d'un script tombe sur les mêmes crans que ceux du menu. \
         Par défaut : 0.",
    ),
    (
        "detail.scene.arc_h",
        "En degrés ; 0 donne une scène plate. Les pas sont de 5 degrés entre −30 et 30 et de 30 \
         degrés au-delà, jusqu'à ±180. La ligne Centre de l'arc ci-dessous choisit autour de quoi \
         le niveau se courbe. Un script y accède sous le nom spl.arc_yx, et l'écriture d'un script \
         tombe sur les mêmes crans que ceux du menu. Par défaut : 0.",
    ),
    (
        "detail.scene.arc_center",
        "La hauteur autour de laquelle l'arc se courbe : le milieu de l'écran, son bord inférieur \
         — huit unités sous l'image — ou l'un des trois quarts entre les deux, qui placent le \
         pivot d'autant plus bas. Cela ne fait rien tant qu'Arc horizontal vaut 0. Un script y \
         accède sous le nom spl.arc_center, où 0 et 1 sont les deux extrêmes, exactement comme \
         dans 3dSen, et une valeur intermédiaire tombe sur le quart le plus proche. \
         Par défaut : centre de l'écran.",
    ),
    (
        "detail.scene.bend_2d_mode",
        "Un réglage propre à Zaxnes : 3dSen courbe toujours le rectangle plat du mode 2D avec la \
         scène, et c'est ainsi que l'on garde ce rectangle plat pendant que la scène 3D reste \
         courbée. Cela ne fait rien en mode 3D, ni quand les trois angles valent 0. Aucun script n'y \
         accède — le programme d'origine n'a pas ce réglage. Par défaut : activé, sur les deux \
         plates-formes.",
    ),
    (
        "detail.render.mesh_algo",
        "Comment les voxels d'une tuile deviennent un maillage. GREEDY fusionne les faces voisines \
         en les plus grands rectangles possibles sans regarder leur couleur, ce qui donne le moins \
         de triangles ; GREEDY_MULTICOLOR ne fusionne que des faces de même couleur ; MARCHING \
         construit une surface lissée par marching cubes, ce qui arrondit les formes et coûte le \
         plus cher. La forme d'un profil peut demander son propre algorithme, qui l'emporte alors \
         sur celui-ci. Par défaut : GREEDY.",
    ),

    // --------------------------------------------- onglet Lumière et ombres
    (
        "detail.light.intensity",
        "La force de l'unique lumière directionnelle de la scène. Le curseur va de 0,5 à 1,5, 1 \
         étant la lumière telle que le profil l'a réglée ; un script ou un profil importé peut en \
         demander davantage, et une valeur de 2 réglée ici survit à un chargement de profil. La \
         direction et la couleur de la lumière appartiennent au profil et n'ont pas de ligne. \
         Par défaut : 1.",
    ),
    (
        "detail.render.shadow",
        "Projette l'ombre des formes de la scène depuis la lumière, au moyen d'une seule carte \
         d'ombres. Désactivées, la passe d'ombres n'est pas exécutée du tout et rien ne reçoit \
         d'ombre, ce qui laisse aussi les ombres pixel ci-dessous sans rien à modifier. Quels \
         calques projettent et lesquels reçoivent appartient au profil. Par défaut : activées sur \
         le bureau, désactivées sur une console portable, où la passe coûte l'image qu'elle vaut.",
    ),
    (
        "detail.render.backdrop_lighting",
        "Seuls certains profils placent un décor derrière la scène — il est allumé par un script, \
         et éteint par défaut — si bien que sur la plupart des jeux cet interrupteur ne montre \
         rien. Là où il y en a un, le lustre fait passer une surface noire à environ sRGB 26 et lui \
         donne la teinte de la lumière. C'est un coût par fragment sur une surface qui peut remplir \
         l'image : gratuit dans le bruit de mesure sur un GPU de bureau en 640 × 480 et 1280 × 720, \
         d'où la valeur par défaut plus économe des consoles portables. Par défaut : activé sur le \
         bureau, désactivé sur une console portable.",
    ),
    (
        "detail.render.style.pixel_shadow",
        "Aligne le bord de l'ombre sur la grille de pixels du jeu au lieu de le laisser tomber là \
         où la géométrie le veut, si bien que l'ombre a l'air dessinée plutôt que calculée. Cela \
         n'ajoute pas d'ombre : cela change la façon de dessiner les ombres que l'interrupteur \
         ci-dessus projette, et n'a rien à faire quand celles-ci sont désactivées. Nécessite le \
         style Enhanced. Par défaut : désactivées.",
    ),
    (
        "detail.render.style.ao",
        "Assombrit les coins et les creux que la lumière n'atteint pas vraiment, ce qui est un \
         effet différent des ombres ci-dessus — il n'a besoin ni d'une direction de lumière ni \
         d'une carte d'ombres. Désactivée, elle ne coûte rien. MSVO est la méthode du programme \
         d'origine et Depth une méthode moins chère qui travaille depuis le tampon de profondeur ; \
         cette version dessine les deux comme Depth, et c'est pourquoi Épaisseur ci-dessous est \
         grisée. Nécessite le style Enhanced. Par défaut : désactivée.",
    ),
    (
        "detail.render.style.ao_strength",
        "De 1 à 5 : à quel point le coin le plus sombre s'assombrit. Cela change l'aspect, pas le \
         coût. Nécessite l'occlusion ambiante. Par défaut : 3.",
    ),
    (
        "detail.render.style.ao_radius",
        "De 1 à 5 : jusqu'à quelle distance d'un coin l'assombrissement porte encore. Un rayon plus \
         grand étale les mêmes échantillons plus loin, si bien que l'assombrissement devient plus \
         grossier plutôt que plus coûteux. Nécessite l'occlusion ambiante. Par défaut : 3.",
    ),
    (
        "detail.render.style.ao_thickness",
        "De 1 à 5 : quelle épaisseur on suppose à une surface pour décider si elle masque ce qui \
         est derrière elle. Seule la méthode MSVO s'en sert, et cette version dessine MSVO comme \
         Depth : la ligne est donc grisée, la valeur étant tout de même conservée et enregistrée. \
         Par défaut : 3.",
    ),
    (
        "detail.render.style.ao_quality",
        "Combien d'échantillons chaque pixel prend et à quelle résolution, ce qui est le coût réel \
         de l'occlusion : Low vaut 6 échantillons en demi-résolution, Medium 10. C'est la ligne à \
         baisser en premier sur une petite machine. Nécessite l'occlusion ambiante. Par défaut : \
         Medium sur le bureau, Low sur une console portable.",
    ),

    // ---------------------------------------------------------- onglet Caméra
    (
        "detail.camera.fov",
        "Le champ de vision de la caméra 3D, en degrés : un angle faible aplatit la scène, un angle \
         large exagère sa profondeur. Le curseur va de 45 à 95 par pas de 5 ; un script peut \
         demander n'importe quoi entre 10 et 150. Le changer ne joue pas le son de validation, la \
         valeur bougeant continûment sous la souris. Par défaut : 70.",
    ),
    (
        "detail.camera.frame_picture",
        "La vue 3D, au début d'une partie et quand on la recentre, cadre l'image comme la vue 2D : \
         passer de la 2D à la 3D ne fait plus reculer la caméra. Désactivé : le cadrage de 3dSen, \
         l'image sur environ les neuf dixièmes de la hauteur. Par défaut : activé.",
    ),

    // ----------------------------------------------------------- onglet Audio
    (
        "detail.audio.volume",
        "Le gain appliqué à tout ce que Zaxnes joue, de 0,00 à 1,00. Muet, dans le menu Audio, \
         coupe le son sans toucher à cette valeur, si bien que désactiver Muet rend le même volume. \
         Le changer ne joue pas le son de validation. Par défaut : 1,00.",
    ),
    (
        "detail.audio.latency_ms",
        "La quantité de son tenue en réserve devant le périphérique, de 20 à 200 millisecondes. Une \
         valeur élevée encaisse un à-coup plus long du fil principal sans craquement et retarde \
         d'autant le son d'un appui sur un bouton ; une valeur basse est plus immédiate et moins \
         indulgente. Au-delà de ce que l'anneau d'échantillons du cœur peut contenir — environ \
         138 ms à 48 kHz — elle est bornée et le journal le dit. La changer vide la réserve et la \
         remplit de nouveau, ce qui fait un bref creux. Elle ne concerne que la sortie par \
         défaut : la sortie classique ci-dessous a sa propre réserve fixe, et la ligne est grisée \
         tant qu'elle est active. Par défaut : 60 ms.",
    ),
    (
        "detail.audio.classic_output",
        "Rétablit la sortie audio que Zaxnes avait en version 0.4.2 : une réserve de sortie de \
         quatre blocs de 512 trames, environ 43 ms, réalimentée par le fil principal une fois \
         par image dessinée. À utiliser si la nouvelle sortie se comporte mal sur cet appareil. \
         C'est la plus courte des deux réserves, et elle émet un bloc de silence chaque fois que \
         l'émulation est en retard sur ses échantillons : elle peut donc craquer — ce que la \
         nouvelle sortie a justement été écrite pour empêcher. Le réglage de latence de sortie et \
         les compteurs de latence, de correction et de ruptures du panneau de performance \
         appartiennent à la nouvelle sortie et ne font rien ici. Le choix prend effet à la \
         réouverture du périphérique audio, ce que cocher cette case fait aussitôt. Par défaut : \
         désactivé.",
    ),

    // -------------------------------------------------------- onglet Émulation
    (
        "detail.emu.game_speed",
        "Fait tourner le jeu plus vite ou plus lentement, 0 étant sa vitesse normale. L'échelle \
         n'est pas symétrique : le facteur vaut 1 + s/4 au-dessus de 0 et 1 / (1 − s/4) en dessous, \
         si bien que +8 va trois fois plus vite et −8 trois fois moins. Le son suit. Il n'y a pas de \
         mode ralenti séparé — la moitié négative en tient lieu — et le raccourci Avance rapide \
         emploie toujours son propre +2 (une fois et demie), quoi qu'il y ait ici. La valeur revient \
         à 0 à la fin d'une partie. Le changer ne joue pas le son de validation. Par défaut : 0.",
    ),
    (
        "detail.emu.run_ahead",
        "Masque le délai d'entrée de la console en émulant une, deux ou trois images au-delà de \
         celle affichée et en montrant la dernière, si bien qu'un appui sur un bouton agit plus \
         tôt. Chaque image demandée coûte une image émulée de plus, plus un état sauvegardé et \
         rechargé, à chaque image affichée — d'où la désactivation sur une console portable. Un jeu \
         dont les scripts de profil écrivent dans la console est maintenu à 0 pendant qu'il tourne, \
         et la ligne l'indique à côté. Par défaut : 1 image sur le bureau, désactivé sur une \
         console portable.",
    ),
    (
        "detail.emu.smooth_scroll_ahead",
        "Expérimental. Sur un affichage plus rapide que la console, dessine la partie qui défile \
         de l'image entre deux images émulées : l'image est montrée ramenée au défilement que \
         l'image réelle du run-ahead a dessiné, puis glisse vers le sien au fil de l'image, ce qui \
         dépense une des images de latence du run-ahead en un mouvement plus fluide. Les barres \
         d'état et les sprites qui suivent la caméra restent où ils sont. Sans run-ahead ou à 60 \
         images par seconde ou moins, rien ne se passe. Un jeu dont le pack de hacks rapporte sa \
         propre caméra utilise celle-ci à la place. La scène 3D ne suit qu'avec le réglage \
         suivant. Par défaut : activé, quel que soit le préréglage.",
    ),
    (
        "detail.emu.smooth_scroll_3d",
        "Expérimental. Le défilement lissé, qu'il vienne du run-ahead ou d'un pack de hacks qui \
         rapporte sa caméra, fait aussi glisser la partie qui défile de la scène 3D, pas \
         seulement de l'image 2D. La 3D est alors dessinée au rythme de l'écran (120 ou 144 fois \
         par seconde au lieu de 60), ce qui coûte sur les machines modestes. À 60 images par \
         seconde ou moins, rien ne se passe. Par défaut : désactivé, quel que soit le \
         préréglage.",
    ),
    (
        "detail.emu.smooth_scroll_view",
        "Une vue de mise au point du réglage ci-dessus : la bande de lignes que le défilement \
         lissé fait glisser est teintée en rouge et les sprites qu'il laisse immobiles en vert, de \
         sorte qu'on voit d'un coup d'œil ce qu'il déplace et ce qu'il laisse en place. Rien n'est \
         teinté tant que le défilement lissé ne fait rien. Par défaut : désactivé.",
    ),
    (
        "detail.emu.auto_tv_mode",
        "Lit la région dans l'en-tête de la ROM au chargement d'un jeu et l'écrit dans la ligne \
         ci-dessous, de sorte qu'un dump européen tourne de lui-même à 50 Hz. C'est une valeur par \
         défaut propre à Zaxnes — 3dSen l'a désactivée — et la désactiver est ce qui rend la ligne \
         ci-dessous à votre main. Prend effet au prochain chargement de ROM. Par défaut : activée.",
    ),
    (
        "detail.emu.tv_mode",
        "En quelle console le jeu est émulé. NTSC tourne à environ 60 images par seconde et PAL à \
         50, avec le minutage et le son qui vont avec ; MULTI est exécuté comme NTSC ; Dendy est le \
         clone soviétique, qui tourne à 50 avec un minutage qui lui est propre. Cela choisit aussi \
         laquelle des deux séries de marges d'overscan s'applique. C'est un réglage de profil : \
         charger un profil l'écrase, et la détection automatique ci-dessus aussi tant qu'elle est \
         active.",
    ),
    (
        "detail.input.emulated_menu_button",
        "Un réglage de 3dSen que Zaxnes conserve afin qu'un fichier de réglages et un script écrits \
         pour 3dSen relisent exactement ce qu'ils ont écrit. Rien dans Zaxnes n'en tient compte : \
         l'interface des consoles atteint son menu en maintenant Select et Start, ce qui n'est pas \
         ce réglage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.core",
        "Quel cœur d'émulation exécute le jeu, dans une version qui en aurait plusieurs. Celle-ci \
         n'en a qu'un, si bien que la valeur est prise modulo 1 : tout ce qu'on écrit ici se relit \
         comme 0 et rien ne s'en sert. La ligne est conservée parce que la clé fait partie du \
         fichier de réglages et de la surface de script (Setting.Core). Par défaut : 0.",
    ),
    (
        "detail.emu.cartridge_db",
        "Complète ce qu'un ancien en-tête iNES ne peut pas dire d'une cartouche, d'après une base d'environ 10 000 dumps : la révision de la puce (une MMC3A ou une MMC1A, par exemple), le sous-mappeur, la vraie RAM de sauvegarde, la pile, la région. Elle ne change jamais le numéro de mappeur et ne touche jamais un en-tête NES 2.0. Prend effet au prochain chargement. Par défaut : activée.",
    ),
    (
        "detail.emu.oam_corruption",
        "Un défaut de la vraie console : couper l'image au milieu d'une ligne abîme une rangée de la mémoire des sprites quand l'image revient, et quelques sprites peuvent manquer pendant une image. Les jeux écrits pour la console l'évitent ou s'en accommodent. Le modèle suit des mesures faites sur une console (AccuracyCoin) ; avec le démarrage en haut de l'image, il passe tous les programmes de test connus. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.ppu_bus_conflicts",
        "Un défaut de la vraie console : un jeu qui lit la mémoire vidéo pendant que l'image est dessinée entre en collision avec ce que la puce vidéo lit à cet instant, et la valeur obtenue, ou une tuile du décor, vient du mauvais endroit. La plupart des jeux l'évitent ; Super Mario Bros. 3 le montre sur la ligne au-dessus du sol de son écran titre, comme la console. Des programmes de test le vérifient. Le délai d'une nouvelle adresse vidéo a son propre réglage. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.oamaddr_bug",
        "Un défaut de la puce vidéo de la NES et de la Famicom (2C02G et suivantes) : si un jeu laisse l'adresse de la mémoire des sprites à 8 ou plus quand l'image commence, huit octets de cette mémoire sont recopiés sur les huit premiers, et un ou deux sprites peuvent en remplacer d'autres pendant une image. Les jeux écrits pour la console remettent cette adresse à zéro et ne le montrent jamais. Une console PAL n'a pas ce défaut : il ne s'y applique jamais. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.vs_dip_1",
        "Interrupteur 1 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.vs_dip_2",
        "Interrupteur 2 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.vs_dip_3",
        "Interrupteur 3 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.vs_dip_4",
        "Interrupteur 4 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.vs_dip_5",
        "Interrupteur 5 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.vs_dip_6",
        "Interrupteur 6 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.vs_dip_7",
        "Interrupteur 7 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.vs_dip_8",
        "Interrupteur 8 des huit DIP switches d'une borne d'arcade VS. System. Chaque jeu leur donne son propre sens : difficulté, nombre de vies, pièces par crédit, parfois la langue ou un mode de test ; le manuel du jeu ou la fiche de l'exploitant les liste. Seulement pour les jeux VS. System. Prend effet au prochain chargement, comme sur une borne, qui les lit à l'allumage. Par défaut : désactivé.",
    ),
    (
        "detail.emu.rainbow_network",
        "Permet aux jeux de la carte Rainbow (mapper 682, Broke Studio) d'utiliser votre réseau par la puce Wi-Fi de la carte : connexion à leur serveur de jeu et téléchargement de fichiers. Désactivé, la puce fonctionne toujours (fichiers, nombres aléatoires) mais n'annonce aucun Wi-Fi, et rien ne sort de l'émulateur. Les sauvegardes d'état, le retour en arrière et les replays ne peuvent pas rétablir une connexion. Prend effet au prochain chargement. Par défaut : désactivé.",
    ),
    (
        "detail.emu.oam_decay",
        "La console garde ses sprites dans une mémoire qui s'efface quand l'image n'est pas dessinée : si un jeu laisse l'écran éteint assez longtemps (environ 8 images ici, cela varie d'une console à l'autre) puis affiche des sprites sans les renvoyer, ils reviennent brouillés. Les jeux écrits pour la console renvoient leurs sprites à chaque image et ne le montrent jamais. Le run-ahead et le retour en arrière relancent ce délai à chaque image : avec eux, l'effacement n'arrive jamais. Prend effet au prochain chargement. Par défaut : désactivé.",
    ),
    (
        "detail.emu.power_on_top",
        "L'endroit où la puce vidéo se trouve dans son image quand on allume la console. La vraie console commence en haut de l'image ; Zaxnes a toujours commencé à la fin, ce qui raccourcit d'une fraction d'image l'attente d'un jeu avant sa première image. Activé, les jeux démarrent comme sur la console, une fraction d'image plus tard qu'avec les anciennes versions de Zaxnes : les enregistrements et les sauvegardes d'état faites juste après l'allumage avec une ancienne version peuvent se décaler. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.ppu_2006_delay",
        "Quand un jeu écrit une nouvelle adresse vidéo pendant l'image, la console ne la prend que trois points plus tard, au milieu d'une lecture, qui peut alors lire brièvement une tuile à une adresse mélangée. Mesuré sur le schéma de la console au niveau des transistors. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.ppu_2006_glitch",
        "Un défaut de la puce vidéo : si cette nouvelle adresse arrive pile au point où la puce passe à la tuile suivante, les deux se mélangent et la position de défilement devient un mélange des deux, jusqu'à ce que le jeu la redonne. Les jeux écrits pour la console l'évitent. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.cpu_ppu_alignment",
        "À l'allumage, le processeur et la puce vidéo de la console calent leurs horloges de l'une de quatre façons (numérotées de 0 à 3, comme la cartouche de test AccuracyCoin), qui change d'un allumage à l'autre. Quelques défauts en dépendent : le moment où arrive une lecture de la mémoire vidéo faite pendant l'image, le glitch de bus ouvert des écritures $2000 et $2005 (alignement 3), le pixel de bord d'une image coupée en milieu de ligne, et l'altération de la mémoire des sprites, qu'une console en alignement 1 ou 2 ne montre pas quand l'image est rallumée en cours d'écran. Les jeux écrits pour la console fonctionnent dans les quatre. L'alignement 3 est celui que Zaxnes a toujours suivi ; Aléatoire en tire un à chaque chargement, comme une console à chaque allumage. Prend effet au prochain chargement. Par défaut : Alignement 3.",
    ),
    (
        "detail.emu.cpu_apu_parity",
        "À l'allumage, le processeur de la console et son unité sonore commencent leur rythme à deux cycles sur l'un ou l'autre cycle, et la minuterie du canal d'échantillons démarre quelque part dans son compte. Cela décide si un transfert de sprites dure 513 ou 514 cycles et où la lecture du canal d'échantillons interrompt une lecture. Zaxnes est le départ qu'il a toujours pris ; Netlist du 2A03 celui du modèle au transistor près de la puce (avec lui le processeur suit ce modèle cycle pour cycle) ; Aléatoire en tire un à chaque chargement, comme une console à chaque allumage. Les jeux écrits pour la console fonctionnent avec tous. Prend effet au prochain chargement. Par défaut : Zaxnes.",
    ),
    (
        "detail.emu.ppu_2000_glitch",
        "Un défaut de la puce vidéo : une écriture dans son registre de contrôle qui commence sur un point précis d'une ligne peut faire dessiner la ligne suivante depuis la moitié gauche de l'écran, et une écriture dans son registre de défilement qui tombe sur la recopie du défilement peut décaler une ligne ou une image de ce qui était sur le bus de données. Cela n'arrive que sur certaines consoles, selon l'alignement des puces à l'allumage. Les jeux écrits pour la console l'évitent. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.exact_rate",
        "Fait tourner les jeux à la cadence propre de la console, 60,0988 images par seconde sur une console NTSC et 50,0070 sur une PAL, au lieu de 60 ou 50 tout rond, et le son avec, à sa vraie hauteur. Sur un écran 60 Hz, une image est montrée deux fois environ toutes les dix secondes ; un écran à fréquence variable les montre toutes. Prend effet tout de suite. Par défaut : désactivé.",
    ),
    (
        "detail.emu.late_present",
        "Avec la manette en temps réel, le temps qui reste avant le rafraîchissement de l'écran sert d'abord à faire avancer la console, et l'image est dessinée juste avant le rafraîchissement, d'après la console à cet instant. Sans elle, l'image est dessinée d'abord et montrée au rafraîchissement suivant, presque un rafraîchissement plus vieille que la console. Un appui apparaît à l'écran jusqu'à un rafraîchissement plus tôt (environ 10 ms à 60 Hz). Demande un ordinateur qui dessine une image bien en deçà d'un rafraîchissement ; si le dessin est en retard, un rafraîchissement est manqué. Seulement avec la manette en temps réel activée et un rafraîchissement à attendre. Prend effet tout de suite. Par défaut : désactivé.",
    ),
    (
        "detail.emu.realtime_input",
        "Lit la manette environ toutes les millisecondes et fait avancer la console par tranches de temps réel entre deux rafraîchissements de l'écran, au lieu de lire la manette une fois par rafraîchissement et de calculer une image entière d'un coup : le jeu voit un bouton environ 9 ms plus tôt en moyenne (jusqu'à 16 ms), et le lit au moment où il le demande, comme sur la console. Pas utilisé avec le run-ahead ni pendant l'enregistrement ou la lecture d'un film : ceux-ci gardent des images entières. Prend effet tout de suite. Par défaut : activé."
    ),
    (
        "detail.emu.fast_core",
        "Fait tourner la console plusieurs fois plus vite sur les cartes commerciales courantes (NROM, MMC1, UxROM, CNROM, AxROM, MMC3) : la puce vidéo dessine une ligne huit points à la fois, les cycles du processeur sont comptés et remis aux puces d'un coup, les boucles d'attente du jeu sont comptées au lieu d'être exécutées. La même console, les mêmes images et les mêmes sons ; les autres cartes tournent comme sans l'option. Pensé pour les consoles portables. Fidélité+ prend le dessus quand les deux sont activés. Prend effet tout de suite. Par défaut : désactivé.",
    ),
    (
        "detail.emu.fidelity_plus",
        "Fait tourner la console dans la machine Fidélité+ : les effets des accès du jeu à la puce vidéo tombent sur les fronts d'horloge mesurés sur le modèle au transistor près des puces, au lieu des délais du cœur standard. Au plus près de la console, plus lent. En construction : consoles NTSC seulement pour l'instant (PAL et Dendy tournent comme sans l'option). Prend effet tout de suite. Par défaut : désactivé.",
    ),
    (
        "detail.emu.ppu_reset_line",
        "Sur la NES de salon, la ligne de reset arrive aussi à la puce vidéo : pendant environ une demi-image après l'allumage ou un appui sur Reset, la puce ignore les écritures du jeu dans ses registres de contrôle, de masque, de défilement et d'adresse, et le bouton Reset ramène l'image à sa première ligne. Désactivé : la NES « top-loader » et la Famicom, dont la puce vidéo ne voit pas le bouton. Les jeux écrits pour la console l'attendent, mais certains homebrews non, et restent alors sur un écran vide. Prend effet au prochain chargement. Par défaut : désactivé.",
    ),
    (
        "detail.emu.palette_corruption",
        "Un défaut de la puce vidéo : couper l'image au milieu d'une ligne, ou faire sortir l'adresse vidéo de la palette, peut changer certaines couleurs de la palette. Modélisé seulement pour les consoles en alignement 2, celui où il a été observé de façon fiable, d'après les mesures de l'auteur d'AccuracyCoin ; les couleurs exactes sont une approximation. Les jeux écrits pour la console l'évitent. Prend effet au prochain chargement. Par défaut : activé.",
    ),
    (
        "detail.emu.cpu_revision",
        "Quelle révision du processeur de la console, qui fait aussi son son. Les RP2A03G et suivants, par défaut, équipent presque toutes les consoles ; le RP2A03 sans lettre des premières Famicom a un canal de bruit plus simple : pas de mode court métallique, et une note la plus grave différente. Seuls les bruitages de certains jeux changent. Prend effet au prochain chargement. Par défaut : RP2A03G et suivants.",
    ),
    (
        "detail.emu.mmc3_revision",
        "Quelle révision de la puce MMC3 contient une cartouche qui l'utilise. Les deux comptent les lignes de l'image un peu différemment quand un jeu met le compte à zéro : le MMC3A et le MMC6 interrompent une fois, le MMC3B et le MMC3C à chaque ligne. Celle de la cartouche, par défaut, prend ce que disent l'en-tête de la ROM et la base de cartouches, c'est-à-dire le MMC3B / MMC3C pour presque tous les jeux. Ne choisissez une révision que pour un programme de test écrit pour elle. Prend effet au prochain chargement. Par défaut : celle de la cartouche.",
    ),
    (
        "detail.emu.ppu_revision",
        "La révision de la puce vidéo de la console. Le 2C02G, par défaut et le plus courant dans les consoles plus récentes, permet à un jeu de relire la palette de couleurs ; les révisions d'avant ne le peuvent pas, et une lecture y rend ce que la puce avait lu avant, comme pour le reste de sa mémoire. Les jeux écrits pour la console ne relisent pas la palette. Prend effet au prochain chargement. Par défaut : 2C02G.",
    ),
    (
        "detail.emu.oamaddr_write_corruption",
        "Un défaut signalé pour la puce vidéo 2C02G : régler l'adresse de la mémoire des sprites recopie une rangée de huit octets de sprites sur la rangée visée. Plusieurs cartouches de test vérifiées sur console passent sans lui, il est donc désactivé ; activez-le pour imiter une console qui le montre. Prend effet au prochain chargement. Par défaut : désactivé.",
    ),
    (
        "detail.emu.power_on_ppu_ram",
        "Ce que contiennent les mémoires propres de la puce vidéo (palette de couleurs, table des sprites et mémoire de fond) à l'allumage. Comme la RAM de travail, elles démarrent avec ce que leurs cellules ont pris, ce qui change d'une console à l'autre. Zéros, par défaut, est ce que faisait toujours cet émulateur ; Console de blargg est la palette qu'attend une ROM de test connue (mesurée sur une console) ; Aléatoire change à chaque allumage. Seules les premières images, avant que le jeu ne pose ses couleurs, peuvent le montrer. Prend effet au prochain chargement. Par défaut : Zéros.",
    ),
    (
        "detail.emu.power_on_ram",
        "Le contenu des 2 Kio de RAM de travail de la console à l'allumage. Une vraie NES démarre avec ce que sa mémoire contient au hasard ; la plupart des jeux l'effacent d'abord, quelques-uns la lisent avant. Le motif FCEUX (00 00 00 00 FF FF FF FF), par défaut, est celui de FCEUX et de BizHawk, dont quelques jeux ont besoin ; Zéros est celui de Mesen ; Aléatoire change à chaque allumage. Prend effet au prochain chargement. Par défaut : motif FCEUX.",
    ),
    (
        "detail.emu.engine",
        "Quel émulateur exécute ce jeu : le cœur de Zaxnes, FCEUX, ou NesHawk. C'est un choix par \
         jeu et non une préférence, parce que la raison de passer à un autre moteur est toujours \
         un jeu précis. FCEUX est là pour la couverture — un jeu que Zaxnes exécute mal, ou dont \
         la carte lui manque. NesHawk est le portage en Rust du cœur de BizHawk, et c'est lui qui \
         fait rejouer un film de BizHawk à l'identique : un .bk2 enregistré sur NesHawk s'y \
         rejoue trame pour trame, ce qu'aucun autre moteur ne peut promettre. Il prend effet au \
         prochain chargement, changer de moteur voulant dire redémarrer la machine. Ni FCEUX ni \
         NesHawk n'a de panoramique, de pack de hack, de réglage de la limite de sprites ni \
         d'avance de trames : ce sont des pièces de la machine de Zaxnes, et leurs lignes le \
         disent tant qu'un autre moteur est choisi. Une sauvegarde d'état faite sur un moteur ne \
         se charge pas sur un autre. Par défaut : Zaxnes.",
    ),

    // -------------------------------------------------------- onglet Interface
    (
        "detail.ui.language",
        "La langue de toute l'interface — les menus, les fenêtres, les notifications et cette \
         infobulle. Le changement s'applique aussitôt, sans redémarrage, et il est conservé pour le \
         prochain lancement. Un texte qui manque à une traduction se rabat sur l'anglais. Le journal \
         de la console et la ligne de commande restent en anglais quel que soit ce choix. \
         Par défaut : la langue du système sur un dossier utilisateur neuf, l'anglais quand ce n'est \
         pas le français.",
    ),
    (
        "detail.ui.roms_folder",
        "Dans quel dossier le mode console liste les jeux. Vide signifie le dossier roms à côté de \
         vos données Zaxnes, ce que le champ montre en gris, et l'option --roms de la ligne de \
         commande l'emporte sur les deux. Il est écrit quand vous appuyez sur Entrée ou quittez le \
         champ ; le bouton à côté ouvre un sélecteur de dossier. Rien d'autre dans Zaxnes ne le lit.",
    ),
    (
        "detail.ui.sort_option",
        "Dans quel ordre la bibliothèque liste les jeux : A-Z par titre, Récents d'abord par la \
         dernière partie jouée, ou Favoris d'abord — les jeux marqués comme favoris en tête, chaque \
         groupe ensuite par titre. Par défaut : A-Z.",
    ),
    (
        "detail.ui.key_layout",
        "Les touches qui pilotent le programme. RetroArch : ses touches par défaut partout où          Zaxnes a la même fonction (F1 menu, F2 et F4 sauvegardent et chargent le slot courant,          F6 et F7 le choisissent, L avance rapide, R retour arrière, H reset, F plein écran), la          caméra sur le pavé numérique, ou Ctrl droit avec les flèches, Page préc. et Page suiv.,          Origine. Zaxnes : les touches des versions précédentes (caméra sur F5 à F10, une touche          par slot). Les touches de jeu que vous avez choisies vous-même sont gardées. Par défaut :          RetroArch.",
    ),
    (
        "detail.ui.escape_quits",
        "Avec la disposition RetroArch, Échap en jeu avertit, et un second Échap dans les deux          secondes quitte le programme. Désactivé, Échap ne fait rien en jeu ; Ctrl+Q quitte          toujours. Par défaut : activé.",
    ),
    (
        "detail.ui.show_fps",
        "Affiche un compteur d'images dans un coin de l'image : les images affichées par seconde, \
         c'est-à-dire la cadence de rendu et non la vitesse du jeu. Ne coûte rien de mesurable. \
         Par défaut : désactivé.",
    ),
    (
        "detail.ui.profiler",
        "Mesure ce que coûte chaque étape d'une image — le cœur, la conversion, les scripts, la \
         liste de dessin, le rendu — et écrit les chiffres à côté du compteur d'images et dans \
         logs/zaxnes.log toutes les deux secondes. C'est un outil de diagnostic, pas un réglage de \
         confort : la mesure elle-même prend un peu de temps et le journal grossit tant qu'elle est \
         active. Par défaut : désactivé.",
    ),
    (
        "detail.ui.editor_mode",
        "Active ce qu'il faut pour écrire un profil 3D avec un agent IA : le dossier editor de vos \
         données utilisateur est installé avec ses documents et ses modèles, un serveur MCP écoute \
         sur la boucle locale pour que l'agent puisse observer le jeu en cours, et les stylos \
         permettent de désigner un motif à l'écran. Ne change rien à la façon dont un jeu se joue \
         ou s'affiche : c'est pour fabriquer des profils, pas pour jouer. Voir docs/EDITOR.md. \
         Par défaut : désactivé.",
    ),

    // ------------------------------------------------- onglet RetroAchievements
    (
        "detail.achievements.enabled",
        "Active RetroAchievements : la ROM de chaque jeu que vous lancez est identifiée auprès du \
         serveur, ses succès sont récupérés, et les déblocages sont envoyés au fur et à mesure. \
         Zaxnes ne joue qu'en softcore : les sauvegardes d'état, le retour en arrière et l'avance \
         rapide restent permis, et les déblocages comptent comme softcore. Rien ne se passe avant \
         que vous ne soyez connecté, ci-dessus. Les quatre lignes ci-dessous ne font rien tant que \
         ceci est désactivé. Par défaut : désactivé.",
    ),
    (
        "detail.achievements.sound",
        "Joue un son bref au déblocage d'un succès, par-dessus la fenêtre qui l'annonce. Nécessite \
         RetroAchievements. Par défaut : activé.",
    ),
    (
        "detail.achievements.challenges",
        "Affiche une petite icône dans un coin de l'image pour chaque défi que le jeu a entamé — un \
         succès chronométré ou conditionnel dont vous êtes au milieu. Nécessite RetroAchievements. \
         Par défaut : désactivé.",
    ),
    (
        "detail.achievements.unofficial",
        "Charge aussi les succès que le site garde comme non officiels : ceux qui sont à l'essai, \
         ou retirés. Ils peuvent se déclencher à tort, et c'est pourquoi ils ne font pas partie \
         d'une partie normale. Lu au chargement d'un jeu, il faut donc recharger pour en changer. \
         Nécessite RetroAchievements. Par défaut : désactivé.",
    ),
    (
        "detail.achievements.encore",
        "Réarme les succès que vous avez déjà débloqués, de sorte qu'ils se déclenchent une seconde \
         fois quand vous rejouez un jeu. Le serveur en est informé comme d'habitude ; rien n'est \
         perdu. Lu au chargement d'un jeu, il faut donc recharger pour en changer. Nécessite \
         RetroAchievements. Par défaut : désactivé.",
    ),

    // ------------------------------------------- handheld interface (Rust-only)
    (keys::HANDHELD_GAMES_TITLE, "Jeux"),
    (keys::HANDHELD_ROMS_EMPTY, "Aucune ROM trouvée. Copiez des fichiers .nes dans {0}"),
    (keys::HANDHELD_TAG_2D, "2D"),
    (keys::HANDHELD_BACK_TO_LIBRARY, "< Retour à la bibliothèque"),
    (keys::HANDHELD_PROFILE_FOR, "Profil pour {0}"),
    (keys::HANDHELD_HINT_PLAY, "Jouer"),
    (keys::HANDHELD_HINT_PROFILE, "Profil 3D"),
    (keys::HANDHELD_HINT_OPTIONS, "Options"),
    (keys::HANDHELD_HINT_BACK, "Retour"),
    (keys::HANDHELD_HINT_JUMP, "A-Z"),
    (keys::HANDHELD_HINT_CHOOSE, "Choisir"),
    (keys::HANDHELD_HINT_CHANGE, "Modifier"),
    (keys::HANDHELD_HINT_REBIND, "Réassigner"),
    (keys::HANDHELD_HINT_CANCEL, "Annuler"),
    (keys::HANDHELD_EXIT_HOLD, "Maintenez Select + Start pour quitter"),
    (keys::HANDHELD_MENU_TITLE, "Menu"),
    (keys::HANDHELD_MENU_RESUME, "Reprendre"),
    (keys::HANDHELD_STATES_SAVE_TITLE, "SAUVEGARDER — choisir un emplacement"),
    (keys::HANDHELD_STATES_LOAD_TITLE, "CHARGER — choisir un emplacement"),
    (keys::HANDHELD_STATES_NO_PICTURE, "Pas d'aperçu"),
    (keys::HANDHELD_HINT_SAVE_HERE, "Sauvegarder ici"),
    (keys::HANDHELD_HINT_LOAD, "Charger"),
    (keys::HANDHELD_HINT_PAGE, "Page"),
    (keys::HANDHELD_GAME_OPTIONS_TITLE, "Options du jeu — {0}"),
    (keys::HANDHELD_GAME_OPTIONS_RESET, "Tout remettre en global"),
    (keys::HANDHELD_USE_GLOBAL, "Use Global : {0}"),
    (keys::HANDHELD_GLOBAL_VALUE, "Global : {0}"),
    (keys::HANDHELD_OWN_COUNT, "{0} propres"),
    (keys::HANDHELD_RUN_AHEAD_FORCED, "(forcé à 0 : scripts)"),
    (keys::HANDHELD_ZAXNES_ONLY, "(moteur Zaxnes uniquement)"),
    (keys::HANDHELD_HINT_GAME_OPTIONS, "Options du jeu"),
    (keys::HANDHELD_HINT_FAVORITE, "Favori"),
    (keys::HANDHELD_HINT_RESET_ALL, "Tout remettre"),
    (keys::HACKS_TITLE, "Hacks — {0}"),
    (keys::HACKS_NONE, "Aucun hack n'est fait pour ce jeu."),
    (keys::HACKS_PATCH_LIVE_NOTE, "Le patch de la ROM suit l'écran large : activé ou désactivé en cours de partie, il bascule tout de suite et le jeu reprend où il en était."),
    (keys::HACKS_PATCH_NOTE, "Le patch de la ROM suit l'écran large au lancement du jeu : activé ou désactivé en cours de partie, il prend effet au prochain lancement."),
    (keys::HACK_WIDESCREEN, "Écran large"),
    (keys::HACK_RATIO, "Ratio"),
    (keys::HACK_MARGIN, "Marge supplémentaire"),
    (keys::HACK_ADVANCED, "Mode avancé (ennemis dans les marges)"),
    (keys::HACK_ON, "activé"),
    (keys::HACK_FORCE, "Forcer l'activation"),
    (keys::HACKS_FORCED_NOTE, "Le hack écran large de {0}, fait pour la ROM « {1} » (CRC {2}), est forcé sur celle-ci : {3} (CRC {4}). Il prend effet au lancement du jeu ; si le jeu ne fonctionne pas avec, désactivez « Forcer l'activation »."),
    (keys::HACKS_FORCE_QUESTION, "Forcer quand même ? Un hack de la ROM proche de l'original a de bonnes chances de fonctionner ; sinon le jeu peut planter."),
    (keys::HACKS_FORCE_BUTTON, "Forcer"),
    (keys::HACKS_PACK_SIGNATURES, "Elle n'a pas non plus le code que le hack modifie, là où il le modifie : le hack ne peut pas s'y appliquer."),
    (keys::HACK_COMPAT, "Compatibilité du hack"),
    (keys::HACK_COMPAT_ORIGINAL, "ROM d'origine"),
    (keys::HACK_COMPAT_FULL, "Compatible"),
    (keys::HACK_COMPAT_PARTIAL, "Partielle"),
    (keys::HACK_COMPAT_NONE, "Incompatible"),
    (keys::HACKS_COMPAT_ORIGINAL, "ROM d'origine : le hack est fait pour elle (CRC {0}). Il s'applique en entier."),
    (keys::HACKS_COMPAT_FULL, "Compatible - {0}/{1}"),
    (keys::HACKS_COMPAT_FULL_BODY, "Pas la ROM d'origine (CRC {0}), mais tout le code que le hack modifie est là."),
    (keys::HACKS_COMPAT_PARTIAL, "Partielle - {0}/{1}"),
    (keys::HACKS_COMPAT_PARTIAL_BODY, "Une partie ne s'applique pas, le jeu y garde son fonctionnement."),
    (keys::HACKS_COMPAT_NONE, "Le hack ne peut pas s'appliquer à cette ROM."),
    (keys::HACKS_COMPAT_MAPPER, "Mapper {0} : le hack est fait pour {1}."),
    (keys::HACKS_COMPAT_REGION, "ROM {0} : ce pack est fait pour une ROM {1}."),
    (keys::HACKS_COMPAT_PART_DIFFERS, "x {0} {1} : code du jeu différent à {2}"),
    (keys::HACKS_COMPAT_PART_NEEDS, "x {0} : dépend d'une partie qui ne s'applique pas"),
    (keys::HACKS_COMPAT_EXPECTED, "ROM attendue : {0}, CRC {1}"),
    (keys::HACKS_FORCE_SIZE, "Sa taille n'est pas celle pour laquelle le hack est fait : il ne peut pas non plus être forcé."),
    (keys::HACKS_OTHER_ROM, "Le hack écran large de {0} est fait pour la ROM « {1} » (CRC {2}). Cette ROM-ci ne correspond pas : {3} (CRC {4}). L'écran large ne peut pas être activé pour elle."),
    (keys::HACK_OFF, "désactivé"),
    (keys::MENU_HACKS, "Hacks…"),
    (keys::MENU_WIDESCREEN_SETTINGS, "Réglages de l'écran large…"),
    (keys::MENU_AIDS, "Hacks"),
    (keys::MENU_CHEATS, "Triches…"),
    (keys::MENU_RAM_SEARCH, "Recherche RAM…"),
    (keys::HANDHELD_MENU_CHEATS, "Triches"),
    (keys::CHEATS_TITLE, "Triches : {0}"),
    (keys::CHEATS_ALL_ON, "Triches activées"),
    (keys::CHEATS_EMPTY, "Aucune triche pour ce jeu : ajoutez-en une ci-dessous, prenez-en dans la base de triches, ou trouvez une adresse avec la recherche RAM."),
    (keys::CHEATS_GENIE_REFUSED, "Ce moteur ignore les codes Game Genie ; les codes RAM fonctionnent."),
    (keys::CHEATS_DELETE, "Supprimer"),
    (keys::CHEATS_ADD_HEADER, "Ajouter une triche"),
    (keys::CHEATS_DESCRIPTION, "Description"),
    (keys::CHEATS_CODES, "Codes"),
    (keys::CHEATS_CODES_HINT, "Game Genie (6 ou 8 lettres), Pro Action Replay (8 chiffres) ou AAAA:VV et AAAA?CC:VV ; plusieurs, joints par +."),
    (keys::CHEATS_ADD, "Ajouter"),
    (keys::CHEATS_DATABASE_HEADER, "Base de triches"),
    (keys::CHEATS_DATABASE_PATH, "Base"),
    (keys::CHEATS_DATABASE_USE, "Utiliser"),
    (keys::CHEATS_DATABASE_BROWSE, "Parcourir…"),
    (keys::CHEATS_DATABASE_LOOK_UP, "Chercher ce jeu"),
    (keys::CHEATS_NO_DATABASE, "Aucune base de triches : choisissez un dossier ou un zip de fiches, une par ROM et à son nom (.json de Mesen ou .cht de libretro)."),
    (keys::CHEATS_NO_SHEET, "La base n'a pas de fiche pour ce jeu."),
    (keys::CHEATS_SHEET, "Fiche : {0}"),
    (keys::CHEATS_MATCH_EXACT, "Faite pour cette ROM."),
    (keys::CHEATS_MATCH_FILE_NAME, "Trouvée par le nom de fichier de la ROM."),
    (keys::CHEATS_MATCH_SAME_REGION, "Faite pour une autre version du jeu : les codes peuvent ne pas marcher."),
    (keys::CHEATS_MATCH_OTHER_REGION, "Faite pour une autre région du jeu : les codes risquent fort de ne pas marcher."),
    (keys::CHEATS_ADD_ALL, "Tout ajouter (désactivé)"),
    (keys::CHEATS_SKIPPED, "{0} codes de la fiche ne se lisent pas."),
    (keys::CHEATS_DATABASE_ERROR, "La base de triches ne se lit pas : {0}"),
    (keys::CHEATS_NONE_NOTICE, "Aucune triche pour ce jeu"),
    (keys::CHEATS_SELECTED_NOTICE, "Triche {0}/{1} : {2} ({3})"),
    (keys::CHEATS_STATE_ON, "activée"),
    (keys::CHEATS_STATE_OFF, "désactivée"),
    (keys::CHEATS_KEYS_NOTE, "Touches RetroArch : T et Y choisissent une triche, U l'active ou la désactive."),
    (keys::CHEATS_ACHIEVEMENTS_NOTE, "Des triches sont actives avec RetroAchievements : les succès peuvent être refusés."),
    (keys::SEARCH_TITLE, "Recherche RAM"),
    (keys::SEARCH_SIZE, "Taille"),
    (keys::SEARCH_SIZE_BYTE, "1 octet"),
    (keys::SEARCH_SIZE_WORD, "2 octets"),
    (keys::SEARCH_SIZE_BCD, "BCD (2 chiffres par octet)"),
    (keys::SEARCH_SIGNED, "Signé"),
    (keys::SEARCH_START, "Nouvelle recherche"),
    (keys::SEARCH_COUNT, "{0} adresses"),
    (keys::SEARCH_NOT_STARTED, "Lancez une recherche, jouez, puis gardez les adresses dont la valeur a changé comme prévu : une vie perdue, une pièce ramassée."),
    (keys::SEARCH_TO_PREVIOUS, "la valeur précédente"),
    (keys::SEARCH_TO_DELTA, "la valeur précédente plus"),
    (keys::SEARCH_TO_VALUE, "la valeur"),
    (keys::SEARCH_FILTER, "Filtrer"),
    (keys::SEARCH_UNDO, "Annuler"),
    (keys::SEARCH_CHANGED, "Changé"),
    (keys::SEARCH_UNCHANGED, "Inchangé"),
    (keys::SEARCH_INCREASED, "Augmenté"),
    (keys::SEARCH_DECREASED, "Diminué"),
    (keys::SEARCH_ADDRESS, "Adresse"),
    (keys::SEARCH_PREVIOUS, "Avant"),
    (keys::SEARCH_CURRENT, "Maintenant"),
    (keys::SEARCH_SHOWN, "Les {0} premières sont listées."),
    (keys::SEARCH_VALUE, "Valeur"),
    (keys::SEARCH_ADD_CHEAT, "Ajouter comme triche"),
    (keys::SEARCH_ADDED, "Triche ajoutée : {0}"),
    (keys::SEARCH_BAD_VALUE, "Cette valeur ne tient pas dans cette taille."),
    (keys::SEARCH_NO_GAME, "Aucun jeu en cours."),
    (keys::HANDHELD_HOLD_FOR_SETTINGS, "A maintenu : réglages"),
    (keys::HANDHELD_HINT_HACKS, "WideScreen & Hacks"),
    (keys::SHORTCUT_HOME, "Origine"),
    (keys::SHORTCUT_KEYPAD_5, "5 du pavé"),
    (keys::HANDHELD_ARCHIVE_GAMES, "{0} jeux"),
    (keys::HANDHELD_FILTER_HINT, "Filtrer (Ctrl+F)"),
    (keys::SHADER_NONE, "aucun"),
    (keys::SHADER_CHOSEN, "Shader : {0}"),
    (keys::SCREENSHOT_SAVED, "Capture : {0}"),
    (keys::SCREENSHOT_FAILED, "La capture n'a pas pu être écrite"),
    (keys::SHORTCUT_KEYPAD_ADD, "+ du pavé"),
    (keys::SHORTCUT_KEYPAD_SUBTRACT, "- du pavé"),
    (keys::SHORTCUT_ESCAPE, "Échap"),
    (keys::STATE_SLOT_CHOSEN, "Slot {0}"),
    (keys::STATE_SLOT_CHOSEN_EMPTY, "Slot {0} (vide)"),
    (keys::STATE_SAVED_TO_SLOT, "Sauvegardé dans le slot {0}"),
    (keys::STATE_SLOT_LOADED, "Slot {0} chargé"),
    (keys::STATE_SLOT_EMPTY, "Slot {0} vide"),
    (keys::ESCAPE_AGAIN_TO_QUIT, "Appuie encore sur Échap pour quitter"),
    (keys::HANDHELD_MENU_SAVE_STATE, "Sauvegarder l'état"),
    (keys::HANDHELD_MENU_LOAD_STATE, "Charger l'état"),
    (keys::HANDHELD_MENU_GAME_SPEED, "Vitesse de jeu"),
    (keys::HANDHELD_MENU_QUIT_TO_LIST, "Retour à la liste des ROM"),
    (keys::HANDHELD_MENU_EXIT, "Quitter vers le lanceur"),
    (keys::HANDHELD_MENU_WEB_OPEN_ROM, "Web : ouvrir une ROM"),
    (keys::HANDHELD_MENU_WEB_IMPORT_PROFILE, "Web : importer un profil 3D"),
    (keys::HANDHELD_MENU_WEB_MANAGE_CACHE, "Web : gérer le stockage"),
    (keys::HANDHELD_OPTION_RENDER_SCALE, "Échelle de rendu"),
    (keys::HANDHELD_OPTION_FRAME_CAP, "Limite d'images"),
    (keys::HANDHELD_OPTION_CAMERA, "Vue de la caméra"),
    (keys::HANDHELD_OPTION_CONTROLS, "Commandes"),
    (keys::HANDHELD_OPTION_REGION, "Région"),
    (keys::HANDHELD_OPTION_OVERSCAN, "Overscan"),
    (keys::HANDHELD_OPTION_CATCH_UP, "Rattrapage"),
    (keys::HANDHELD_OPTION_PIPELINE, "Scripts en parallèle"),
    (keys::HANDHELD_OPTION_OPTIMIZATIONS, "Optimisations"),
    (keys::HANDHELD_REGION_AUTO, "Auto (ROM)"),
    (keys::HANDHELD_OPTION_ADVANCED, "Options avancées"),
    (keys::HANDHELD_OPTION_TOUCH_PAD, "Pad tactile"),
    (keys::HANDHELD_OPTION_TOUCH_SCREEN, "Écran tactile"),
    (keys::HANDHELD_TOUCH_PAD_ON, "Oui"),
    (keys::HANDHELD_TOUCH_PAD_OFF, "Non"),
    (keys::HANDHELD_CAMERA_RESET, "Vue par défaut"),
    (keys::HANDHELD_CAMERA_KEPT, "Caméra mémorisée"),
    (keys::HANDHELD_CAMERA_RESTORED, "Caméra restaurée"),
    (keys::HANDHELD_CONTROLS_PLAYER, "Manette du joueur 1"),
    // ----------------------------------- RetroAchievements (Rust-only)
    (keys::TAB_ACHIEVEMENTS, "RetroAchievements"),
    (keys::ACH_LIST, "Succès du jeu"),
    (keys::ACH_LIST_EMPTY, "Aucun succès : aucun jeu n'est lancé, ou RetroAchievements ne le connaît pas."),
    (keys::ACH_GRID_STUDYING, "RetroAchievements est interrogé sur ce jeu..."),
    (keys::ACH_GRID_SIGNED_OUT, "Connectez-vous à RetroAchievements (Options > RetroAchievements) pour voir les succès."),
    (keys::ACH_GRID_NONE, "Aucun succès : RetroAchievements ne connaît pas ce jeu, ou n'a pas pu être joint."),
    (keys::ACH_ACCOUNT, "Compte et options..."),
    (keys::ACH_SIGNED_OUT, "Non connecté."),
    (keys::ACH_SIGNING_IN, "Connexion..."),
    (keys::ACH_SIGNED_IN, "Connecté en tant que {0} ({1} points softcore)."),
    (keys::ACH_USERNAME, "Nom d'utilisateur"),
    (keys::ACH_PASSWORD, "Mot de passe"),
    (keys::ACH_SIGN_IN, "Se connecter"),
    (keys::ACH_SIGN_OUT, "Se déconnecter"),
    (keys::ACH_PASSWORD_HINT, "Le mot de passe est envoyé une fois à retroachievements.org et n'est jamais enregistré : un jeton garde la connexion."),
    (keys::ACH_SOFTCORE_HINT, "Mode softcore : sauvegardes d'état, retour en arrière et avance rapide restent permis ; les succès comptent en softcore."),
    (keys::ACH_TURN_ON_HINT, "Activez RetroAchievements pour vous connecter."),
    (keys::ACH_UNLOCKED, "Succès débloqué"),
    (keys::ACH_UNLOCKED_TEXT, "{0} ({1} pts)"),
    (keys::ACH_COMPLETED, "Jeu terminé à 100 % !"),
    (keys::ACH_COMPLETED_TEXT, "Tous les succès de {0} sont débloqués."),
    (keys::ACH_GAME_SUMMARY, "{0} succès débloqués sur {1} ({2}/{3} points, softcore)"),
    (keys::ACH_PERCENT, "{0} %"),
    (keys::ACH_GAME_NO_ACHIEVEMENTS, "Ce jeu n'a pas encore de succès."),
    (keys::ACH_GAME_UNKNOWN, "RetroAchievements ne connaît pas cette ROM."),
    (keys::ACH_GAME_FAILED, "RetroAchievements : les succès du jeu n'ont pas pu être chargés ({0})."),
    (keys::ACH_WELCOME, "RetroAchievements : connecté en tant que {0} ({1} points softcore)."),
    (keys::ACH_SIGN_IN_FAILED, "RetroAchievements : échec de la connexion ({0})."),
    (keys::ACH_SERVER_ERROR, "RetroAchievements : {0}"),
    (keys::ACH_DISCONNECTED, "RetroAchievements : serveur injoignable, les succès seront envoyés plus tard."),
    (keys::ACH_RECONNECTED, "RetroAchievements : les succès en attente sont envoyés."),
    (keys::ACH_CHALLENGE, "Défi"),
    (keys::ACH_POINTS, "{0} pts"),
    (keys::ACH_RARITY, "{0} % des joueurs"),
    (keys::ACH_BUCKET_LOCKED, "Verrouillés"),
    (keys::ACH_BUCKET_UNLOCKED, "Débloqués"),
    (keys::ACH_BUCKET_RECENT, "Débloqués pendant cette partie"),
    (keys::ACH_BUCKET_CHALLENGE, "Défis en cours"),
    (keys::ACH_BUCKET_ALMOST, "Presque débloqués"),
    (keys::ACH_BUCKET_UNOFFICIAL, "Non officiels"),
    (keys::ACH_BUCKET_UNSUPPORTED, "Non pris en charge"),
    (keys::ACH_BUCKET_UNSYNCED, "En attente du serveur"),
    (keys::ACH_KEY_TYPE, "Taper"),
    (keys::ACH_KEY_DELETE, "Effacer"),
    (keys::ACH_KEY_SHIFT, "Maj"),
    (keys::ACH_KEY_SPACE, "Espace"),
    (keys::ACH_KEY_DONE, "OK"),
    ("setting.achievements.enabled", "Activer RetroAchievements"),
    ("setting.achievements.encore", "Mode encore (les succès déjà obtenus se redéclenchent)"),
    ("setting.achievements.unofficial", "Tester les succès non officiels"),
    ("setting.achievements.sound", "Son de déblocage"),
    ("setting.achievements.challenges", "Afficher les défis en cours"),
    // --------------------------------------------------------- shaders window
    ("cmd.ui.show_shaders", "Shaders…"),
    ("cmd.ui.escape_quit", "Échap deux fois pour quitter"),
    ("shaders.title", "Shaders"),
    ("shaders.not_built", "Cette version de Zaxnes n'a pas les shaders slang."),
    ("shaders.pack_missing", "Cette version de Zaxnes a été construite sans son pack de shaders."),
    ("shaders.needs_wgpu", "Les shaders demandent le rendu Vulkan ou DirectX 12 (option API graphique)."),
    ("shaders.use_vulkan", "Passer en Vulkan (après redémarrage)"),
    ("shaders.restart", "Redémarrez Zaxnes pour utiliser le nouveau rendu."),
    ("shaders.slot.2d", "2D"),
    ("shaders.slot.2d_ws", "2D large"),
    ("shaders.slot.3d", "3D"),
    ("shaders.slot.3d_ws", "3D large"),
    ("shaders.slot.on_screen", "{0} (à l'écran)"),
    ("shaders.mode", "Shader"),
    ("shaders.mode.off", "Aucun"),
    ("shaders.mode.preset", "Preset"),
    ("shaders.mode.follow", "Comme {0}"),
    ("shaders.lines", "Lignes de l'image"),
    ("shaders.lines.window", "Celles de la fenêtre"),
    ("shaders.lines.console", "240 (les pixels de la console)"),
    ("shaders.lines.help", "Les lignes de l'image que lit le preset : 240 lui donnent les pixels de la console, pour lesquels les presets CRT et pixel-art sont écrits ; celles de la fenêtre gardent le détail de la scène 3D."),
    ("shaders.frame", "Cadrage"),
    ("shaders.frame.auto", "Auto"),
    ("shaders.frame.window", "Fenêtre entière"),
    ("shaders.frame.picture", "Image du jeu"),
    ("shaders.frame.help", "Ce que le preset lit de la scène 3D : toute la fenêtre, ou la scène cadrée sur l'image du jeu (le format de l'image NES, environ 4:3), pour qu'une TV contienne le jeu seul en grand. Auto cadre sur l'image pour les presets qui dessinent une TV ou un cadre."),
    ("shaders.follows", "Ce mode utilise le shader de l'onglet {0}."),
    ("shaders.none", "Pas de shader dans ce mode."),
    ("shaders.copy", "Copier depuis"),
    ("shaders.copy.none", "—"),
    ("shaders.copy.help", "Reprend le preset, ses paramètres et sa zone d'un autre onglet ; les lignes de l'image restent celles de cet onglet."),
    ("shaders.quick", "Recommandés :"),
    ("shaders.preset", "Preset"),
    ("shaders.preset.choose", "Choisir un preset"),
    ("shaders.menu.recommended", "Recommandés pour ce mode"),
    ("shaders.menu.all", "Tous les presets du pack"),
    ("shaders.menu.empty", "Le pack de shaders n'a pas été écrit dans le dossier utilisateur."),
    ("shaders.family.crt", "TV cathodique"),
    ("shaders.family.composite", "Signal composite (NTSC)"),
    ("shaders.family.bezel", "TV avec cadre (Mega Bezel)"),
    ("shaders.family.lcd", "Écran LCD"),
    ("shaders.family.smoothing", "Lissage / pixels nets"),
    ("shaders.family.combination", "Combinaisons"),
    ("shaders.prefilter", "Pré-filtre"),
    ("shaders.prefilter.help", "Agrandit ou lisse l'image avant le préréglage, comme « Append » de RetroArch : les passes des deux, l'une après l'autre."),
    ("shaders.prefilter.none", "Aucun"),
    ("shaders.prefilter.xbrz2", "xBRZ ×2"),
    ("shaders.prefilter.xbrz4", "xBRZ ×4"),
    ("shaders.prefilter.scalefx", "ScaleFX ×3"),
    ("shaders.prefilter.nnedi3_2", "nnedi3 ×2"),
    ("shaders.prefilter.nnedi3_4", "nnedi3 ×4"),
    ("shaders.backend", "Moteur graphique (au prochain démarrage)"),
    ("shaders.backend.gl", "OpenGL : pas de shaders"),
    ("shaders.backend.vulkan", "Vulkan : shaders"),
    ("shaders.backend.not_built", "OpenGL (cette version n'a pas Vulkan)"),
    ("shaders.cost.light", "léger"),
    ("shaders.cost.medium", "moyen"),
    ("shaders.cost.heavy", "lourd"),
    ("shaders.preset.guest", "CRT Guest Advanced"),
    ("shaders.preset.guest.help", "Le tube cathodique le plus complet : lignes, masque, halo, courbure."),
    ("shaders.preset.easymode", "CRT Easymode"),
    ("shaders.preset.easymode.help", "Un CRT simple et léger."),
    ("shaders.preset.zfast", "zfast CRT"),
    ("shaders.preset.zfast.help", "Le CRT le plus léger, pour les petites cartes."),
    ("shaders.preset.crt_pi", "CRT Pi"),
    ("shaders.preset.crt_pi.help", "Un CRT léger écrit pour le Raspberry Pi."),
    ("shaders.preset.nes_mini", "CRT NES Mini"),
    ("shaders.preset.nes_mini.help", "Les lignes de la NES Classic Mini."),
    ("shaders.preset.lottes", "CRT Lottes"),
    ("shaders.preset.lottes.help", "Le CRT de Timothy Lottes : lignes nettes et masque."),
    ("shaders.preset.hyllian", "CRT Hyllian"),
    ("shaders.preset.hyllian.help", "Un CRT net aux couleurs vives."),
    ("shaders.preset.geom", "CRT Geom"),
    ("shaders.preset.geom.help", "Un écran cathodique courbé."),
    ("shaders.preset.royale", "CRT Royale"),
    ("shaders.preset.royale.help", "Un masque de phosphore très fidèle, pour les grosses cartes."),
    ("shaders.preset.guest_ntsc", "CRT Guest NTSC"),
    ("shaders.preset.guest_ntsc.help", "Le tube Guest branché en composite, comme la NES l'était."),
    ("shaders.preset.ntsc_nes", "Composite NES"),
    ("shaders.preset.ntsc_nes.help", "Le bavement d'un signal composite de 256 pixels, avec lignes."),
    ("shaders.preset.blargg", "Blargg NTSC"),
    ("shaders.preset.blargg.help", "Le filtre NTSC de Blargg, celui de nombreux émulateurs NES."),
    ("shaders.preset.mega_bezel", "Mega Bezel"),
    ("shaders.preset.mega_bezel.help", "Une TV avec son cadre, son reflet et sa pièce, au format de l'image."),
    ("shaders.preset.mega_bezel_light", "Mega Bezel (léger)"),
    ("shaders.preset.mega_bezel_light.help", "La TV Mega Bezel sans le reflet, pour les cartes plus modestes."),
    ("shaders.preset.mega_bezel_reflect", "Mega Bezel (avancé)"),
    ("shaders.preset.mega_bezel_reflect.help", "La TV Mega Bezel avec tous les effets, pour les grosses cartes."),
    ("shaders.preset.mega_bezel_screen", "Écran Mega Bezel"),
    ("shaders.preset.mega_bezel_screen.help", "Le tube Mega Bezel seul sur toute l'image, quelle que soit sa largeur : pas de cadre 4:3 autour d'un jeu en grand écran."),
    ("shaders.preset.mega_bezel_screen_ntsc", "Écran Mega Bezel NTSC"),
    ("shaders.preset.mega_bezel_screen_ntsc.help", "Le tube Mega Bezel seul, branché en composite."),
    ("shaders.preset.mega_bezel_screen_light", "Écran Mega Bezel (léger)"),
    ("shaders.preset.mega_bezel_screen_light.help", "Le tube Mega Bezel léger seul sur toute l'image."),
    ("shaders.preset.lcd", "Grille LCD"),
    ("shaders.preset.lcd.help", "La grille de pixels d'un écran LCD de console portable."),
    ("shaders.preset.sharp", "Pixels nets"),
    ("shaders.preset.sharp.help", "Des pixels carrés sans scintillement à toute taille."),
    ("shaders.preset.xbrz", "xBRZ"),
    ("shaders.preset.xbrz.help", "Le pixel art lissé en courbes."),
    ("shaders.preset.scalefx", "ScaleFX"),
    ("shaders.preset.scalefx.help", "Le pixel art lissé, aux bords nets."),
    ("shaders.preset.scalefx_guest_hd", "ScaleFX + CRT Guest HD"),
    ("shaders.preset.scalefx_guest_hd.help", "Le pixel art lissé par ScaleFX, puis le tube Guest fait pour une image lissée."),
    ("shaders.preset.nnedi3_hyllian", "nnedi3 + CRT Hyllian"),
    ("shaders.preset.nnedi3_hyllian.help", "L'image agrandie quatre fois par nnedi3, puis le tube net Hyllian."),
    ("shaders.preset.nnedi3_easymode", "nnedi3 + CRT Easymode"),
    ("shaders.preset.nnedi3_easymode.help", "L'image agrandie quatre fois par nnedi3, puis le tube simple Easymode."),
    ("shaders.preset.nnedi3_mattias", "nnedi3 + CRT Mattias + lignes (TV 4K)"),
    ("shaders.preset.nnedi3_mattias.help", "L'image agrandie quatre fois par nnedi3, le tube doux Mattias et des lignes de balayage par-dessus : pour un écran 4K."),
    ("shaders.preset.fakelottes", "Fake Lottes"),
    ("shaders.preset.fakelottes.help", "Une version légère du CRT de Lottes."),
    ("shaders.preset.mattias", "CRT Mattias"),
    ("shaders.preset.mattias.help", "Un CRT doux avec une légère lueur."),
    ("shaders.preset.zfast_composite", "zfast CRT composite"),
    ("shaders.preset.zfast_composite.help", "Le CRT léger zfast avec le flou d'un signal composite."),
    ("shaders.preset.fakelottes_ntsc", "Fake Lottes NTSC"),
    ("shaders.preset.fakelottes_ntsc.help", "Le CRT léger de Lottes alimenté par un signal composite."),
    ("shaders.preset.royale_nes_rf", "CRT Royale NES (RF)"),
    ("shaders.preset.royale_nes_rf.help", "Le tube Royale alimenté par la sortie RF de la NES, masque à fentes : le plus fidèle, pour les cartes puissantes."),
    ("shaders.preset.mega_bezel_screen_smooth", "Mega Bezel écran (ScaleFX)"),
    ("shaders.preset.mega_bezel_screen_smooth.help", "Le tube Mega Bezel seul, le pixel art d'abord lissé par ScaleFX."),
    ("shaders.area", "Zone"),
    ("shaders.area.auto", "Auto"),
    ("shaders.area.picture", "Image"),
    ("shaders.area.window", "Fenêtre entière"),
    ("shaders.area.help", "Où dessine le preset : le rectangle de l'image, ou toute la fenêtre pour les presets qui dessinent une TV ou un cadre autour (Auto choisit la fenêtre pour les TV Mega Bezel, koko-aio et uborder)."),
    ("shaders.state.compiling", "Compilation…"),
    ("shaders.state.active", "Actif"),
    ("shaders.state.refused", "Ce preset n'a pas pu être utilisé :"),
    ("shaders.parameters", "Paramètres ({0} modifiés)"),
    ("shaders.parameters.reading", "Lecture des paramètres…"),
    ("shaders.parameters.none", "Ce preset n'a pas de paramètre."),
    ("shaders.parameters.default", "Valeur de départ :"),
    ("shaders.filter", "Filtrer"),
    ("shaders.reset_all", "Tout réinitialiser"),
    ("shaders.reset", "Réinit."),
];
