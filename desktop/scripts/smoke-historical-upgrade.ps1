# AutoTrace historical installer acceptance on an EPHEMERAL Windows Actions VM.
# Tests installed NSIS binaries, a real persisted synthetic SQLite v1 file,
# and rollback between two code revisions. No customer data or API secrets.
param(
    [Parameter(Mandatory=$true)][string]$PreviousInstaller,
    [Parameter(Mandatory=$true)][string]$CandidateInstaller
)
$ErrorActionPreference = "Stop"
$previous = (Resolve-Path -LiteralPath $PreviousInstaller -ErrorAction Stop).Path
$candidate = (Resolve-Path -LiteralPath $CandidateInstaller -ErrorAction Stop).Path
if ($previous -eq $candidate) { throw "Historical and candidate installers are identical paths" }
$root = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$dataDir = Join-Path $env:LOCALAPPDATA "ar.com.autotrace.desktop"
$database = Join-Path $dataDir "autotrace.sqlite3"
$evidence = Join-Path $env:RUNNER_TEMP "autotrace-historical-db.sha256"
$verifier = Join-Path $root "desktop/scripts/check-upgrade-v1.py"

function Install-Quiet([string] $path, [string] $label) {
    if (-not (Test-Path -LiteralPath $path)) { throw "$label installer missing" }
    Write-Host "$label installer sha256=$((Get-FileHash $path -Algorithm SHA256).Hash)"
    $p = Start-Process -FilePath $path -ArgumentList "/S" -PassThru
    if (-not $p.WaitForExit(180000)) {
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
        throw "$label installation timed out"
    }
    if ($p.ExitCode -ne 0) { throw "$label NSIS installer exit code $($p.ExitCode)" }
}

function Verify-SyntheticState([string] $phase) {
    if (-not (Test-Path -LiteralPath $database)) { throw "$phase: persisted SQLite disappeared" }
    & python $verifier verify $database $evidence
    if ($LASTEXITCODE -ne 0) { throw "$phase: historical SQLite lost integrity" }
    Write-Host "$phase: v1 SQLite and monetary balances survived"
}

if (Test-Path -LiteralPath $database) {
    throw "Runner contains an existing AutoTrace workshop database; refusing to run historical upgrade smoke"
}
if (Test-Path -LiteralPath $evidence) { throw "Runner is not clean: evidence already exists" }

Install-Quiet $previous "PREVIOUS"
New-Item -Path $dataDir -ItemType Directory -Force | Out-Null
& python $verifier prepare $database $evidence
if ($LASTEXITCODE -ne 0) { throw "Could not prepare synthetic historical v1 database" }
Verify-SyntheticState "PRIOR-TO-UPGRADE"

Install-Quiet $candidate "CANDIDATE"
Verify-SyntheticState "AFTER-UPGRADE"

# This is a code-revision rollback when both installers carry the same
# declared version. Do NOT claim this proves every historical semver downgrade.
Install-Quiet $previous "ROLLBACK"
Verify-SyntheticState "AFTER-ROLLBACK"

$installed = Get-ChildItem -Path $env:LOCALAPPDATA -Filter "autotrace-desktop.exe" -File -Recurse -ErrorAction SilentlyContinue |
    Sort-Object FullName -Unique
if (@($installed).Count -ne 1) {
    throw "Expected one installed AutoTrace executable, found $(@($installed).Count)"
}
$uninstall = Join-Path $installed[0].DirectoryName "uninstall.exe"
if (-not (Test-Path $uninstall)) { throw "Historical uninstall.exe not found" }
$p = Start-Process -FilePath $uninstall -ArgumentList "/S" -PassThru
if (-not $p.WaitForExit(180000)) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    throw "Uninstall timeout"
}
if ($p.ExitCode -ne 0) { throw "Uninstaller exit code $($p.ExitCode)" }
Verify-SyntheticState "AFTER-UNINSTALL"
Write-Host "HISTORICAL NSIS PASS: previous code revision -> candidate -> rollback -> uninstall preserved synthetic SQLite v1, revision, ledger, SHA-256."
