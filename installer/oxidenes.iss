; ============================================================================
; OxideNES — Inno Setup Installer Script
; ============================================================================
; Build:
;   1. cargo build --release
;   2. iscc installer\oxidenes.iss
;   Output: installer\Output\oxidenes-0.1.0-setup.exe
; ============================================================================

#define MyAppName      "OxideNES"
#define MyAppVersion   "0.1.0"
#define MyAppPublisher "OxideNES contributors"
#define MyAppURL       "https://github.com/deaddeadbeef/OxideNES"
#define MyAppExeName   "oxidenes.exe"
#define MyAppRoot      ".."

[Setup]
AppId={{B7E4F2A1-8C3D-4F5E-9A1B-2D7C6E8F0A3B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile={#MyAppRoot}\LICENSE
OutputDir=Output
OutputBaseFilename=oxidenes-{#MyAppVersion}-setup
Compression=lzma2/ultra
SolidCompression=yes
WizardStyle=modern
MinVersion=10.0
PrivilegesRequiredOverridesAllowed=dialog
PrivilegesRequired=lowest
ChangesAssociations=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=force
RestartApplications=no
; Uncomment if you have installer/icon.ico:
; SetupIconFile=icon.ico
; UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "associatenes"; Description: "Associate .nes files with {#MyAppName}"; GroupDescription: "File associations:"; Flags: checkedonce

[Files]
Source: "{#MyAppRoot}\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#MyAppRoot}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#MyAppRoot}\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.nes"; ValueType: string; ValueName: ""; ValueData: "NESEmulator.ROM"; Flags: uninsdeletevalue; Tasks: associatenes
Root: HKA; Subkey: "Software\Classes\NESEmulator.ROM"; ValueType: string; ValueName: ""; ValueData: "NES ROM File"; Flags: uninsdeletekey; Tasks: associatenes
Root: HKA; Subkey: "Software\Classes\NESEmulator.ROM\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: associatenes

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
