using Zax.Settings;

namespace Zax.App.Ui;

/// <summary>
/// The French table of AMD-035. The texts the interface already had are the base (AMD-035 §3);
/// the ones that had no French wording follow the same style.
/// </summary>
/// <remarks>
/// The three attribution lines of the about block are the sentence AMD-029 §3 gives word for
/// word, so the French table is the normative one there and the English table holds its
/// translation.
/// </remarks>
internal static partial class UiStrings
{
    private static readonly Dictionary<string, string> French = new(StringComparer.Ordinal)
    {
        // --------------------------------------------------------- menu bar
        [UiKeys.MenuFile] = "Fichier",
        [UiKeys.MenuEmulation] = "Émulation",
        [UiKeys.MenuVideo] = "Vidéo",
        [UiKeys.MenuAudio] = "Audio",
        [UiKeys.MenuTools] = "Outils",

        [UiKeys.MenuRecentRoms] = "ROMs récentes",
        [UiKeys.MenuLoadState] = "Charger un état",
        [UiKeys.MenuSaveState] = "Sauvegarder l'état",
        [UiKeys.MenuDeleteState] = "Supprimer un état",
        [UiKeys.MenuSpeed] = "Vitesse",
        [UiKeys.MenuRegion] = "Région",
        [UiKeys.MenuPorts] = "Ports",
        [UiKeys.MenuCameraMode] = "Mode caméra",
        [UiKeys.MenuVolume] = "Volume",
        [UiKeys.MenuNothingHere] = "(vide)",

        [UiKeys.MenuRegionAuto] = "Auto",
        [UiKeys.MenuPort] = "Port",
        [UiKeys.MenuPortController] = "Manette",
        [UiKeys.MenuPortZapper] = "Zapper",
        [UiKeys.MenuCameraOrbital] = "Orbitale (TPV)",
        [UiKeys.MenuCameraFirstPerson] = "Première personne (FPV)",

        [UiKeys.SlotEmpty] = "vide",
        [UiKeys.SlotQuick] = "État rapide",
        [UiKeys.SlotPrefix] = "Emplacement",

        // ----------------------------------------------------- window titles
        [UiKeys.WindowOptions] = "Options",
        [UiKeys.WindowKeyConfig] = "Configuration des touches",
        [UiKeys.WindowAbout] = "À propos",
        [UiKeys.WindowProfileChooser] = "Choisir le profil 3D",
        [UiKeys.WindowOpenRom] = "Ouvrir une ROM",
        [UiKeys.WindowImportProfile] = "Importer un profil 3D",
        [UiKeys.WindowImportCover] = "Importer une jaquette",
        [UiKeys.WindowImportThreeDSen] = "Importer depuis 3dSen",

        // ----------------------------------------------------------- buttons
        [UiKeys.ButtonClose] = "Fermer",
        [UiKeys.ButtonCancel] = "Annuler",
        [UiKeys.ButtonOpen] = "Ouvrir",
        [UiKeys.ButtonLaunch] = "Lancer",
        [UiKeys.ButtonParentFolder] = "Dossier parent",
        [UiKeys.ButtonResetDefaults] = "Valeurs par défaut",
        [UiKeys.ButtonResetBindings] = "Réinitialiser toutes les touches",
        [UiKeys.ButtonStopBinding] = "Arrêter la capture",
        [UiKeys.ButtonYes] = "Oui",
        [UiKeys.ButtonNo] = "Non",

        // AMD-056 REQ-UI-214 : les boutons des trois dialogues d'import. « OK » ne se
        // traduit pas ; « Replace » nomme l'action, donc « Remplacer » et non « Oui ».
        [UiKeys.ButtonOk] = "OK",
        [UiKeys.ButtonReplace] = "Remplacer",

        // ------------------------ notifications de SPEC-10 REQ-UI-082 à REQ-UI-084
        // La traduction des textes que la spec fixe mot pour mot (AMD-036 §1) : les
        // particularités de l'original appartiennent à l'entrée anglaise, pas à celle-ci.
        [UiKeys.NotifyRollBack] = "Retour arrière",
        [UiKeys.NotifyFastForward] = "Avance rapide",
        [UiKeys.NotifyQuickSave] = "Sauvegarde rapide",
        [UiKeys.NotifyQuickLoad] = "Chargement rapide",

        [UiKeys.NotifyCorruptedZip] = "Archive zip corrompue",
        [UiKeys.NotifyUncompressionFailed] = "La décompression a échoué",
        [UiKeys.NotifyInvalidRom] = "Fichier ROM invalide.",
        [UiKeys.NotifyGameMissing] = "Le profil du jeu n'existe plus.",
        [UiKeys.NotifyForeignCore] = "La sauvegarde d'état vient d'un autre cœur.",

        // AMD-051 §3 point 6 : la traduction de « Loading timed out. » (REQ-LOOP-073).
        [UiKeys.NotifyLoadingTimedOut] = "Délai de chargement dépassé.",

        // AMD-056 §2 a réécrit les deux entrées anglaises suivantes ; les traductions sont
        // refaites dessus. Celle de la version reste la même — « {0} » y recevait déjà le
        // numéro, assaini maintenant — et celle du format gagnait déjà l'espace après le
        // chemin que l'anglais vient de prendre, « obsolète » ayant toujours été le mot juste.
        [UiKeys.NotifyNotA3dnFile] = "Ce fichier n'est pas un fichier 3dn",
        [UiKeys.NotifyUnsupported3dnVersion] =
            "La version {0} du format 3dn n'est pas prise en charge",
        [UiKeys.NotifyObsoleteProfileFormat] =
            "{0} a un format de profil obsolète ou incorrect. "
            + "Ce profil n'est pas compatible avec la version actuelle de Zaxnes.",

        // ------------- notifications des fichiers persistants (REQ-PERS-022, REQ-PERS-024)
        [UiKeys.NotifySettingsUnreadable] =
            "Le fichier de réglages était illisible et a été réinitialisé.",
        [UiKeys.NotifySettingsNewerVersion] =
            "Les réglages ont été créés par une version plus récente.",
        [UiKeys.NotifyGameListUnreadable] =
            "La liste des jeux était illisible et a été réinitialisée.",
        [UiKeys.NotifyGameListNewerVersion] =
            "La liste des jeux a été créée par une version plus récente.",
        [UiKeys.NotifyBindingsUnreadable] =
            "Les affectations de touches étaient illisibles et ont été réinitialisées.",
        [UiKeys.NotifyBindingsNewerVersion] =
            "Les affectations de touches ont été créées par une version plus récente.",

        // Le texte d'AMD-030 §5 lui-même, dont l'entrée anglaise est la traduction.
        [UiKeys.NotifyThreeDSenSummary] =
            "3dSen : {0} profil(s) importé(s), {1} déjà présent(s), {2} en échec.",

        // ------------------- confirmations de REQ-PERS-042 et REQ-PERS-054
        [UiKeys.ConfirmDeleteGame] = "Voulez-vous supprimer le jeu et ses sauvegardes d'état ?",
        [UiKeys.ConfirmDeleteSlot] = "Voulez-vous supprimer l'emplacement {0} ?",

        // ------------------- les dialogues d'import d'AMD-056 REQ-UI-214. Rien pour un
        //                     import réussi : il n'affiche ni notification ni dialogue, et
        //                     l'entrée « Profil importé : … » de REQ-UI-212 a disparu.
        //                     Le corps du remplacement énumère au singulier ce que la ROM,
        //                     la jaquette et les sauvegardes d'état gardent.
        [UiKeys.DialogImportRefusedTitle] = "Impossible d'importer ce fichier",
        [UiKeys.DialogImportRefusedBody] = "{0} : {1}",
        [UiKeys.DialogImportBuiltInTitle] = "Déjà dans Zaxnes",
        [UiKeys.DialogImportBuiltInBody] =
            "{0} est un profil embarqué ; {1} n'a pas été importé.",
        [UiKeys.DialogImportReplaceTitle] = "Remplacer {0} ?",
        [UiKeys.DialogImportReplaceBody] =
            "Le profil sera remplacé par celui de {0}. "
            + "La ROM, la jaquette et les sauvegardes d'état seront conservées.",

        // ------------------------------------------------------------- hints
        [UiKeys.HintImportThreeDSen] =
            "Choisissez le dossier d'installation de 3dSen (celui qui contient 3dSen.exe), "
            + "ou directement le fichier .bundle des profils.",
        [UiKeys.HintResScaleDirect] = "(sans effet en rendu direct : résolution native)",
        [UiKeys.HintThicknessMsvoOnly] = "(sans effet avec l'occlusion Depth)",
        [UiKeys.HintPressAControl] = "Appuyez sur une touche ou un bouton…",
        [UiKeys.HintClickACell] = "Cliquez sur une case pour réassigner.",
        [UiKeys.HintAtNextStart] = "(au prochain démarrage)",
        [UiKeys.HintAtNextLoad] = "(au prochain chargement)",
        [UiKeys.HintProfileSetting] = "(réglage de profil)",
        [UiKeys.HintAdvanced] = "(avancé)",

        // ---------------------------------------------------- fields, columns
        [UiKeys.FieldSearch] = "Rechercher",

        // AMD-053 §4.
        [UiKeys.FieldFavorite] = "Favori",
        [UiKeys.FieldFavoritesOnly] = "Favoris seulement",

        [UiKeys.ColumnAction] = "Action",
        [UiKeys.ColumnKeyboard] = "Clavier",
        [UiKeys.ColumnGamepad] = "Manette",
        [UiKeys.ValueNoGamepad] = "(aucune manette)",

        // ---------------------------------------------------- welcome screen
        [UiKeys.WelcomeHint] = "Fichier ▸ Ouvrir une ROM",
        [UiKeys.WelcomeRecent] = "ROMs récentes",
        [UiKeys.WelcomeNoRecent] = "Aucune ROM n'a encore été ouverte.",

        // AMD-053 §4 : l'entrée de REQ-LOOP-076, en tête de l'énumération.
        [UiKeys.WelcomeResume] = "Reprendre",

        // -------------------------------------------------------- about block
        [UiKeys.AboutLicence] = "GPL v3",
        [UiKeys.AboutCreditInspired] = "Projet indépendant inspiré de 3dSen (Geod Studio) ;",
        [UiKeys.AboutCreditAdapted] = "contient du code adapté de 3DNes_Engine (Geod Studio, GPL v3).",
        [UiKeys.AboutCreditDisclaimer] = "Sans lien avec Geod Studio ni Nintendo.",
        [UiKeys.AboutVersionPrefix] = "Version",

        // -------------------------------------------------- options window tabs
        [UiKeys.TabVideo] = "Vidéo",
        [UiKeys.TabCamera] = "Caméra",
        [UiKeys.TabLight] = "Lumière et ombres",
        [UiKeys.TabAudio] = "Audio",
        [UiKeys.TabEmulation] = "Émulation",
        [UiKeys.TabInterface] = "Interface",

        // ------------------------------- groups inside a tab (AMD-043 §6)
        [UiKeys.GroupGeometry] = "Géométrie",
        [UiKeys.GroupStyle] = "Style",

        // ---------------------------------------- key configuration sections
        [UiKeys.KeysPlayerOne] = "Joueur 1 — boutons NES",
        [UiKeys.KeysPlayerTwo] = "Joueur 2 — boutons NES",
        [UiKeys.KeysCamera] = "Caméra",
        [UiKeys.KeysHotkeys] = "Raccourcis système",
        [UiKeys.KeysCommands] = "Commandes de menu",

        // ------------------------------------------------------ file browser
        [UiKeys.BrowserComputer] = "Poste de travail",
        [UiKeys.BrowserFilterNesRoms] = "ROMs NES (*.nes, *.zip)",
        [UiKeys.BrowserFilterNesFiles] = "Fichiers .nes",
        [UiKeys.BrowserFilterZip] = "Archives .zip",
        [UiKeys.BrowserFilterProfiles] = "Profils 3D (*.3dn, *.3dh)",
        [UiKeys.BrowserFilterBundle] = "Bundle de profils 3dSen (*.bundle)",
        [UiKeys.BrowserFilterPng] = "Images PNG (*.png)",
        [UiKeys.BrowserFilterAll] = "Tous les fichiers",

        // ---------------------------------------------------- profile chooser
        [UiKeys.ProfileTwoDimensional] = "2D Mode",
        [UiKeys.ProfileSourceTwoDimensional] = "sans conversion 3D",
        [UiKeys.ProfileSourceDefaultThreeD] = "géométrie 3D par défaut",
        [UiKeys.ProfileSourceBuiltIn] = "profil intégré",
        [UiKeys.ProfileSourceProgramFolder] = "profil du dossier du programme",
        [UiKeys.ProfileSourceUser] = "profil utilisateur",

        // ---------------------------------------------------------- shortcuts
        [UiKeys.ShortcutAltEnter] = "Alt+Entrée",
        [UiKeys.ShortcutShiftPrefix] = "Maj",
        [UiKeys.ShortcutQuickSave] = "raccourci QuickSave",
        [UiKeys.ShortcutQuickLoad] = "raccourci QuickLoad",
        [UiKeys.ShortcutFastForward] = "raccourci FastForward",
        [UiKeys.ShortcutRollback] = "raccourci RollBack",
        [UiKeys.ShortcutSaveSlots] = "Maj+F1…F10",
        [UiKeys.ShortcutLoadSlots] = "Ctrl+F1…F10",
        [UiKeys.ShortcutLongPressMenu] = "appui long sur Menu",

        // ------------------------------------------- commands of REQ-UI-205
        ["cmd.file.open_rom"] = "Ouvrir une ROM…",
        ["cmd.file.open_recent_rom"] = "ROMs récentes",
        ["cmd.file.choose_profile"] = "Choisir le profil 3D…",
        ["cmd.file.import_profile"] = "Importer un profil…",
        ["cmd.file.import_3dsen"] = "Importer depuis 3dSen…",
        ["cmd.file.launch_game"] = "Lancer le jeu",
        ["cmd.file.delete_game"] = "Supprimer le jeu…",
        ["cmd.file.create_game_art"] = "Créer la jaquette",
        ["cmd.file.import_cover"] = "Importer une jaquette…",

        // AMD-053 §3 point 17 : les quatre commandes ajoutées au modèle de REQ-UI-205.
        ["cmd.favorite.toggle"] = "Favori",
        ["cmd.library.filter_favorites"] = "Favoris seulement",
        ["cmd.library.search"] = "Rechercher",
        ["cmd.resume.last"] = "Reprendre",
        ["cmd.app.quit"] = "Quitter",
        ["cmd.state.save_slot"] = "Sauvegarder l'état",
        ["cmd.state.load_slot"] = "Charger un état",
        ["cmd.state.quick_save"] = "État rapide : sauvegarder",
        ["cmd.state.quick_load"] = "État rapide : charger",
        ["cmd.state.delete_slot"] = "Supprimer l'état",
        ["cmd.emulation.pause_resume"] = "Pause / Reprise",
        ["cmd.emulation.pause"] = "Pause",
        ["cmd.emulation.resume"] = "Reprise",
        // AMD-044 §3: the two resets now sit side by side, so the hard one is named as such.
        ["cmd.emulation.reset"] = "Reset complet",
        ["cmd.emulation.soft_reset"] = "Reset doux",
        ["cmd.emulation.stop"] = "Arrêter",
        ["cmd.emulation.fast_forward"] = "Avance rapide",
        ["cmd.emulation.rollback"] = "Rollback",
        ["cmd.emulation.set_speed"] = "Vitesse",
        ["cmd.emulation.speed_up"] = "Plus vite",
        ["cmd.emulation.speed_down"] = "Moins vite",
        ["cmd.emulation.set_region"] = "Région",
        ["cmd.emulation.auto_region"] = "Région : Auto",
        ["cmd.emulation.set_port_type"] = "Type de port",
        ["cmd.video.toggle_2d"] = "Mode 3D / Mode 2D",
        ["cmd.video.reset_view"] = "Réinitialiser la vue",
        ["cmd.video.set_camera_mode"] = "Mode caméra",
        ["cmd.video.toggle_local_camera"] = "Vue de dessus",
        ["cmd.video.set_skybox"] = "Skybox",
        ["cmd.video.toggle_shadows"] = "Ombres",
        ["cmd.video.toggle_fullscreen"] = "Plein écran",
        ["cmd.video.toggle_fps"] = "Afficher les FPS",
        ["cmd.audio.set_volume"] = "Volume",
        ["cmd.audio.volume_up"] = "Volume +",
        ["cmd.audio.volume_down"] = "Volume −",
        ["cmd.audio.toggle_mute"] = "Muet",
        ["cmd.audio.toggle_menu_music"] = "Musique de menu",
        ["cmd.tools.options"] = "Options…",
        ["cmd.tools.key_config"] = "Configuration des touches…",
        ["cmd.tools.data_folder"] = "Dossier des données",
        ["cmd.tools.about"] = "À propos",
        ["cmd.ui.cancel_modal"] = "Fermer la fenêtre modale",
        ["cmd.ui.toggle_debug_overlay"] = "Panneau de statistiques",
        ["cmd.ui.close_windows"] = "Fermer les fenêtres",

        // ------------------------------------------- labels of SPEC-09 settings
        ["setting." + SettingKeys.VideoFrameRate] = "Fréquence de rendu",

        // AMD-052 §3 point 10, mot pour mot.
        ["setting." + SettingKeys.VideoMaxQueuedFrames] = "Frames en file GPU",
        ["help." + SettingKeys.VideoMaxQueuedFrames] =
            "Limite le nombre de frames rendues que le GPU peut garder en attente d'affichage. "
            + "1 donne la latence la plus basse ; 2 est plus fluide.",
        ["setting." + SettingKeys.VideoSkipFrame] = "Rendu accéléré (saut d'images)",
        ["setting." + SettingKeys.VideoVSync] = "Synchronisation verticale",
        ["setting." + SettingKeys.VideoRenderMethod] = "Méthode de rendu",
        ["setting." + SettingKeys.VideoResScale] = "Échelle de résolution",
        ["setting." + SettingKeys.VideoAntiAlias] = "Anticrénelage",
        ["setting." + SettingKeys.RenderMeshAlgo] = "Algorithme de maillage",
        ["setting." + SettingKeys.RenderShadow] = "Ombres",
        ["setting." + SettingKeys.CameraFov] = "Champ de vision (degrés)",
        ["setting." + SettingKeys.LightIntensity] = "Intensité lumineuse",
        ["setting." + SettingKeys.SceneSkybox] = "Fond (skybox)",
        ["setting." + SettingKeys.SceneCustomBgColor] = "Couleur de fond",

        // AMD-043 §6, mot pour mot.
        ["setting." + SettingKeys.SceneBendLeftRight] = "Courbure gauche-droite",
        ["setting." + SettingKeys.SceneBendTopBottom] = "Courbure haut-bas",
        ["setting." + SettingKeys.SceneArcHorizontal] = "Arc horizontal",
        ["setting." + SettingKeys.SceneArcCenter] = "Centre de l'arc",
        ["help." + SettingKeys.SceneBendLeftRight] =
            "Courbe la scène horizontalement. Une valeur positive la bombe comme un écran "
            + "cathodique ; une valeur négative l'enroule autour du joueur.",
        ["help." + SettingKeys.SceneBendTopBottom] =
            "Courbe la scène verticalement. Une valeur positive la bombe ; une valeur négative "
            + "l'enroule autour du joueur.",
        ["help." + SettingKeys.SceneArcHorizontal] =
            "Pose le niveau sur un arc, comme sur une toute petite planète. Idéal pour les jeux "
            + "à défilement horizontal.",

        // AMD-046 §3, mot pour mot.
        ["setting." + SettingKeys.SceneBend2dMode] = "Courber le mode 2D",
        ["help." + SettingKeys.SceneBend2dMode] =
            "Applique aussi la courbure à l'image plate du mode 2D. Désactivez-le pour garder "
            + "l'image plate.",

        // AMD-055 §4, mot pour mot : le style visuel et ses douze paramètres.
        ["setting." + SettingKeys.RenderVisualStyle] = "Style",
        ["help." + SettingKeys.RenderVisualStyle] =
            "Classic garde l'aspect d'origine. Enhanced ajoute des effets optionnels.",
        ["setting." + SettingKeys.RenderStyleAo] = "Occlusion ambiante",
        ["help." + SettingKeys.RenderStyleAo] = "Ombre douce dans les coins et les creux.",
        ["setting." + SettingKeys.RenderStyleAoStrength] = "Force",
        ["help." + SettingKeys.RenderStyleAoStrength] = "Intensité de l'assombrissement.",
        ["setting." + SettingKeys.RenderStyleAoRadius] = "Rayon",
        ["help." + SettingKeys.RenderStyleAoRadius] = "Portée de l'assombrissement.",
        ["setting." + SettingKeys.RenderStyleAoThickness] = "Épaisseur",
        ["help." + SettingKeys.RenderStyleAoThickness] =
            "Sans effet avec l'occlusion Depth.",
        ["setting." + SettingKeys.RenderStyleAoQuality] = "Qualité de l'occlusion",
        ["help." + SettingKeys.RenderStyleAoQuality] =
            "Plus d'échantillons, ombre plus lisse.",
        ["setting." + SettingKeys.RenderStylePixelShadow] = "Ombres pixel",
        ["help." + SettingKeys.RenderStylePixelShadow] =
            "Des ombres qui suivent la grille de pixels du jeu.",
        ["setting." + SettingKeys.RenderStyleVoxelGrid] = "Grille de voxels",
        ["help." + SettingKeys.RenderStyleVoxelGrid] =
            "Des rainures tous les 2, 4 ou 8 voxels, comme des briques de jouet.",
        ["setting." + SettingKeys.RenderStyleReflections] = "Reflets",
        ["help." + SettingKeys.RenderStyleReflections] =
            "Le sol reflète la scène comme une surface polie.",
        ["setting." + SettingKeys.RenderStyleSsrShine] = "Brillance du sol",
        ["help." + SettingKeys.RenderStyleSsrShine] = "Intensité du reflet.",
        ["setting." + SettingKeys.RenderStyleSsrClarity] = "Netteté",
        ["help." + SettingKeys.RenderStyleSsrClarity] =
            "Une valeur élevée garde le reflet net.",
        ["setting." + SettingKeys.RenderStyleSsrLength] = "Longueur",
        ["help." + SettingKeys.RenderStyleSsrLength] = "Portée du reflet.",
        ["setting." + SettingKeys.RenderStyleSsrQuality] = "Qualité des reflets",
        ["help." + SettingKeys.RenderStyleSsrQuality] =
            "Plus de pas, moins de trous dans le reflet.",

        ["setting." + SettingKeys.EmuGameSpeed] = "Vitesse de jeu",
        ["setting." + SettingKeys.EmuAutoTvMode] = "Région détectée automatiquement",
        ["setting." + SettingKeys.EmuTvMode] = "Région vidéo",
        ["setting." + SettingKeys.EmuCore] = "Cœur d'émulation",
        ["setting." + SettingKeys.EmuRunAhead] = "Run-ahead",
        ["setting." + SettingKeys.AudioVolume] = "Volume général",
        ["setting." + SettingKeys.AudioBgMusic] = "Musique de menu",
        ["setting." + SettingKeys.AudioMute] = "Muet",
        ["setting." + SettingKeys.InputEmulatedMenuButton] = "Menu = Select + Start",
        ["setting." + SettingKeys.UiFullscreen] = "Plein écran",
        ["setting." + SettingKeys.UiShowFps] = "Afficher les FPS",
        ["setting." + SettingKeys.UiSortOption] = "Tri de la bibliothèque",
        ["setting." + SettingKeys.UiLanguage] = "Langue",

        // ----------------------------------- labels of the enumerated values
        ["option.Off"] = "Désactivée",
        ["option.Every refresh"] = "À chaque rafraîchissement",
        ["option.Every other refresh"] = "Un rafraîchissement sur deux",
        ["option.Default"] = "Par défaut",
        ["option.Color"] = "Couleur",
        ["option.Gradient"] = "Dégradé",
        ["option.Fantasy"] = "Fantaisie",
        ["option.None"] = "Aucun",
        ["option.Native"] = "Native",
        ["option.enabled"] = "activées",
        ["option.disabled"] = "désactivées",
        ["option.A-Z"] = "A-Z",
        ["option.Recent"] = "Récent",
        ["option.Favorites first"] = "Favoris d'abord",
        ["option.en"] = "English",
        ["option.fr"] = "Français",
        ["option.Screen Center"] = "Centre de l'écran",
        ["option.Screen Edge"] = "Bord de l'écran",

        // AMD-055 §4 : les valeurs des listes de style, mot pour mot. « Désactivée » est
        // l'entrée partagée « option.Off » ci-dessus, qui convient à l'occlusion comme à la
        // grille. Les deux listes de qualité portent des libellés distincts, parce qu'elles
        // peuvent être visibles en même temps dans le même panneau.
        ["option.Classic"] = "Classic",
        ["option.Enhanced"] = "Enhanced",
        ["option.MSVO"] = "MSVO",
        ["option.Depth"] = "Depth",
        ["option.On"] = "Activée",
        ["option.Low"] = "Faible",
        ["option.Medium"] = "Moyenne",
        ["option.High"] = "Élevée",
        ["option.Ultra"] = "Ultra",
        ["option.Every 2 voxels"] = "Tous les 2 voxels",
        ["option.Every 4 voxels"] = "Tous les 4 voxels",
        ["option.Every 8 voxels"] = "Tous les 8 voxels",

        // Les quatre valeurs de Run-ahead d'AMD-041 §3.
        ["option." + SettingKeys.EmuRunAhead + ".Off"] = "Désactivé",
        ["option." + SettingKeys.EmuRunAhead + ".1 frame (Recommended)"] = "1 frame (recommandé)",
        ["option." + SettingKeys.EmuRunAhead + ".2 frames"] = "2 frames",
        ["option." + SettingKeys.EmuRunAhead + ".3 frames"] = "3 frames",
    };
}
