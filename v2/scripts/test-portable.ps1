$ErrorActionPreference = 'Stop'
$scratch = Join-Path ([System.IO.Path]::GetTempPath()) "atv-portable-test-$([guid]::NewGuid().ToString('N'))"
[System.IO.Directory]::CreateDirectory($scratch) | Out-Null
try {
    $binary = Join-Path $scratch 'fixture.exe'
    [System.IO.File]::WriteAllBytes($binary, [byte[]](77, 90, 0, 1))
    $output = Join-Path $scratch 'output'
    & (Join-Path $PSScriptRoot 'package-portable.ps1') -BinaryPath $binary -OutputDirectory $output
    $archive = Get-ChildItem -LiteralPath $output -Filter '*.zip' -File
    if (@($archive).Count -ne 1) { throw 'Expected one portable archive' }
    $unpacked = Join-Path $scratch 'unpacked'
    Expand-Archive -LiteralPath $archive.FullName -DestinationPath $unpacked
    foreach ($file in @('atv-optimizer.exe', 'portable.txt', 'LICENSE', 'resources/scrcpy-server-v3.1', 'resources/direct-adb-MIT-LICENSE.txt', 'resources/native-discovery-MIT-LICENSES.txt')) {
        if (-not (Test-Path -LiteralPath (Join-Path $unpacked $file) -PathType Leaf)) { throw "Missing $file" }
    }
    $checksum = Get-Content -LiteralPath ($archive.FullName + '.sha256') -Raw
    if (-not $checksum.StartsWith((Get-FileHash -LiteralPath $archive.FullName -Algorithm SHA256).Hash.ToLowerInvariant())) {
        throw 'Checksum does not match archive'
    }
    [System.IO.File]::WriteAllBytes($binary, [byte[]](0, 1))
    $rejected = $false
    try { & (Join-Path $PSScriptRoot 'package-portable.ps1') -BinaryPath $binary -OutputDirectory $output } catch { $rejected = $true }
    if (-not $rejected) { throw 'Invalid executable was accepted' }
    Write-Output 'PASS: portable resources, marker, checksum and invalid executable rejection'
} finally {
    $root = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not ([System.IO.Path]::GetFullPath($scratch)).StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing cleanup outside temporary directory'
    }
    Remove-Item -LiteralPath $scratch -Recurse -Force
}
