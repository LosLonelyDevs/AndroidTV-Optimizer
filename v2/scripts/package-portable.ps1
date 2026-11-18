[CmdletBinding()]
param(
    [string]$BinaryPath = (Join-Path $PSScriptRoot '../target/release/atv-optimizer.exe'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../target/portable')
)

$ErrorActionPreference = 'Stop'
$v2Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$config = Get-Content -LiteralPath (Join-Path $v2Root 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$version = $config.version
if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') { throw 'Invalid application version' }
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$header = [System.IO.File]::OpenRead($binary)
try {
    if ($header.ReadByte() -ne 77 -or $header.ReadByte() -ne 90) { throw 'Expected a Windows PE executable' }
} finally { $header.Dispose() }

$output = [System.IO.Path]::GetFullPath($OutputDirectory)
[System.IO.Directory]::CreateDirectory($output) | Out-Null
$name = "Android-TV-Optimizer_${version}_windows_x64_portable"
$stage = Join-Path $output "$name-stage-$([guid]::NewGuid().ToString('N'))"
[System.IO.Directory]::CreateDirectory($stage) | Out-Null
try {
    Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'atv-optimizer.exe')
    foreach ($resource in $config.bundle.resources) {
        if ($resource -notmatch '^resources/[A-Za-z0-9._-]+$') {
            throw "Unsupported portable resource mapping: $resource"
        }
        $destination = Join-Path $stage $resource
        [System.IO.Directory]::CreateDirectory((Split-Path $destination)) | Out-Null
        Copy-Item -LiteralPath (Join-Path $v2Root "src-tauri/$resource") -Destination $destination
    }
    Copy-Item -LiteralPath (Join-Path $v2Root 'portable/portable.txt') -Destination $stage
    Copy-Item -LiteralPath (Join-Path $v2Root '../LICENSE') -Destination $stage
    Get-ChildItem -LiteralPath (Split-Path $binary) -Filter '*.dll' -File | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $stage
    }
    $archive = Join-Path $output "$name.zip"
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $archive -Force
    $checksum = "$archive.sha256"
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText($checksum, "$hash  $name.zip`n", [System.Text.UTF8Encoding]::new($false))
    if ($env:GITHUB_OUTPUT) {
        Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value "archive=$archive"
        Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value "checksum=$checksum"
    }
    Write-Output $archive
    Write-Output $checksum
} finally {
    $resolvedStage = [System.IO.Path]::GetFullPath($stage)
    if (-not $resolvedStage.StartsWith($output + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing cleanup outside the portable output directory'
    }
    Remove-Item -LiteralPath $resolvedStage -Recurse -Force
}
