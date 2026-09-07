param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDirectory,
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function ConvertTo-FilmSlug([string]$Value) {
    $normalized = $Value.Normalize([Text.NormalizationForm]::FormKD).ToLowerInvariant()
    $builder = [Text.StringBuilder]::new()
    foreach ($character in $normalized.ToCharArray()) {
        $category = [Globalization.CharUnicodeInfo]::GetUnicodeCategory($character)
        if ($category -eq [Globalization.UnicodeCategory]::NonSpacingMark) {
            continue
        }
        if (($character -ge 'a' -and $character -le 'z') -or ($character -ge '0' -and $character -le '9')) {
            [void]$builder.Append($character)
        } else {
            [void]$builder.Append('-')
        }
    }
    return (($builder.ToString() -replace '-+', '-').Trim('-'))
}

$sourceRoot = (Resolve-Path -LiteralPath $SourceDirectory).Path
$projectRootPath = (Resolve-Path -LiteralPath $ProjectRoot).Path
$catalogPath = Join-Path $projectRootPath 'src\data\film-catalog.csv'
$targetRoot = Join-Path $projectRootPath 'public\film-stocks'
$catalog = (Get-Content -LiteralPath $catalogPath | Select-Object -Skip 1) |
    ConvertFrom-Csv -Delimiter ';'

$sourceFiles = @(Get-ChildItem -LiteralPath $sourceRoot -File)
$sourceHashesBefore = @{}
$sourcesBySlug = @{}
foreach ($sourceFile in $sourceFiles) {
    $sourceHashesBefore[$sourceFile.FullName] = (Get-FileHash -LiteralPath $sourceFile.FullName -Algorithm SHA256).Hash
    $slug = ConvertTo-FilmSlug $sourceFile.BaseName
    if ($sourcesBySlug.ContainsKey($slug)) {
        throw "源图片名称规范化后发生冲突：$($sourcesBySlug[$slug].Name) / $($sourceFile.Name)"
    }
    $sourcesBySlug[$slug] = $sourceFile
}

# 这三张源图沿用过往的 OROW 拼写，仅用于定位桌面源文件。
$sourceAliases = @{
    'orwo-original-wolfen-nc400' = 'orow-original-wolfen-nc400'
    'orwo-original-wolfen-nc500' = 'orow-original-wolfen-nc500'
    'orwo-original-wolfen-un-54' = 'orow-original-wolfen-un-54'
}

$plan = @()
$missing = @()
$conflicts = @()
foreach ($film in $catalog) {
    $targetName = $film.image
    $targetSlug = [IO.Path]::GetFileNameWithoutExtension($targetName)
    $expectedSlug = ConvertTo-FilmSlug "$($film.brand) $($film.name)"
    if ($targetSlug -ne $expectedSlug) {
        throw "目录中的目标文件名不符合品牌与型号：$($film.brand) $($film.name) -> $targetName"
    }
    $sourceSlug = if ($sourceAliases.ContainsKey($targetSlug)) {
        $sourceAliases[$targetSlug]
    } else {
        $targetSlug
    }
    if (-not $sourcesBySlug.ContainsKey($sourceSlug)) {
        $missing += "$($film.brand) $($film.name)"
        continue
    }
    $sourceFile = $sourcesBySlug[$sourceSlug]
    $targetPath = Join-Path $targetRoot $targetName
    $sourceHash = $sourceHashesBefore[$sourceFile.FullName]
    if (Test-Path -LiteralPath $targetPath) {
        $targetHash = (Get-FileHash -LiteralPath $targetPath -Algorithm SHA256).Hash
        if ($targetHash -ne $sourceHash) {
            $conflicts += "$($sourceFile.Name) -> $targetName"
            continue
        }
    }
    $plan += [pscustomobject]@{
        Film = "$($film.brand) $($film.name)"
        Source = $sourceFile.FullName
        Target = $targetPath
        Exists = Test-Path -LiteralPath $targetPath
    }
}

if ($missing.Count -gt 0 -or $conflicts.Count -gt 0) {
    throw "图片预检失败。未匹配：$($missing -join '；')；内容冲突：$($conflicts -join '；')"
}

$copied = 0
$skipped = 0
foreach ($item in $plan) {
    if ($item.Exists) {
        $skipped++
    } else {
        Copy-Item -LiteralPath $item.Source -Destination $item.Target
        $copied++
    }
}

foreach ($sourceFile in $sourceFiles) {
    $afterHash = (Get-FileHash -LiteralPath $sourceFile.FullName -Algorithm SHA256).Hash
    if ($afterHash -ne $sourceHashesBefore[$sourceFile.FullName]) {
        throw "源图片在同步过程中发生变化：$($sourceFile.FullName)"
    }
}

[pscustomobject]@{
    SourceFiles = $sourceFiles.Count
    CatalogBindings = $catalog.Count
    Copied = $copied
    SkippedSameHash = $skipped
    Missing = $missing.Count
    Conflicts = $conflicts.Count
    SourceHashesUnchanged = $true
    TargetDirectory = $targetRoot
}
