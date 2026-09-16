#ifndef AppVersion
  #error Pass the version with /DAppVersion=x.y.z
#endif
#ifndef SourceExe
  #error Pass the built exe with /DSourceExe=path
#endif
#ifndef AppIcon
  #error Pass the icon with /DAppIcon=path
#endif

#define AppName "Simple Mic Lock"
#define AppExeName "simple-mic-lock.exe"

[Setup]
AppId={{6F1C2A9E-3B7D-4E58-9A14-C2D8E5B7F301}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=SalahaldinBilal
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename=SimpleMicLock-Setup-{#AppVersion}
SetupIconFile={#AppIcon}
UninstallDisplayIcon={app}\{#AppExeName}
UninstallDisplayName={#AppName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"

[Registry]
; Removes the Start with Windows entry the app itself creates.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "SimpleMicLock"; ValueType: none; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent runasoriginaluser

[Code]
procedure StopRunningApp;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopRunningApp;
  Result := '';
end;

function InitializeUninstall(): Boolean;
begin
  StopRunningApp;
  Result := True;
end;
