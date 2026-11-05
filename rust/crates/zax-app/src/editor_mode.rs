//! The editor mode (owner request): writing a Lua profile with an AI agent.
//!
//! Turned on (`ui.editor_mode`, Tools ▸ Editor mode, `--editor`), it starts the MCP server and
//! the pens (EDITOR-1) and lays out the `editor` folder of the user data, so that an agent that
//! knows nothing of Zaxnes finds there all it needs:
//!
//! ```text
//! editor/
//!   AGENT.md              start here
//!   docs/                 the editor, the method, profile.json, the Lua API, the specifications
//!   templates/            the script and the notes of a new project
//!   examples/             a complete project (FlappyZaxNes) and its classification script
//!   projects/<game>/      one project per game: profile.json, main.lua, NOTES.md, PROMPT.txt
//! ```
//!
//! The documents are carried by the executable and written when the folder lacks them or when
//! they changed with a new version; the projects are the user's and never written again. A
//! project's `.3dn` is built by Zaxnes from its `profile.json` ([`build_project`]) when it is
//! made and at each script reload, so the agent needs neither the harness nor any script of its
//! own to see its work.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use zax_profiles::description::{self, BuiltProfile, RgbaPicture};

/// The source of Zaxnes, for what the documents do not say.
pub const SOURCE_URL: &str = "https://github.com/nixxou/zaxnes";

/// The name of the folder in the user data.
pub const FOLDER: &str = "editor";

/// The documents the executable carries: their path in the folder and their text.
const DOCUMENTS: &[(&str, &str)] = &[
    ("AGENT.md", include_str!("../editor/AGENT.md")),
    (
        "docs/PROFILE-JSON.md",
        include_str!("../editor/PROFILE-JSON.md"),
    ),
    ("docs/EDITOR.md", include_str!("../../../docs/EDITOR.md")),
    (
        "docs/EDITOR-PROFILE-WORKFLOW.md",
        include_str!("../../../docs/EDITOR-PROFILE-WORKFLOW.md"),
    ),
    (
        "docs/LUA-API.txt",
        include_str!("../../../goldens/lua-api.txt"),
    ),
    (
        "docs/spec/SPEC-00-glossaire.md",
        include_str!("../../../../docs/spec/SPEC-00-glossaire.md"),
    ),
    (
        "docs/spec/SPEC-03-format-profil.md",
        include_str!("../../../../docs/spec/SPEC-03-format-profil.md"),
    ),
    (
        "docs/spec/SPEC-04-conversion.md",
        include_str!("../../../../docs/spec/SPEC-04-conversion.md"),
    ),
    (
        "docs/spec/SPEC-04B-geometrie.md",
        include_str!("../../../../docs/spec/SPEC-04B-geometrie.md"),
    ),
    (
        "docs/spec/SPEC-05-lua.md",
        include_str!("../../../../docs/spec/SPEC-05-lua.md"),
    ),
    (
        "docs/DUMP-FORMAT.md",
        include_str!("../editor/DUMP-FORMAT.md"),
    ),
    ("docs/CLASSIFY.md", include_str!("../editor/CLASSIFY.md")),
    ("docs/RECIPES.md", include_str!("../editor/RECIPES.md")),
    ("docs/CHECKLIST.md", include_str!("../editor/CHECKLIST.md")),
    (
        "examples/flappyzaxnes/README.md",
        include_str!("../editor/examples/flappyzaxnes/README.md"),
    ),
    (
        "examples/flappyzaxnes/profile.json",
        include_str!("../demo/source/profile/flappyzaxnes.json"),
    ),
    (
        "examples/flappyzaxnes/profile3d.py",
        include_str!("../demo/source/tools/profile3d.py"),
    ),
    (
        "examples/classify-rules.json",
        include_str!("../editor/examples/classify-rules.json"),
    ),
    ("templates/main.lua", TEMPLATE_SCRIPT),
    ("templates/NOTES.md", TEMPLATE_NOTES),
];

/// The script of a new project.
const TEMPLATE_SCRIPT: &str = include_str!("../editor/main.lua");
/// The notes of a new project, which the agent keeps up to date.
const TEMPLATE_NOTES: &str = include_str!("../editor/NOTES.md");

