# Validates NSIS distribution without opening a GUI or using customer data.
# Run only on a disposable Windows CI runner.
$ErrorActionPreference = "Stop"
$root = (Resolve-Path ".").Path
$installer = Get-ChildItem -Path "desktop/src-tauri/target/release/bundle/nsis" -Filter "*-setup.exe" -File | Select-Object -First 1
if (-not $installer) { throw "No se encontró el instalador NSIS" }

$builtExe = Join-Path $root "desktop/src-tauri/target/release/autotrace-desktop.exe"
if (-not (Test-Path $builtExe)) { throw "No se encontró el ejecutable compilado" }
Write-Host "Instalando: $($installer.Name)"
$p = Start-Process -FilePath $installer.FullName -ArgumentList "/S" -PassThru
if (-not $p.WaitForExit(180000)) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    throw "El instalador silencioso excedió 180 segundos"
}
if ($p.ExitCode -ne 0) { throw "El instalador devolvió $($p.ExitCode)" }

# Tauri/NSIS can choose different per-user directories across versions.
# Prefer the actual Windows uninstall registry entry over guessed paths.
$keys = @(
    Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue
    Get-ItemProperty "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue
)
$entries = @($keys | Where-Object { $_.DisplayName -like "AutoTrace*" })
$folders = @(
    (Join-Path $env:LOCALAPPDATA "AutoTrace")
    (Join-Path $env:LOCALAPPDATA "Programs\AutoTrace")
    (Join-Path $env:APPDATA "AutoTrace")
)
foreach ($entry in $entries) {
    if ($entry.InstallLocation) { $folders += $entry.InstallLocation.Trim('"') }
}
$installed = @()
foreach ($folder in ($folders | Select-Object -Unique)) {
    if (-not (Test-Path $folder)) { continue }
    $installed += @(Get-ChildItem -Path $folder -File -Filter "autotrace-desktop.exe" -Recurse -ErrorAction SilentlyContinue)
}
# Final fallback for runners where NSIS uses an unexpected per-user folder.
if ($installed.Count -eq 0) {
    $installed = @(Get-ChildItem -Path $env:LOCALAPPDATA -Filter "autotrace-desktop.exe" -File -Recurse -ErrorAction SilentlyContinue)
}
$installed = @($installed | Sort-Object -Property FullName -Unique)
if ($installed.Count -ne 1) {
    Write-Host "AutoTrace uninstall registry entries:"
    $entries | Select-Object DisplayName, InstallLocation, UninstallString | Format-Table -AutoSize
    throw "Se esperó exactamente un ejecutable instalado, encontrados: $($installed.Count)"
}
$exe = $installed[0]
$shaOriginal=(Get-FileHash $builtExe -Algorithm SHA256).Hash
$shaInstalled=(Get-FileHash $exe.FullName -Algorithm SHA256).Hash
Write-Host "SHA256 del ejecutable de compilación: $shaOriginal"
Write-Host "SHA256 del ejecutable instalado:    $shaInstalled"
# Packaging can patch PE resources; identical bytes are not guaranteed.
# Validate PE structure and product version instead of assuming byte identity.
if ($exe.Length -lt 100000) { throw "El ejecutable instalado es demasiado pequeño" }
$stream=[System.IO.File]::OpenRead($exe.FullName)
try {
    $header=New-Object byte[] 2
    [void]$stream.Read($header,0,2)
    if ($header[0] -ne 0x4D -or $header[1] -ne 0x5A) { throw "El ejecutable instalado no es un binario PE válido" }
} finally { $stream.Dispose() }
$installedInfo=(Get-Item $exe.FullName).VersionInfo
$builtInfo=(Get-Item $builtExe).VersionInfo
Write-Host "Producto instalado: $($installedInfo.ProductName)"
Write-Host "Versión instalada: $($installedInfo.FileVersion)"
if ($installedInfo.ProductName -notmatch "AutoTrace") { throw "Identidad de producto instalada incorrecta" }
if ($builtInfo.FileVersion -and $installedInfo.FileVersion -and $builtInfo.FileVersion -ne $installedInfo.FileVersion) {
    throw "La versión instalada difiere de la compilación"
}
Write-Host "Instalado correctamente: $($exe.FullName)"
# Regression smoke: a re-install on the same machine must not wipe the
# dedicated application data directory. This is NOT an old-version migration test.
$dataDir = Join-Path $env:LOCALAPPDATA "ar.com.autotrace.desktop"
New-Item -Path $dataDir -ItemType Directory -Force | Out-Null
$sentinel = Join-Path $dataDir "nsis-acceptance-sentinel.txt"
if (Test-Path $sentinel) { throw "El marcador de prueba ya existe: runner no limpio" }
$sentinelContents = "NSIS-DO-NOT-DELETE-$(New-Guid)"
[System.IO.File]::WriteAllText($sentinel, $sentinelContents)
Write-Host "Reinstalando NSIS para verificar preservación de datos de usuario..."
$upgrade = Start-Process -FilePath $installer.FullName -ArgumentList "/S" -PassThru
if (-not $upgrade.WaitForExit(180000)) {
    Stop-Process -Id $upgrade.Id -Force -ErrorAction SilentlyContinue
    throw "La reinstalación NSIS excedió 180 segundos"
}
if ($upgrade.ExitCode -ne 0) { throw "La reinstalación devolvió $($upgrade.ExitCode)" }
if (-not (Test-Path $exe.FullName)) { throw "La reinstalación eliminó el ejecutable" }
if (-not (Test-Path $sentinel) -or [System.IO.File]::ReadAllText($sentinel) -ne $sentinelContents) {
    throw "La reinstalación NSIS alteró los datos persistentes del usuario"
}
Write-Host "SMOKE PASS: reinstalación conservó el marcador en AppData."


$uninstall = Join-Path $exe.Directory.FullName "uninstall.exe"
if (-not (Test-Path $uninstall)) { throw "No se encontró uninstall.exe" }
$p=Start-Process -FilePath $uninstall -ArgumentList "/S" -PassThru
if (-not $p.WaitForExit(180000)) {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    throw "El desinstalador silencioso excedió 180 segundos"
}
if ($p.ExitCode -ne 0) { throw "El desinstalador devolvió $($p.ExitCode)" }
for ($retry=0; $retry -lt 10 -and (Test-Path $exe.FullName); $retry++) {
    Start-Sleep -Milliseconds 500
}
if (Test-Path $exe.FullName) { throw "El ejecutable sigue instalado luego de desinstalar" }
if (-not (Test-Path $sentinel) -or [System.IO.File]::ReadAllText($sentinel) -ne $sentinelContents) {
    throw "El desinstalador eliminó datos persistentes del usuario"
}
Remove-Item -LiteralPath $sentinel -Force
Write-Host "SMOKE PASS: instalación, reinstalación y desinstalación preservan AppData sintético (runner)."

