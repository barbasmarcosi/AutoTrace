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

$installBase = Join-Path $env:LOCALAPPDATA "Programs"
if (-not (Test-Path $installBase)) { throw "No se creó carpeta de programas para el usuario" }
$installed = @(Get-ChildItem $installBase -Recurse -Filter "autotrace-desktop.exe" -File -ErrorAction Stop)
if ($installed.Count -ne 1) {
    throw "Se esperó un único AutoTrace instalado, encontrados: $($installed.Count)"
}
$exe = $installed[0]
$shaOriginal=(Get-FileHash $builtExe -Algorithm SHA256).Hash
$shaInstalled=(Get-FileHash $exe.FullName -Algorithm SHA256).Hash
if ($shaOriginal -ne $shaInstalled) { throw "El ejecutable instalado es diferente al compilado" }
Write-Host "Instalado correctamente: $($exe.FullName)"
Write-Host "SHA256: $shaInstalled"

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
Write-Host "SMOKE PASS: NSIS instala y desinstala en el runner Windows."