/// The file that records which documents were written, to write them again when they change.
const STAMP: &str = ".documents";

/// The editor folder of the user data `root`.
#[must_use]
pub fn folder(root: &Path) -> PathBuf {
    root.join(FOLDER)
}

/// A digest of the documents (FNV-1a), which changes with any of them.
fn documents_stamp() -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (path, text) in DOCUMENTS {
        for byte in path.bytes().chain([0]).chain(text.bytes()).chain([0]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{}-{hash:016x}", env!("CARGO_PKG_VERSION"))
}

/// Lays out the folder: the documents when one is missing or they changed since they were
/// written, and the `projects` folder. Returns whether anything was written.
///
/// # Errors
/// A file could not be written.
pub fn deploy(folder: &Path) -> io::Result<bool> {
    let stamp = documents_stamp();
    let stamp_file = folder.join(STAMP);
    let current = std::fs::read_to_string(&stamp_file).is_ok_and(|text| text.trim() == stamp);
    let mut written = false;
    for (path, text) in DOCUMENTS {
        let file = folder.join(path);
        if current && file.is_file() {
            continue;
        }
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, text)?;
        written = true;
    }
    std::fs::create_dir_all(folder.join("projects"))?;
    if !current {
        std::fs::write(&stamp_file, &stamp)?;
    }
    Ok(written)
}

/// A PNG texture of a description, read with the renderer's decoder.
fn read_png(path: &Path) -> Result<RgbaPicture, String> {
    let image = zax_render::assets::png_image::load(path).map_err(|e| e.to_string())?;
    let pixels = (0..image.height)
        .flat_map(|y| (0..image.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (r, g, b, a) = image.pixel(x, y);
            [r, g, b, a]
        })
        .collect();
    Ok(RgbaPicture {
        width: image.width,
        height: image.height,
        pixels,
    })
}

/// The description of a project.
fn description_file(project: &Path) -> PathBuf {
    project.join("profile.json")
}

/// Builds the profile of the project folder `project` from its `profile.json`.
///
/// # Errors
/// The text to show: the description, a script or a texture cannot be read, or is malformed.
pub fn build_project(project: &Path) -> Result<BuiltProfile, String> {
    description::build_profile(&description_file(project), &read_png)
}

/// The identifier a project's description gives its profile.
fn project_id(project: &Path) -> Option<String> {
    let text = std::fs::read_to_string(description_file(project)).ok()?;
    let spec: Value = serde_json::from_str(&text).ok()?;
    spec.get("id").and_then(Value::as_str).map(str::to_owned)
}

/// The project of the editor folder `folder` whose profile is `profile_id`, if any.
#[must_use]
pub fn project_of(folder: &Path, profile_id: &str) -> Option<PathBuf> {
    std::fs::read_dir(folder.join("projects"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .find(|path| project_id(path).as_deref() == Some(profile_id))
}

/// A project just made.
#[derive(Debug, Clone)]
pub struct NewProject {
    /// Its folder.
    pub folder: PathBuf,
    /// Its profile, built: the file to import into the library.
    pub profile_file: PathBuf,
    /// The title of its profile.
    pub title: String,
}

/// The name of a game made fit for a folder: letters, digits, `-` and `_`, the rest spaces.
fn folder_name(game: &str) -> String {
    let name: String = game
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
                c
            } else {
                ' '
            }
        })
        .collect();
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let name = name.trim_matches('.').to_owned();
    if name.is_empty() {
        "game".to_owned()
    } else {
        name
    }
}

/// An identifier of its own for a new profile: `lua-<name>-<time>`.
fn new_id(name: &str) -> String {
    let slug: String = name
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '-' || c == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    format!("lua-{slug}-{:08x}", stamp & 0xffff_ffff)
}

/// The project folder for the game `name`: `projects/<name>`, or `<name> 2`, `<name> 3`... when
/// it is taken.
fn free_project_folder(folder: &Path, name: &str) -> PathBuf {
    let projects = folder.join("projects");
    (1..=999)
        .map(|n| {
            if n == 1 {
                projects.join(name)
            } else {
                projects.join(format!("{name} {n}"))
            }
        })
        .find(|path| !path.exists())
        .unwrap_or_else(|| projects.join(name))
}

