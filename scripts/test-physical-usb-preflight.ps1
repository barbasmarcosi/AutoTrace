#Requires -Version 5.1
# Runs on an ephemeral Windows runner without a physical USB device.
# Only NEGATIVE preflight paths are asserted. Never claim U2/U3 PASS.
$ErrorActionPreference = 'Stop'
$rootBase = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
$script = Join-Path $PSScriptRoot 'collect-physical-recovery-evidence.ps1'
$root = Join-Path $rootBase ("autotrace-usb-negative-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $root -Force | Out-Null
try {
    $primary = Join-Path $root 'primary'
    New-Item -ItemType Directory -Path $primary | Out-Null
    $evidence = Join-Path $root 'should-not-exist.json'
    $fake = Join-Path $root 'fake.atbackup.age'
    $realHeader = Join-Path $root 'synthetic-header.atbackup.age'
    function Assert-Rejected([string]$Case,[string]$Primary,[string]$Archive,[string]$Match) {
        $rejected = $false
        try {
            & $script -PrimaryDataDirectory $Primary -PortableArchive $Archive -EvidencePath $evidence | Out-Null
        } catch {
            if ($_.Exception.Message -notmatch $Match) {
                throw "Wrong reason for $Case; expected $Match"
            }
            $rejected = $true
        }
        if (-not $rejected) { throw "$Case unexpectedly passed the USB preflight" }
        if (Test-Path -LiteralPath $evidence) {
            throw "$Case emitted a physical acceptance record on failure"
        }
        Write-Host "NEGATIVE PASS: $Case"
    }
    [System.IO.File]::WriteAllBytes($fake, [byte[]](0..127))
    Assert-Rejected 'renamed plaintext or random bytes' $primary $fake 'cabecera cifrada'
    [System.IO.File]::WriteAllText($realHeader, ("age-encryption.org/v1" + [char]10 + "-> scrypt test-salt-stub 18" + [char]10 + ('X' * 150)))
    Assert-Rejected 'same physical drive' $primary $realHeader 'misma unidad'
    $wrongExt = Join-Path $root 'fake.txt'
    Copy-Item -LiteralPath $realHeader -Destination $wrongExt
    Assert-Rejected 'invalid extension' $primary $wrongExt '\.atbackup\.age'
    $junction = Join-Path $root 'junction-primary'
    New-Item -ItemType Junction -Path $junction -Target $primary -ErrorAction Stop | Out-Null
    Assert-Rejected 'Windows junction redirection' $junction $realHeader 'redirecciones'
    Write-Host 'USB PREFLIGHT NEGATIVE SUITE PASS: no physical USB was accessed and U0-U7 remain PENDING.'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
