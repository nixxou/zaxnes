<#
.SYNOPSIS
    Checks that the round-1 traceability matrix covers every requirement id of the spec.

.DESCRIPTION
    Collects every REQ-<MOD>-NNN identifier that appears in spec/*.md and in spec/changes/*.md,
    collects the ids of the first column of verify/round1/traceability.csv, and reports the three
    differences: ids of the spec missing from the matrix, ids of the matrix absent from the spec,
    and ids the matrix lists more than once.

    Documentation task of phase P6 (PLAN.md). The script changes nothing; with -Report it writes
    a Markdown report.

.PARAMETER Root
    The cleanroom root (the folder holding spec/, impl/ and verify/). Found automatically from
    the script's location otherwise.

.PARAMETER Report
    Path of a Markdown report to write. Nothing is written without it.

.PARAMETER FailOnDifference
    Exit with 1 when a difference other than the known false positives remains.

.EXAMPLE
    powershell -File impl/tools/traceability-check.ps1
    powershell -File impl/tools/traceability-check.ps1 -Report ../spec/TRACEABILITY-CHECK-P6.md
#>

[CmdletBinding()]
param(
    [string]$Root,
    [string]$Report,
    [switch]$FailOnDifference
)

$ErrorActionPreference = 'Stop'

if (-not $Root) { $Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot) }

$specDir = Join-Path $Root 'spec'
$changesDir = Join-Path $specDir 'changes'
$matrixPath = Join-Path $Root 'verify\round1\traceability.csv'

foreach ($path in @($specDir, $changesDir, $matrixPath)) {
    if (-not (Test-Path $path)) { throw "traceability-check: $path was not found." }
}

# A requirement id. The spec also writes wildcards ('REQ-INP-03x', 'REQ-INP-04x') when it names a
# family rather than one requirement; the trailing letter is not part of the id, so the pattern
# below matches their numeric stem and those two stems are known false positives.
$pattern = 'REQ-[A-Z0-9]+-[0-9]+'

function Get-Ids {
    param([string[]]$Files)

    $found = @{}
    foreach ($file in $Files) {
        foreach ($match in [regex]::Matches((Get-Content -Raw -LiteralPath $file), $pattern)) {
            $id = $match.Value
            if (-not $found.ContainsKey($id)) { $found[$id] = [System.Collections.Generic.List[string]]::new() }
            $name = Split-Path -Leaf $file
            if (-not $found[$id].Contains($name)) { $found[$id].Add($name) }
        }
    }

    return $found
}

# The frozen documents only: the v1.2 index and this script's own report cite requirement ids
# too, and counting them would make the check circular.
$specFiles = @(Get-ChildItem -LiteralPath $specDir -Filter 'SPEC-*.md' -File |
    Where-Object { $_.Name -match '^SPEC-\d' } |
    ForEach-Object FullName)
$specFiles += (Join-Path $specDir 'DECISIONS-R0.md')
$changeFiles = @(Get-ChildItem -LiteralPath $changesDir -Filter '*.md' -File | ForEach-Object FullName)

$inSpec = Get-Ids -Files $specFiles
$inChanges = Get-Ids -Files $changeFiles

$expected = @{}
foreach ($id in $inSpec.Keys) { $expected[$id] = $inSpec[$id] }
foreach ($id in $inChanges.Keys) {
    if ($expected.ContainsKey($id)) {
        foreach ($f in $inChanges[$id]) { if (-not $expected[$id].Contains($f)) { $expected[$id].Add($f) } }
    } else {
        $expected[$id] = $inChanges[$id]
    }
}

# First column of the matrix, header line dropped, BOM tolerated.
$rows = Get-Content -LiteralPath $matrixPath | Select-Object -Skip 1
$counts = @{}
foreach ($row in $rows) {
    $id = ($row -split ',')[0].Trim().TrimStart([char]0xFEFF)
    if ($id -notmatch "^$pattern$") { continue }
    if ($counts.ContainsKey($id)) { $counts[$id]++ } else { $counts[$id] = 1 }
}

$inMatrix = [System.Collections.Generic.HashSet[string]]::new([string[]]$counts.Keys)

$missing = @($expected.Keys | Where-Object { -not $inMatrix.Contains($_) } | Sort-Object)
$orphan = @($counts.Keys | Where-Object { -not $expected.ContainsKey($_) } | Sort-Object)
$duplicate = @($counts.Keys | Where-Object { $counts[$_] -gt 1 } | Sort-Object)

# Known false positives: wildcard families the spec writes as 'REQ-INP-03x' / 'REQ-INP-04x', and
# REQ-E3D-512, a dead cross-reference of SPEC-04B REQ-E3D-406 (see spec/changes/Q-063.md).
$known = @('REQ-INP-03', 'REQ-INP-04', 'REQ-E3D-512')
$realMissing = @($missing | Where-Object { $known -notcontains $_ })

Write-Host "traceability-check: $($expected.Count) ids in spec/ and spec/changes/, $($counts.Count) rows in the matrix."
Write-Host "  missing from the matrix : $($missing.Count) ($($realMissing.Count) once the known false positives are set aside)"
Write-Host "  absent from the spec    : $($orphan.Count)"
Write-Host "  listed twice or more    : $($duplicate.Count)"

foreach ($id in $missing) {
    $mark = if ($known -contains $id) { 'known false positive' } else { 'MISSING' }
    Write-Host "    $id  [$mark]  cited in: $($expected[$id] -join ', ')"
}
foreach ($id in $orphan) { Write-Host "    $id  [ORPHAN in the matrix]" }
foreach ($id in $duplicate) { Write-Host "    $id  [$($counts[$id]) rows]" }

if ($Report) {
    $byDocument = $counts.Keys | Group-Object { ($_ -split '-')[1] } | Sort-Object Name
    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add('# Contrôle de couverture de la matrice de traçabilité (phase P6)')
    $lines.Add('')
    $lines.Add("Version : 1.0 | $(Get-Date -Format 'yyyy-MM-dd') | **Documentation seulement**.")
    $lines.Add('')
    $lines.Add('Produit par `impl/tools/traceability-check.ps1`, qui relève tous les identifiants')
    $lines.Add('`REQ-<MOD>-NNN` de `spec/*.md` et de `spec/changes/*.md` et les compare à la première')
    $lines.Add('colonne de `verify/round1/traceability.csv`. Voir aussi `SPEC-v1.2-INDEX.md`.')
    $lines.Add('')
    $lines.Add('## 1. Résultat')
    $lines.Add('')
    $lines.Add('| Grandeur | Valeur |')
    $lines.Add('|---|---|')
    $lines.Add('| Identifiants relevés dans `spec/` et `spec/changes/` | ' + $expected.Count + ' |')
    $lines.Add("| Lignes d'exigence dans la matrice | $($counts.Count) |")
    $lines.Add("| Présents dans la spec, **absents de la matrice** | $($missing.Count) dont $($realMissing.Count) réel(s) |")
    $lines.Add("| Présents dans la matrice, **absents de la spec** | $($orphan.Count) |")
    $lines.Add("| Listés plus d'une fois dans la matrice | $($duplicate.Count) |")
    $lines.Add('')
    $verdict = if ($realMissing.Count -eq 0 -and $orphan.Count -eq 0 -and $duplicate.Count -eq 0) {
        '**Couverture complète** : chaque exigence de la spécification et des amendements a exactement une ligne dans la matrice, et la matrice ne contient aucun identifiant qui ne soit pas dans la spécification.'
    } else {
        '**Écarts à instruire** : voir le §2.'
    }
    $lines.Add($verdict)
    $lines.Add('')
    $lines.Add('## 2. Différences relevées')
    $lines.Add('')
    if ($missing.Count -eq 0 -and $orphan.Count -eq 0 -and $duplicate.Count -eq 0) {
        $lines.Add('Aucune.')
    } else {
        $lines.Add('| Identifiant | Différence | Cité dans | Lecture |')
        $lines.Add('|---|---|---|---|')
        foreach ($id in $missing) {
            $where = ($expected[$id] -join ', ')
            $reading = switch ($id) {
                'REQ-INP-03' { 'Faux positif : la spec écrit `REQ-INP-03x` pour désigner la famille REQ-INP-030/031 (AMD-020, Q-048). Ce n''est pas un identifiant.' }
                'REQ-INP-04' { 'Faux positif : la spec écrit `REQ-INP-04x` pour désigner la famille de la capture, REQ-INP-042/045/047 (AMD-015, Q-033). Ce n''est pas un identifiant.' }
                'REQ-E3D-512' { 'Renvoi mort : REQ-E3D-406 cite « REQ-E3D-504, REQ-E3D-512 », mais REQ-E3D-512 n''est définie nulle part (le maillage local d''une tuile est REQ-E3D-500 à -507 et -520 à -524). Sans effet normatif ; consigné dans `changes/Q-063.md`. Le texte gelé n''est pas modifié.' }
                default { 'À instruire.' }
            }
            $lines.Add("| ``$id`` | absente de la matrice | $where | $reading |")
        }
        foreach ($id in $orphan) { $lines.Add("| ``$id`` | absente de la spec | matrice | À instruire. |") }
        foreach ($id in $duplicate) { $lines.Add("| ``$id`` | $($counts[$id]) lignes dans la matrice | matrice | À instruire. |") }
    }
    $lines.Add('')
    $lines.Add('## 3. Répartition de la matrice par document')
    $lines.Add('')
    $lines.Add('| Famille | Document | Lignes |')
    $lines.Add('|---|---|---|')
    $documents = @{
        'ARCH' = 'SPEC-01'; 'CORE' = 'SPEC-02'; 'PROF' = 'SPEC-03'
        'E3D' = 'SPEC-04 / SPEC-04B'; 'LUA' = 'SPEC-05'; 'SCN' = 'SPEC-06'
        'LOOP' = 'SPEC-07'; 'INP' = 'SPEC-08'; 'PERS' = 'SPEC-09'
        'UI' = 'SPEC-10 (+ AMD-010)'; 'AUD' = 'SPEC-11'; 'ACC' = 'SPEC-12'
    }
    foreach ($group in $byDocument) {
        $doc = if ($documents.ContainsKey($group.Name)) { $documents[$group.Name] } else { '—' }
        $lines.Add("| REQ-$($group.Name)-… | $doc | $($group.Count) |")
    }
    $lines.Add("| **Total** | | **$($counts.Count)** |")
    $lines.Add('')
    $lines.Add('## 4. Reproduire')
    $lines.Add('')
    $lines.Add('```')
    $lines.Add('powershell -File impl/tools/traceability-check.ps1 -Report spec/TRACEABILITY-CHECK-P6.md')
    $lines.Add('```')

    $directory = Split-Path -Parent $Report
    if ($directory -and -not (Test-Path $directory)) { New-Item -ItemType Directory -Force -Path $directory | Out-Null }
    Set-Content -Path $Report -Value $lines -Encoding utf8
    Write-Host "traceability-check: report written to $Report"
}

if ($FailOnDifference -and ($realMissing.Count -gt 0 -or $orphan.Count -gt 0 -or $duplicate.Count -gt 0)) {
    exit 1
}

exit 0