/// A script's name made fit for a file name.
fn script_file_name(name: &str, taken: &mut Vec<String>) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = if stem.is_empty() {
        "script".to_owned()
    } else {
        stem
    };
    let mut file = format!("{stem}.lua");
    let mut n = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&file)) {
        file = format!("{stem}_{n}.lua");
        n += 1;
    }
    taken.push(file.clone());
    file
}

/// Makes a project from the existing profile file `source` of the game `game`: the profile kept
/// whole as `base.3dn` (its patterns and settings, which `profile.json` adds to), its scripts
/// as Lua files, and a profile of its own (a new identifier, " (Lua)" after the title), so that
/// the original stays as it is.
///
/// # Errors
/// The text to show: the profile cannot be read, a file could not be written, or the project
/// does not build.
pub fn export_project(folder: &Path, game: &str, source: &Path) -> Result<NewProject, String> {
    let bytes = std::fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?;
    let profile = zax_profiles::format::load(&bytes, None, None)
        .map_err(|e| format!("{}: {}", source.display(), e.message))?
        .profile;
    let name = folder_name(game);
    let project = free_project_folder(folder, &name);
    let fail = |e: io::Error| format!("{}: {e}", project.display());
    std::fs::create_dir_all(&project).map_err(fail)?;
    std::fs::write(project.join("base.3dn"), &bytes).map_err(fail)?;
    let mut taken = Vec::new();
    let mut scripts = Vec::new();
    for script in &profile.scripts {
        let script_name = script.name.clone().unwrap_or_else(|| "script".to_owned());
        let file = script_file_name(&script_name, &mut taken);
        std::fs::write(project.join(&file), script.source.as_deref().unwrap_or(""))
            .map_err(fail)?;
        scripts.push(json!({
            "name": script_name,
            "file": file,
            "enabled": script.enabled,
            "tags": script.tag_string.as_deref().unwrap_or("*"),
        }));
    }
    let original = profile.title.clone().unwrap_or_else(|| name.clone());
    let title = format!("{original} (Lua)");
    let spec = json!({
        "id": new_id(&name),
        "title": title,
        "base": "base.3dn",
        "tiles": {},
        "patterns": [],
        "scripts": scripts,
    });
    let text = serde_json::to_string_pretty(&spec).map_err(|e| e.to_string())?;
    std::fs::write(description_file(&project), text + "\n").map_err(fail)?;
    let notes = TEMPLATE_NOTES.replace("<game>", &name)
        + &format!(
            "\n## Origin\n\nMade from the profile \"{original}\" ({} permanent and {} temporary \
             patterns, {} scripts), kept whole as `base.3dn`: its patterns and settings stay, the \
             patterns of `profile.json` are added to them, its scripts are the `.lua` files here. \
             A pattern of the base is changed from Lua (its tags), or by a pattern of \
             `profile.json` of the same tiles.\n",
            profile.permanent_patterns.len(),
            profile.temporary_patterns.len(),
            profile.scripts.len()
        );
    std::fs::write(project.join("NOTES.md"), notes).map_err(fail)?;
    let built = build_project(&project)?;
    let build_folder = project.join("build");
    std::fs::create_dir_all(&build_folder).map_err(fail)?;
    let profile_file = build_folder.join(format!("{name}.3dn"));
    std::fs::write(&profile_file, &built.bytes).map_err(fail)?;
    Ok(NewProject {
        folder: project,
        profile_file,
        title,
    })
}

