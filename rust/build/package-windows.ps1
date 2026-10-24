<#
.SYNOPSIS
    Builds the Windows x64 zip of the Rust port: zaxnes.exe, profiles\README.txt, the licences and
    HOWTO-RUN.md. Run it from any folder: `powershell -ExecutionPolicy Bypass -File rust\build\package-windows.ps1`.
.DESCRIPTION
    The executable is built with `bundled`, which links SDL2 statically, so SDL2.dll is not needed
    next to it, and with the C runtime linked statically (`+crt-static` in rust\.cargo\config.toml),
    so neither VCRUNTIME140.dll nor the Visual C++ redistributable is needed: the exe stands alone.
    The release profile (fat LTO, one codegen unit, stripped) keeps it small. To check, run
    `dumpbin /dependents zaxnes.exe` from a Visual Studio prompt: only system DLLs (KERNEL32,
    USER32, GDI32, OPENGL32, ...) and no VCRUNTIME140 / MSVCP140 / api-ms-win-crt-*. Nothing is downloaded and no asset is produced: the folder must hold the files
    listed below and nothing else (REQ-PUB-002). The zip goes to rust\target\package.
#>
[CmdletBinding()]
param(
    [string]$Output = ''
)

$ErrorActionPreference = 'Stop'
$rust = Split-Path -Parent $PSScriptRoot
$root = Split-Path -Parent $rust

Push-Location $rust
try {
    cargo build --release -p zax-app --features bundled,wgpu
    if ($LASTEXITCODE -ne 0) { throw "package: cargo build failed ($LASTEXITCODE)" }
}
finally {
    Pop-Location
}

$version = (Select-String -Path (Join-Path $rust 'Cargo.toml') -Pattern '^version = "(.+?)"').Matches[0].Groups[1].Value
$name = "Zaxnes-$version-win-x64"
if ($Output -eq '') {
    $Output = Join-Path $rust "target\package\$name"
}
if (Test-Path $Output) { Remove-Item -Recurse -Force $Output }
New-Item -ItemType Directory -Force -Path $Output | Out-Null

Copy-Item (Join-Path $rust 'target\release\zaxnes.exe') (Join-Path $Output 'zaxnes.exe')
foreach ($document in 'LICENSE', 'HOWTO-RUN.md', 'ASSETS-LICENSES.md', 'THIRD-PARTY-NOTICES.md') {
    Copy-Item (Join-Path $root $document) (Join-Path $Output $document) -Force
}

# AMD-033 §3: an empty profiles folder next to zaxnes.exe, with a line saying what to put in it.
# It is the second folder the library scans, in every mode, and it is only ever read.
$profiles = Join-Path $Output 'profiles'
New-Item -ItemType Directory -Force -Path $profiles | Out-Null
Set-Content -Path (Join-Path $profiles 'README.txt') -Encoding utf8 `
    -Value "Déposez ici vos profils .3dn ; ils sont chargés au démarrage. Aucun profil n'est fourni."

$readme = @"
Zaxnes $version — émulateur NES avec rendu 3D
==========================================

Build Windows x64 autonome : un seul fichier, ni SDL2.dll ni Visual C++ Redistributable à installer. Lancez zaxnes.exe.

    zaxnes.exe
    zaxnes.exe --rom "jeu.nes" --profile "3D par défaut (sans profil)"
    zaxnes.exe --rom "jeu.nes" --profile "2D Mode"
    zaxnes.exe --help

AUCUN ASSET N'EST NÉCESSAIRE NI LIVRÉ. Les skyboxes et les sons d'interface sont produits par
le code ; la police de l'interface est une police déjà installée sur votre Windows (Segoe UI,
sinon Arial), lue au lancement et jamais redistribuée. Il n'y a pas de musique de menu.
Détail et licences : ASSETS-LICENSES.md.

AUCUNE ROM N'EST FOURNIE. Utilisez une ROM dont vous disposez légalement.

AUCUN PROFIL DE JEU N'EST FOURNI. La bibliothèque démarre avec deux entrées génériques :
« 2D Mode » (l'image de la console, sans conversion) et « 3D par défaut (sans profil) » (la
géométrie 3D par défaut, pour n'importe quelle ROM). Si vous disposez légalement d'un profil
« .3dn », importez-le par Fichier ▸ Importer un profil… : il est copié dans votre dossier de
données et apparaît dans la bibliothèque.

Zaxnes est une réimplémentation indépendante. Il n'est affilié ni à Geod Studio ni à 3dSen, et
ne contient aucun fichier provenant de ce programme.

Prérequis : Windows 10/11 x64, une carte graphique OpenGL 3.3 core. L'audio est facultatif.

Documentation : HOWTO-RUN.md (en français) — menus, raccourcis, options, emplacements de
sauvegarde et ligne de commande complète.

Licence : GNU GPL v3, fichier LICENSE. Le code tiers et les paquets utilisés sont listés
dans THIRD-PARTY-NOTICES.md, les assets dans ASSETS-LICENSES.md.

Vérification non interactive :

    zaxnes.exe --smoke-test --smoke-frames 120    rend 120 images puis quitte avec 0
    zaxnes.exe --smoke-emulated 3600 --rom ...    joue 60 s puis quitte avec 0
    zaxnes.exe --list-profiles                    liste les entrées de la bibliothèque
"@
Set-Content -Path (Join-Path $Output 'README.txt') -Value $readme -Encoding utf8

# The folder ships the executable, the documents and profiles\README.txt, and nothing else.
$expected = @('zaxnes.exe', 'LICENSE', 'HOWTO-RUN.md', 'ASSETS-LICENSES.md',
    'THIRD-PARTY-NOTICES.md', 'README.txt', 'profiles\README.txt')
$shipped = @(Get-ChildItem -Recurse -File $Output | ForEach-Object {
        $_.FullName.Substring($Output.Length + 1)
    })
$unexpected = @($shipped | Where-Object { $expected -notcontains $_ })
if ($unexpected.Count -gt 0) {
    throw "package: unexpected files in the package: $($unexpected -join ', ')"
}

# The package must start: the help and the catalogue run without a window.
& (Join-Path $Output 'zaxnes.exe') --help | Out-Null
if ($LASTEXITCODE -ne 0) { throw "package: zaxnes.exe --help exited with $LASTEXITCODE" }
& (Join-Path $Output 'zaxnes.exe') --list-profiles | Out-Null
if ($LASTEXITCODE -ne 0) { throw "package: zaxnes.exe --list-profiles exited with $LASTEXITCODE" }

$zip = "$Output.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $Output '*') -DestinationPath $zip
Write-Host "package: $zip"
