; NxTools.Next Windows installer (Inno Setup 6)
; Installs the desktop suite into a user-selected directory and registers
; the uninstall entry. The indexer service and protected Service\ binaries
; deploy themselves on first run via the in-app installer (IndexerServiceInstaller).

#define MyAppName "NxTools"
#define MyAppVersion "0.7.0"
#define MyAppPublisher "NxTools"
#define MyAppExeName "NxTools.exe"
#define PackageDir "..\artifacts\win-x64"

[Setup]
AppId={{8C1E4E9A-52B7-4A0E-9F5D-2A1C3B7D9E10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\artifacts\installer
OutputBaseFilename=NxTools.Next-setup-{#MyAppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#MyAppExeName}
ChangesEnvironment=no


[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; \
    GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "autostart"; Description: "开机自动启动 NxTools"; \
    GroupDescription: "其他选项:"; Flags: checkedonce

[Files]
Source: "{#PackageDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.PlatformHost.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.Indexer.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.Cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.BrowserHost.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\THIRD_PARTY_NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\browser_extension\*"; DestDir: "{app}\browser_extension"; \
    Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\卸载 {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; \
    Tasks: desktopicon

[Registry]
; Optional autostart mirrors the in-app toggle (HKCU Run, --silent-start).
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
    ValueType: string; ValueName: "NxTools.Next"; \
    ValueData: """{app}\{#MyAppExeName}"" --silent-start"; \
    Tasks: autostart; Flags: uninsdeletevalue
; Remove the pre-NxTools.Next startup value so one install cannot launch twice.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
    ValueType: none; ValueName: "NxTools"; Flags: deletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; \
    Flags: nowait postinstall skipifsilent

[Code]
function SameOwnedPath(const Actual, Expected: String): Boolean;
begin
  Result := CompareText(ExpandFileName(Actual), ExpandFileName(Expected)) = 0;
end;

function OwnedIndexerService(): Boolean;
var
  ImagePath, Executable, Arguments, ServiceKey, ProtectedDir, DesktopPath: String;
  IndexerHash, DesktopHash, PlatformHash: String;
  ClosingQuote: Integer;
begin
  Result := False;
  ServiceKey := 'SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1';
  if not RegQueryStringValue(HKLM64, ServiceKey, 'ImagePath', ImagePath) then Exit;
  ImagePath := Trim(ImagePath);
  { Require an unambiguous, quoted executable plus the precise service argument. }
  if (Length(ImagePath) < 3) or (ImagePath[1] <> '"') then Exit;
  ClosingQuote := Pos('"', Copy(ImagePath, 2, Length(ImagePath))) + 1;
  if ClosingQuote <= 1 then Exit;
  Executable := Copy(ImagePath, 2, ClosingQuote - 2);
  Arguments := Trim(Copy(ImagePath, ClosingQuote + 1, Length(ImagePath)));
  if CompareText(Arguments, '--service') <> 0 then Exit;
  if SameOwnedPath(Executable, ExpandConstant('{app}\NxTools.Indexer.exe')) then
  begin
    Result := True;
    Exit;
  end;
  ProtectedDir := ExpandConstant('{commonappdata}\NxTools\NxTools.Next\Service');
  if not SameOwnedPath(Executable, ProtectedDir + '\NxTools.Indexer.exe') then Exit;
  if not RegQueryStringValue(HKLM64, ServiceKey, 'NxToolsDesktopPath', DesktopPath) then Exit;
  if not SameOwnedPath(DesktopPath, ProtectedDir + '\NxTools.exe') then Exit;
  if not RegQueryStringValue(HKLM64, ServiceKey, 'NxToolsBinarySha256', IndexerHash) then Exit;
  if not RegQueryStringValue(HKLM64, ServiceKey, 'NxToolsDesktopSha256', DesktopHash) then Exit;
  if not RegQueryStringValue(HKLM64, ServiceKey, 'NxToolsPlatformHostSha256', PlatformHash) then Exit;
  { The protected tuple belongs to this installation only when all three actual
    protected files AND all three installed counterparts match registered identity. }
  try
    Result :=
      (CompareText(GetSHA256OfFile(Executable), IndexerHash) = 0) and
      (CompareText(GetSHA256OfFile(DesktopPath), DesktopHash) = 0) and
      (CompareText(GetSHA256OfFile(ProtectedDir + '\NxTools.PlatformHost.exe'), PlatformHash) = 0) and
      (CompareText(GetSHA256OfFile(ExpandConstant('{app}\NxTools.Indexer.exe')), IndexerHash) = 0) and
      (CompareText(GetSHA256OfFile(ExpandConstant('{app}\NxTools.exe')), DesktopHash) = 0) and
      (CompareText(GetSHA256OfFile(ExpandConstant('{app}\NxTools.PlatformHost.exe')), PlatformHash) = 0);
  except
    Result := False;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ResultCode: Integer;
begin
  if (CurUninstallStep = usUninstall) and OwnedIndexerService() then
    Exec(ExpandConstant('{sys}\sc.exe'), 'stop NxToolsIndexerV1', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;