/// Makes a blank project for the game `game` (the ROM's name) in the editor folder `folder`:
/// `profile.json` naming `main.lua`, the template script, and the profile built from them.
///
/// # Errors
/// The text to show: a file could not be written, or the profile not built.
pub fn create_project(folder: &Path, game: &str) -> Result<NewProject, String> {
    let name = folder_name(game);
    let project = free_project_folder(folder, &name);
    let fail = |e: io::Error| format!("{}: {e}", project.display());
    std::fs::create_dir_all(&project).map_err(fail)?;
    let title = format!("{name} (Lua)");
    let spec = json!({
        "id": new_id(&name),
        "title": title,
        "author": "",
        "publisher": "",
        "description": format!("A Lua profile of {name}, made with the editor mode of Zaxnes."),
        // A profile without patterns is played in 2D: this one keeps the new profile in 3D (the
        // default extrusion of every tile) until the agent writes patterns of its own.
        "tiles": { "placeholder": [PLACEHOLDER_PLANES[0], PLACEHOLDER_PLANES[1]] },
        "patterns": [{ "grid": [["placeholder"]], "depth": 8, "tags": "placeholder" }],
        "scripts": [{ "name": "main", "file": "main.lua" }],
    });
    let text = serde_json::to_string_pretty(&spec).map_err(|e| e.to_string())?;
    std::fs::write(description_file(&project), text + "\n").map_err(fail)?;
    std::fs::write(project.join("main.lua"), TEMPLATE_SCRIPT).map_err(fail)?;
    std::fs::write(
        project.join("NOTES.md"),
        TEMPLATE_NOTES.replace("<game>", &name),
    )
    .map_err(fail)?;
    let built = build_project(&project)?;
    let build_folder = project.join("build");
    std::fs::create_dir_all(&build_folder).map_err(fail)?;
    let profile_file = build_folder.join(format!("{name}.3dn"));
    std::fs::write(&profile_file, &built.bytes).map_err(fail)?;
    Ok(NewProject {
        folder: project,
        profile_file,
        title,
    })
}

/// The picture of the placeholder pattern of a new profile: a tile no game is likely to draw.
const PLACEHOLDER_PLANES: [&str; 2] = ["8142241818244281", "0f0f0f0ff0f0f0f0"];

/// What the prompt for the agent says of this session.
#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// The editor folder.
    pub folder: PathBuf,
    /// The game on screen: its title and its ROM.
    pub game: Option<(String, String)>,
    /// What identifies the ROM: data CRC, mapper, region.
    pub rom_identity: Option<String>,
    /// The movie being played and its length in frames.
    pub movie: Option<(PathBuf, u32)>,
    /// The project of the game's profile, when it has one.
    pub project: Option<PathBuf>,
    /// The URL of the MCP server, when it runs.
    pub mcp_url: Option<String>,
    /// `zax-harness`, when it is next to the executable.
    pub harness: Option<PathBuf>,
    /// Whether the user reads French.
    pub french: bool,
}

