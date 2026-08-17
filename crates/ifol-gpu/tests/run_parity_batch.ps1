param(
    [Parameter(Position = 0)]
    [string[]]$Cases = @('TC23'),

    [switch]$OpenWeb,

    [switch]$SkipPreview
)

$ErrorActionPreference = 'Stop'
$crateDir = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$workspaceDir = (Resolve-Path (Join-Path $crateDir '..\..')).Path

$normalizedCases = @(
    $Cases |
        ForEach-Object { $_.Trim().ToUpperInvariant() } |
        Where-Object { $_ -ne '' } |
        Select-Object -Unique
)

if ($normalizedCases.Count -eq 0) {
    throw 'No test cases were selected.'
}

$cargoArgs = @('test', '-p', 'ifol-gpu')
foreach ($case in $normalizedCases) {
    if ($case -notmatch '^TC[0-9]+(?:\.5)?$') {
        throw "Invalid test case name '$case'. Use names such as TC17, TC23 or TC08.5."
    }

    $suffix = $case.Substring(2).Replace('.', '_').ToLowerInvariant()
    $cargoArgs += @('--test', "tc${suffix}_desktop")
}
$cargoArgs += @('--', '--test-threads=1')

Write-Host "Desktop batch: $($normalizedCases -join ', ')"
Push-Location $workspaceDir
try {
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Desktop batch failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

$query = [string]::Join(',', $normalizedCases)
$url = "http://localhost:8080/?cases=$query&skip_probe=1"
if ($SkipPreview) {
    $url += '&skip_preview=1'
}

Write-Host "Web batch: $url"
if ($OpenWeb) {
    Start-Process $url
}
