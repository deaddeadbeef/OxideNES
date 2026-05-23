param(
    [string]$InstallerGlob = "target\wix\*.msi",
    [string]$InstallRoot = "$env:ProgramFiles\OxideNES"
)

$ErrorActionPreference = "Stop"

function Fail($Message) {
    Write-Error $Message
    exit 1
}

function Invoke-Msi($Arguments, $Action) {
    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList $Arguments -Wait -PassThru
    if ($process.ExitCode -ne 0 -and $process.ExitCode -ne 3010) {
        Fail "$Action failed with msiexec exit code $($process.ExitCode)"
    }
}

$installers = @(Get-ChildItem -Path $InstallerGlob)
if ($installers.Count -ne 1) {
    Fail "Expected exactly one MSI from '$InstallerGlob', found $($installers.Count)"
}

$installer = $installers[0].FullName
$installedExe = Join-Path $InstallRoot "bin\oxidenes.exe"

Write-Host "Installing $installer"
Invoke-Msi @("/i", $installer, "/qn", "/norestart") "Install"

if (-not (Test-Path $installedExe)) {
    Fail "Installed executable was not found at $installedExe"
}

Write-Host "Launching installed executable with --version"
& $installedExe --version
$versionExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
if ($versionExitCode -ne 0) {
    $exitCode = $versionExitCode
    Invoke-Msi @("/x", $installer, "/qn", "/norestart") "Uninstall"
    Fail "Installed executable --version failed with exit code $exitCode"
}

Write-Host "Uninstalling $installer"
Invoke-Msi @("/x", $installer, "/qn", "/norestart") "Uninstall"

if (Test-Path $installedExe) {
    Fail "Installed executable still exists after uninstall: $installedExe"
}

Write-Host "Windows installer smoke check passed"