/// The prompt to give an AI agent, in the user's language.
#[must_use]
#[allow(clippy::too_many_lines)] // the prompt's lines, in their order and both languages
pub fn agent_prompt(context: &PromptContext) -> String {
    let french = context.french;
    let t = |english: &'static str, francais: &'static str| if french { francais } else { english };
    let mut out = String::new();
    out.push_str(t(
        "You are going to write a 3D Lua profile for Zaxnes, a NES emulator that draws games in 3D (profiles compatible with 3dSen).\n",
        "Tu vas écrire un profil 3D en Lua pour Zaxnes, un émulateur NES qui dessine les jeux en 3D (profils compatibles 3dSen).\n",
    ));
    if let Some((title, rom)) = &context.game {
        let _ = writeln!(out, "{} {title} (ROM : {rom})", t("Game:", "Jeu :"));
    }
    if let Some(identity) = &context.rom_identity {
        let _ = writeln!(
            out,
            "{} {identity}",
            t("ROM identity:", "Identité de la ROM :")
        );
    }
    match &context.movie {
        Some((movie, length)) => {
            let _ = writeln!(
                out,
                "{} {} ({length} frames)",
                t(
                    "The movie being played (seek reaches any of its frames):",
                    "Le film en lecture (seek va à n'importe laquelle de ses frames) :",
                ),
                movie.display()
            );
        }
        None => out.push_str(t(
            "No movie is playing: ask the user for one that goes through the whole game (an FCEUX .fm2, from TASVideos for instance), then open it with open_movie.\n",
            "Aucun film n'est en lecture : demande à l'utilisateur un film qui traverse tout le jeu (un .fm2 FCEUX, de TASVideos par exemple), puis ouvre-le avec open_movie.\n",
        )),
    }
    let agent = context.folder.join("AGENT.md");
    let _ = writeln!(
        out,
        "{} {}",
        t("First read", "Lis d'abord"),
        agent.display()
    );
    let _ = writeln!(
        out,
        "{} {}",
        t(
            "It points to the documents of the folder (the editor, the method, the Lua API, the profile format):",
            "Il mène aux documents du dossier (l'éditeur, la méthode, l'API Lua, le format des profils) :",
        ),
        context.folder.join("docs").display()
    );
    match &context.project {
        Some(project) => {
            let _ = writeln!(
                out,
                "{} {}",
                t(
                    "Your project (profile.json and main.lua; work only there):",
                    "Ton projet (profile.json et main.lua ; ne travaille que là) :",
                ),
                project.display()
            );
            out.push_str(t(
                "After each change, call reload_script (or the user presses F5): Zaxnes rebuilds the .3dn from the project and restarts the scripts. Then step one frame and read get_log.\n",
                "Après chaque modification, appelle reload_script (ou l'utilisateur appuie sur F5) : Zaxnes reconstruit le .3dn depuis le projet et relance les scripts. Avance ensuite d'une frame (step) et lis get_log.\n",
            ));
            out.push_str(t(
                "Keep NOTES.md of the project up to date (state, depth plan, my choices, what is left): it is what lets the work resume after a crash or with another agent. Read it first if it is not empty.\n",
                "Tiens à jour NOTES.md du projet (état, plan des profondeurs, mes choix, ce qui reste) : c'est ce qui permet de reprendre après un plantage ou avec un autre agent. Lis-le d'abord s'il n'est pas vide.\n",
            ));
        }
        None => out.push_str(t(
            "The game has no project yet: ask the user to choose Tools > New Lua profile for this game.\n",
            "Le jeu n'a pas encore de projet : demande à l'utilisateur de choisir Outils > Nouveau profil Lua pour ce jeu.\n",
        )),
    }
    match &context.mcp_url {
        Some(url) => {
            let _ = writeln!(
                out,
                "{} {url}",
                t(
                    "Zaxnes runs in editor mode; its MCP server (tools status, seek, step, screenshot, dump_frame, annotations, reload_script, get_log...):",
                    "Zaxnes tourne en mode éditeur ; son serveur MCP (outils status, seek, step, screenshot, dump_frame, annotations, reload_script, get_log...) :",
                ),
            );
            let _ = writeln!(
                out,
                "{} claude mcp add --transport http zaxnes {url}",
                t(
                    "If you do not have its tools, it is added once with:",
                    "Si tu n'as pas ses outils, on l'ajoute une fois avec :",
                ),
            );
        }
        None => out.push_str(t(
            "Its MCP server is not running: ask the user to turn the editor mode on (Tools > Editor mode).\n",
            "Son serveur MCP ne tourne pas : demande à l'utilisateur d'activer le mode éditeur (Outils > Mode éditeur).\n",
        )),
    }
    if let Some(harness) = &context.harness {
        let _ = writeln!(
            out,
            "{} {}",
            t(
                "The harness (classify the tiles of dumps, render from any angle, profile-dump):",
                "Le harnais (classer les tuiles des dumps, rendu sous n'importe quel angle, profile-dump) :",
            ),
            harness.display()
        );
    }
    out.push_str(t(
        "I will show you what to do on screen with the editor's pens (read them with annotations) and tell you what to change. Keep the profile readable by 3dSen: no engine change for one game.\n",
        "Je te montrerai quoi faire à l'écran avec les stylos de l'éditeur (lis-les avec annotations) et te dirai quoi changer. Garde le profil lisible par 3dSen : aucun changement du moteur pour un jeu.\n",
    ));
    out.push_str(t(
        "Before saying a part is done, go through docs/CHECKLIST.md.\n",
        "Avant de dire qu'une partie est finie, passe docs/CHECKLIST.md.\n",
    ));
    let _ = writeln!(
        out,
        "{} {SOURCE_URL}",
        t(
            "The source of Zaxnes, if the documents are not enough:",
            "Le source de Zaxnes, si les documents ne suffisent pas :",
        ),
    );
    out
}

