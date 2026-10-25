#Requires -Version 5.1
<#
Checks the AutoTrace Windows installer BEFORE installation. No networking,
customer records, recovery phrase, OAuth credentials, or EXE execution.
Obtain ExpectedCommit from the TRUSTED GitHub PR/Actions page, not the JSON.
Example:
  .\verify-acceptance-installer.ps1 -Installer .\AutoTrace_0.1.1_x64-setup.exe -Manifest .\autotrace-release-manifest.json -ExpectedCommit YOUR_FULL_40_HEX_SHA -EvidenceFile .\installer-preflight.json
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Installer,
    [Parameter(Mandatory=$true)][string]$Manifest,
    [Parameter(Mandatory=$true)][string]$ExpectedCommit,
    [string]$ExpectedRunId,
    [string]$EvidenceFile
)
$ErrorActionPreference = 'Stop'
$installerFile=Get-Item -LiteralPath $Installer -ErrorAction Stop
$manifestFile=Get-Item -LiteralPath $Manifest -ErrorAction Stop
if ($installerFile.PSIsContainer -or $manifestFile.PSIsContainer) {
    throw "Se necesitan dos archivos reales: instalador y manifiesto."
}
if ($ExpectedCommit -cnotmatch '^[a-f0-9]{40}$') {
    throw "Indicá el SHA completo de 40 caracteres leído de GitHub."
}
if ($manifestFile.Length -gt 16384 -or $manifestFile.Length -lt 80) {
    throw "Manifiesto ausente o demasiado grande."
}
$record=Get-Content -LiteralPath $manifestFile.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
if ($record.schema -cne 'autotrace-installer-preflight-v1' -or
    $record.repository -cne 'barbasmarcosi/AutoTrace' -or
    $record.commitSha -cne $ExpectedCommit -or
    $record.installerKind -cne 'windows-nsis-preview' -or
    $record.signed -ne $false) {
    throw "Identidad, revisión, tipo o firma del paquete no coinciden con el candidato autorizado."
}
# The run ID is metadata, not a signature. When supplied, the ID must be
# independently read from the GitHub Actions run that produced this artifact.
if ([string]$record.workflowRunId -cnotmatch '^[1-9][0-9]{0,19}$') {
    throw "El manifiesto no incluye un ID válido de ejecución de GitHub Actions."
}
if ($ExpectedRunId -and
    ($ExpectedRunId -cnotmatch '^[1-9][0-9]{0,19}$' -or
     [string]$record.workflowRunId -cne $ExpectedRunId)) {
    throw "El manifiesto no corresponde a la ejecución de Actions indicada."
}
if ($record.installerName -cne $installerFile.Name -or
    $record.installerName -notmatch '^AutoTrace_[0-9]+\.[0-9]+\.[0-9]+_x64-setup\.exe$') {
    throw "El nombre del instalador no coincide con el manifiesto."
}
if ($record.installerSha256 -cnotmatch '^[a-f0-9]{64}$') {
    throw "El checksum del manifiesto no es válido."
}
if ($installerFile.Length -lt 100000 -or
    $installerFile.Length -ne [long]$record.installerBytes) {
    throw "El tamaño del instalador no coincide."
}
# The independent SHA-256 sidecar is part of the distributed acceptance kit.
# Reject a missing, mismatched or multi-line checksum even if the manifest
# alone appears internally consistent; neither file is a digital signature.
$checksumFile = Get-Item -LiteralPath "$($installerFile.FullName).sha256.txt" -ErrorAction Stop
if ($checksumFile.PSIsContainer -or $checksumFile.Length -lt 66 -or $checksumFile.Length -gt 256) {
    throw "Falta el archivo SHA-256 del instalador o su tamaño no es válido."
}
$sidecar = Get-Content -LiteralPath $checksumFile.FullName -Raw -Encoding ASCII
$sidecarLine = $sidecar -creplace "(\r\n|\n)\z", ""
$expectedSidecar = "$($record.installerSha256) *$($installerFile.Name)"
if ($sidecarLine -cne $expectedSidecar) {
    throw "El .sha256.txt no coincide con el instalador y el manifiesto."
}
$stream=[System.IO.File]::OpenRead($installerFile.FullName)
try {
    $header=New-Object byte[] 2
    if ($stream.Read($header,0,2) -ne 2 -or
        $header[0] -ne 0x4D -or $header[1] -ne 0x5A) {
        throw "El instalador no es un ejecutable PE de Windows."
    }
} finally {
    $stream.Dispose()
}
$hash=(Get-FileHash -LiteralPath $installerFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
if ($hash -cne $record.installerSha256) {
    throw "SHA-256 INCORRECTO: no ejecutes este instalador."
}
$signature=(Get-AuthenticodeSignature -LiteralPath $installerFile.FullName -ErrorAction Stop)
if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned) {
    throw "La firma Authenticode no coincide con el preview declarado SIN FIRMA. Revisá el origen."
}
Write-Host "PREFLIGHT PASS: SHA-256, archivo .sha256.txt, tamaño, PE, commit y run verificados."
Write-Warning "Instalador SIN FIRMA DIGITAL. El JSON no es una firma criptográfica; verificá el origen en GitHub."
Write-Warning "Esto no prueba instalación, recuperación en PC B ni Google. U0–U7 siguen PENDIENTES."
if ($EvidenceFile) {
    $target=[System.IO.Path]::GetFullPath($EvidenceFile)
    if (Test-Path -LiteralPath $target) {
        throw "El informe ya existe; no se sobrescribe evidencia."
    }
    $dir=[System.IO.Path]::GetDirectoryName($target)
    if (-not (Test-Path -LiteralPath $dir)) {
        throw "Creá la carpeta de evidencias antes de ejecutar el comando."
    }
    $evidence=[ordered]@{
        evidenceType='autotrace-installer-preflight-v1'
        collectedAtUtc=[DateTime]::UtcNow.ToString('o')
        repository='barbasmarcosi/AutoTrace'
        commitSha=$ExpectedCommit
        workflowRunId=[string]$record.workflowRunId
        checksumSidecarVerified=$true
        installerSha256=$hash
        installerBytes=$installerFile.Length
        signed=$false
        state='INSTALLER_PREFLIGHT_ONLY'
        physicalInstallationPassed=$false
        physicalRecoveryPassed=$false
        realGoogleOAuthPassed=$false
        note='Verificación local sin datos personales; no implica U0–U7 PASS.'
    }
    [System.IO.File]::WriteAllText($target,($evidence | ConvertTo-Json -Depth 4),
        (New-Object System.Text.UTF8Encoding($false)))
    Write-Host "Evidencia no sensible generada sin sobrescritura."
}
