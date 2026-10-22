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
; Restart Manager coordinates only the exact installed files, not name-wide kills.
CloseApplications=yes
RestartApplications=no
CloseApplicationsFilter=NxTools.exe,NxTools.PlatformHost.exe,NxTools.Indexer.exe,NxTools.Cli.exe,NxTools.BrowserHost.exe


[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; \
    GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "autostart"; Description: "开机自动启动 NxTools"; \
    GroupDescription: "其他选项:"; Flags: checkedonce

[Files]
; Upgrade cleanup runs before file replacement, including the approved protected
; desktop outside {app}. Embedded only; never deployed as a user workaround.
Source: "close-owned-processes.ps1"; Flags: dontcopy
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
Source: "{#PackageDir}\NxTools.release.json"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#PackageDir}\NxTools.release.sig"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

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
    Flags: nowait postinstall skipifsilent runasoriginaluser; Check: UpgradeCanLaunch

[Code]
var
  UpgradeFilesCommitted, UpgradeFinalizationFailed: Boolean;
  UpgradeStatePath, UpgradeCorrelation: String;
  UpgradeNewBrowserHostHash: String;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  PowerShellPath, CleanupPath, Parameters, Correlation, FailureDetail: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  Result := '';
  NeedsRestart := False;
  ExtractTemporaryFile('close-owned-processes.ps1');
  PowerShellPath := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  CleanupPath := ExpandConstant('{tmp}\close-owned-processes.ps1');
  Correlation := GetDateTimeString('yyyymmddhhnnss', '-', ':') + '-' +
    ExtractFileName(ExpandConstant('{tmp}'));
  UpgradeCorrelation := Correlation;
  UpgradeStatePath := ExpandConstant('{app}\.NxTools.BrowserHost-upgrade.json');
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' +
    CleanupPath + '" -InstallDirectory "' + ExpandConstant('{app}') +
    '" -CorrelationId "' + Correlation + '" -StatePath "' + UpgradeStatePath + '"';
  Log('installer.upgrade_close.started correlation=' + Correlation);
  WizardForm.StatusLabel.Caption := '正在关闭旧版 NxTools，请稍候…';
  Started := Exec(PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if Started and (ResultCode = 5) then
  begin
    Log('installer.upgrade_close.elevation_required correlation=' + Correlation);
    Started := ShellExec('runas', PowerShellPath, Parameters, '', SW_HIDE,
      ewWaitUntilTerminated, ResultCode);
  end;
  if (not Started) or (ResultCode <> 0) then
  begin
    FailureDetail := IntToStr(ResultCode);
    Log('installer.upgrade_close.failed correlation=' + Correlation + ' code=' + FailureDetail);
    Result := '安装程序无法关闭属于本次升级的旧版 NxTools，尚未替换程序文件。' +
      Chr(13) + Chr(10) + '请重试安装，并在 Windows 请求权限时允许安装程序关闭旧进程。' +
      Chr(13) + Chr(10) + '错误码：' + FailureDetail;
    Exit;
  end;
  Log('installer.upgrade_close.completed correlation=' + Correlation);
end;

procedure FinalizeUpgradeFiles(Committed: Boolean);
var
  Parameters: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  if (UpgradeStatePath = '') or not FileExists(UpgradeStatePath) then Exit;
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' +
    ExpandConstant('{tmp}\close-owned-processes.ps1') + '" -InstallDirectory "' +
    ExpandConstant('{app}') + '" -StatePath "' + UpgradeStatePath +
    '" -CorrelationId "' + UpgradeCorrelation + '" -Finalize';
  if Committed then Parameters := Parameters + ' -Committed';
  if (not Committed) and (UpgradeNewBrowserHostHash <> '') then
    Parameters := Parameters + ' -RollbackNewSha256 "' + UpgradeNewBrowserHostHash + '"';
  Started := Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if Started and (ResultCode = 5) then
    Started := ShellExec('runas', ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if (not Started) or (ResultCode <> 0) then
  begin
    UpgradeFinalizationFailed := True;
    Log('installer.upgrade_files.failed correlation=' + UpgradeCorrelation + ' code=' + IntToStr(ResultCode));
    RaiseException('无法完成浏览器宿主的安装恢复或清理，错误码：' + IntToStr(ResultCode));
  end;
  Log('installer.upgrade_files.finalized correlation=' + UpgradeCorrelation);
end;

procedure PrepareTrustedUpdate();
var
  ResultCode: Integer;
begin
  if not FileExists(ExpandConstant('{app}\NxTools.release.json')) then Exit;
  Log('installer.trusted_update.started correlation=' + UpgradeCorrelation);
  if (not Exec(ExpandConstant('{app}\NxTools.PlatformHost.exe'), '--prepare-trusted-update',
      '', SW_HIDE, ewWaitUntilTerminated, ResultCode)) or (ResultCode <> 0) then
  begin
    FinalizeUpgradeFiles(False);
    UpgradeFinalizationFailed := True;
    Log('installer.trusted_update.failed correlation=' + UpgradeCorrelation + ' code=' + IntToStr(ResultCode));
    RaiseException('可信更新未完成，未启动新版本。请检查安装包签名或使用维护安装程序修复。');
  end;
  Log('installer.trusted_update.completed correlation=' + UpgradeCorrelation);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if FileExists(ExpandConstant('{app}\NxTools.BrowserHost.exe')) then
      UpgradeNewBrowserHostHash := GetSHA256OfFile(ExpandConstant('{app}\NxTools.BrowserHost.exe'));
    PrepareTrustedUpdate();
    FinalizeUpgradeFiles(True);
    UpgradeFilesCommitted := True;
  end;
end;

procedure DeinitializeSetup();
begin
  if (not UpgradeFilesCommitted) and (not UpgradeFinalizationFailed) then FinalizeUpgradeFiles(False);
end;

function UpgradeCanLaunch(): Boolean;
begin
  Result := UpgradeFilesCommitted and (not UpgradeFinalizationFailed);
end;

function GetCustomSetupExitCode(): Integer;
begin
  if UpgradeFinalizationFailed then Result := 9 else Result := 0;
end;

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