/// Writes the prompt into the project's `PROMPT.txt`.
///
/// # Errors
/// The file could not be written.
pub fn write_prompt(project: &Path, prompt: &str) -> io::Result<()> {
    std::fs::write(project.join("PROMPT.txt"), prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zaxnes-editor-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn deploy_writes_the_documents_once_and_keeps_the_user_s_edits() {
        let dir = scratch("deploy");
        assert!(deploy(&dir).unwrap_or(false));
        assert!(dir.join("AGENT.md").is_file());
        assert!(dir.join("docs/spec/SPEC-05-lua.md").is_file());
        assert!(dir.join("projects").is_dir());
        // up to date: nothing is written again, an edited document stays
        let _ = std::fs::write(dir.join("docs/EDITOR.md"), "mine");
        assert!(!deploy(&dir).unwrap_or(true));
        assert_eq!(
            std::fs::read_to_string(dir.join("docs/EDITOR.md"))
                .ok()
                .as_deref(),
            Some("mine")
        );
        // a missing one comes back
        let _ = std::fs::remove_file(dir.join("AGENT.md"));
        assert!(deploy(&dir).unwrap_or(false));
        assert!(dir.join("AGENT.md").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_project_builds_and_is_found_by_its_id() {
        let dir = scratch("project");
        let _ = deploy(&dir);
        let project = create_project(&dir, "Metal Force [Korea][Unl][!]").ok();
        let Some(project) = project else {
            panic!("the project was not made");
        };
        assert!(project.folder.ends_with("projects/Metal Force Korea Unl"));
        assert!(project.profile_file.is_file());
        let id = project_id(&project.folder).unwrap_or_default();
        assert!(id.starts_with("lua-metal-force-korea-unl-"), "{id}");
        assert_eq!(project_of(&dir, &id), Some(project.folder.clone()));
        // a second one of the same game gets a folder of its own
        let second = create_project(&dir, "Metal Force [Korea][Unl][!]").ok();
        assert!(second.is_some_and(|second| second.folder.ends_with("Metal Force Korea Unl 2")));
        assert!(build_project(&project.folder).is_ok_and(|built| built.patterns == 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_project_made_from_a_profile_builds_the_same_patterns_and_scripts() {
        let dir = scratch("export");
        let _ = deploy(&dir);
        let source = dir.join("flappy.3dn");
        let _ = std::fs::write(&source, include_bytes!("../demo/flappyzaxnes.3dn"));
        let Ok(project) = export_project(&dir, "FlappyZaxNes", &source) else {
            panic!("the project was not made");
        };
        let original = description::dump(&source, true).unwrap_or_default();
        let rebuilt = description::dump(&project.profile_file, true).unwrap_or_default();
        // the first line names the profile, which has its own title
        let body = |text: &str| {
            text.lines().skip(1).collect::<Vec<_>>().join(
                "
",
            )
        };
        assert_ne!(body(&original).len(), 0);
        assert_eq!(body(&original), body(&rebuilt));
        let load = |path: &Path| {
            std::fs::read(path)
                .ok()
                .and_then(|bytes| zax_profiles::format::load(&bytes, None, None).ok())
                .map(|document| document.profile)
        };
        let (Some(before), Some(after)) = (load(&source), load(&project.profile_file)) else {
            panic!("a profile does not load");
        };
        assert_eq!(before.scripts.len(), after.scripts.len());
        for (a, b) in before.scripts.iter().zip(&after.scripts) {
            assert_eq!(
                (&a.name, a.enabled, &a.source, &a.tag_string),
                (&b.name, b.enabled, &b.source, &b.tag_string)
            );
        }
        assert_ne!(before.id, after.id);
        assert!(after.title.is_some_and(|title| title.ends_with(" (Lua)")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_prompt_names_the_folder_the_project_and_the_server() {
        let context = PromptContext {
            folder: PathBuf::from("data/editor"),
            game: Some(("Metal Force".to_owned(), "mf.nes".to_owned())),
            project: Some(PathBuf::from("data/editor/projects/Metal Force")),
            mcp_url: Some("http://127.0.0.1:8765/mcp".to_owned()),
            harness: None,
            rom_identity: Some("data CRC 1234abcd, mapper 4, NTSC".to_owned()),
            movie: None,
            french: true,
        };
        let prompt = agent_prompt(&context);
        assert!(prompt.contains("AGENT.md"));
        assert!(prompt.contains("projects"));
        assert!(prompt.contains("claude mcp add --transport http zaxnes http://127.0.0.1:8765/mcp"));
        assert!(prompt.contains(SOURCE_URL));
        assert!(prompt.contains("Jeu : Metal Force"));
    }
}
