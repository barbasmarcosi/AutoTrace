#Requires -Version 5.1
<#
Collects NON-PII evidence for the G2 physical USB preflight.
It does not read the encrypted archive, its recovery phrase, or workshop data.
Passing this check is NOT proof of restore or Google OAuth.
Example:
.\\scripts\\collect-physical-recovery-evidence.ps1 -PrimaryDataDirectory "$env:LOCALAPPDATA\\ar.com.autotrace.desktop" -PortableArchive "E:\\Backups\\AutoTrace_123.atbackup.age"
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$PrimaryDataDirectory,
    [Parameter(Mandatory=$true)][string]$PortableArchive,
    [string]$EvidencePath = ".\\autotrace-usb-preflight.json"
)
$ErrorActionPreference = 'Stop'
$primary = (Resolve-Path -LiteralPath $PrimaryDataDirectory -ErrorAction Stop).Path
$archive = (Resolve-Path -LiteralPath $PortableArchive -ErrorAction Stop).Path
if (-not $archive.EndsWith(".atbackup.age", [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "La evidencia solo acepta un archivo portable .atbackup.age"
}
$filesize = (Get-Item -LiteralPath $archive -ErrorAction Stop).Length
if ($filesize -lt 100 -or $filesize -gt 600000000) {
    throw "Archivo portable vacío o fuera del límite permitido"
}
$primaryRoot = [System.IO.Path]::GetPathRoot($primary)
$archiveRoot = [System.IO.Path]::GetPathRoot($archive)
if ($primaryRoot -notmatch '^[A-Z]:\\$' -or $archiveRoot -notmatch '^[A-Z]:\\$') {
    throw "No se pudo verificar el disco físico en una ruta local con letra de unidad"
}
if ($primaryRoot -eq $archiveRoot) {
    throw "El respaldo está en la misma unidad que AutoTrace"
}
$primaryLetter = $primaryRoot.Substring(0, 1)
$archiveLetter = $archiveRoot.Substring(0, 1)
$primaryDisk = Get-Partition -DriveLetter $primaryLetter -ErrorAction Stop | Get-Disk -ErrorAction Stop
$archiveDisk = Get-Partition -DriveLetter $archiveLetter -ErrorAction Stop | Get-Disk -ErrorAction Stop
if ($primaryDisk.Number -eq $archiveDisk.Number) {
    throw "Ambas unidades pertenecen al mismo disco físico; G2 no es aprobable"
}
if ($archiveDisk.BusType.ToString() -ne 'USB') {
    throw "El disco externo no está identificado como USB. Se requiere verificación manual independiente."
}
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
$os = [System.Environment]::OSVersion.Version.ToString()
$evidence = [ordered]@{
    evidenceType = 'autotrace-usb-media-preflight-v1'
    gate = 'G2'
    state = 'MEDIA_PREFLIGHT_ONLY'
    physicalRecoveryPassed = $false
    driveOAuthPassed = $false
    collectedAtUtc = [DateTime]::UtcNow.ToString('o')
    windowsVersion = $os
    sourceDiskNumber = $primaryDisk.Number
    targetDiskNumber = $archiveDisk.Number
    targetBusType = $archiveDisk.BusType.ToString()
    encryptedArchiveBytes = $filesize
    encryptedArchiveSha256 = $hash
    warning = 'Este control no verifica contenido, frase ni recuperación en otra PC.'
}
$target = [System.IO.Path]::GetFullPath($EvidencePath)
if (Test-Path -LiteralPath $target) {
    throw "Ya existe un informe: no se sobrescribe evidencia previa"
}
$json = $evidence | ConvertTo-Json -Depth 3
[System.IO.File]::WriteAllText($target, $json, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "USB PREFLIGHT PASS. Disco diferente y BusType USB; SHA-256: $hash"
Write-Host "Evidencia sin datos personales: $target"
Write-Warning "G3/G4/G5 y recuperación completa siguen PENDIENTES hasta ensayo real."
