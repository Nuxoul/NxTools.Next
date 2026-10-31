; NxTools.Next Windows installer (Inno Setup 6)
; Installs the desktop suite into a user-selected directory and registers
; the uninstall entry. The indexer service and protected Service\ binaries
; deploy themselves on first run via the in-app installer (IndexerServiceInstaller).

#define MyAppName "NxTools"
#define MyAppVersion "0.7.0"
#define MyAppPublisher "NxTools"
#define MyAppExeName "NxTools.exe"
#define PackageDir "..\artifacts\win-x64"
#include "..\artifacts\installer\bootstrap-migration-input.iss"

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
SetupIconFile=..\rust\apps\nxtools-desktop\assets\brand\nxtools.ico
SetupLogging=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#MyAppExeName}
ChangesEnvironment=no
; Restart Manager coordinates only the exact installed files, not name-wide kills.
CloseApplications=yes
RestartApplications=no
CloseApplicationsFilter=NxTools.exe,NxTools.PlatformHost.exe,NxTools.Indexer.exe,NxTools.Cli.exe,NxTools.BrowserHost.exe,NxTools.ArchiveWorker.exe,NxTools.ShellSelectionHelper.exe


[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; \
    GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "autostart"; Description: "开机自动启动 NxTools"; \
    GroupDescription: "其他选项:"; Flags: checkedonce

[Files]
; Upgrade cleanup runs before file replacement, including the approved protected
; desktop outside {app}. Embedded only; never deployed as a user workaround.
Source: "close-owned-processes.ps1"; Flags: dontcopy
Source: "bootstrap-migration.ps1"; Flags: dontcopy
Source: "{#PackageDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.PlatformHost.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.Indexer.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.Cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.BrowserHost.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.ArchiveWorker.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.ShellSelectionHelper.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.ShellSelectionHelper.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.ShellSelectionHelper.deps.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.ShellSelectionHelper.runtimeconfig.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\THIRD_PARTY_NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\browser_extension\*"; DestDir: "{app}\browser_extension"; \
    Flags: ignoreversion recursesubdirs createallsubdirs
; Extract these exact NEW sidecars before any owned-file mutation. The files[]
; SHA256 table is rollback input, not a replacement for trusted verification.
Source: "{#PackageDir}\NxTools.release.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\NxTools.release.sig"; DestDir: "{app}"; Flags: ignoreversion

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
  UpgradePreparationReady: Boolean;
  UpgradeStatePath, UpgradeCorrelation: String;
  UpgradeNewManifestPath, UpgradeNewSignaturePath: String;
  BootstrapMigrationDirectory: String;

function PreservePreparationDiagnostics(SourcePath, Correlation: String; Started: Boolean; ResultCode: Integer): String;
var
  DirectoryPath, Summary: String;
begin
  Result := ExtractFileDir(SourcePath);
  try
    DirectoryPath := ExpandConstant('{localappdata}\NxTools\NxTools.Next\logs\installer\') + Correlation;
    if not ForceDirectories(DirectoryPath) then Exit;
    Summary := 'correlation=' + Correlation + Chr(13) + Chr(10) +
      'started=' + IntToStr(Ord(Started)) + Chr(13) + Chr(10) + 'exit=' + IntToStr(ResultCode);
    SaveStringToFile(DirectoryPath + '\launch.txt', Summary, False);
    if FileExists(SourcePath) then
      FileCopy(SourcePath, DirectoryPath + '\preparation.jsonl', False);
    FileCopy(ExpandConstant('{log}'), DirectoryPath + '\setup.log', False);
    Result := DirectoryPath;
    Log('installer.upgrade_diagnostics.saved directory=' + Result);
  except
    Log('installer.upgrade_diagnostics.unavailable');
  end;
end;

function EnsureBootstrapMigration(): String;
var
  DetectionPath, Parameters, PowerShellPath, MigrationHost: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  Result := '';
  BootstrapMigrationDirectory := '';
  ExtractTemporaryFile('bootstrap-migration.ps1');
  DetectionPath := ExpandConstant('{tmp}\bootstrap-migration.ps1');
  PowerShellPath := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + DetectionPath +
    '" -ManifestPath "' + UpgradeNewManifestPath + '"';
  Log('installer.bootstrap_migration.detecting');
  Started := Exec(PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if Started and (ResultCode = 0) then Exit;
  if (not Started) or (ResultCode <> 10) then
  begin
    Result := '无法验证现有更新服务，尚未替换应用文件。请保留安装日志供检查。';
    Exit;
  end;
  if MsgBox('此旧版本需要升级更新组件才能继续安装。安装器将请求一次管理员授权，完成后自动继续；不会更换发布公钥或清除数据。', mbConfirmation, MB_OKCANCEL) <> IDOK then
  begin
    Result := '你已取消更新组件升级，尚未替换应用文件。';
    Exit;
  end;
  { Extract the ORIGINAL normal-install entries, preserving their relative directories.
    Inno stores this payload once; no second complete ZIP is embedded. }
  ExtractTemporaryFiles('{app}\*');
  BootstrapMigrationDirectory := ExpandConstant('{tmp}\') + '{app}';
  MigrationHost := BootstrapMigrationDirectory + '\NxTools.PlatformHost.exe';
  if CompareText(GetSHA256OfFile(MigrationHost), '{#BootstrapMigrationHostSha256}') <> 0 then
  begin
    Result := '安装包内的更新组件校验失败，尚未替换应用文件。';
    Exit;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  PowerShellPath, CleanupPath, Parameters, Correlation, FailureDetail: String;
  DiagnosticPath, DiagnosticDirectory: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  Result := '';
  NeedsRestart := False;
  UpgradePreparationReady := False;
  ExtractTemporaryFile('close-owned-processes.ps1');
  ExtractTemporaryFile('NxTools.release.json');
  ExtractTemporaryFile('NxTools.release.sig');
  UpgradeNewManifestPath := ExpandConstant('{tmp}\NxTools.release.json');
  UpgradeNewSignaturePath := ExpandConstant('{tmp}\NxTools.release.sig');
  Result := EnsureBootstrapMigration();
  if Result <> '' then Exit;
  PowerShellPath := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  CleanupPath := ExpandConstant('{tmp}\close-owned-processes.ps1');
  Correlation := GetDateTimeString('yyyymmddhhnnss', '-', ':') + '-' +
    ExtractFileName(ExpandConstant('{tmp}'));
  StringChangeEx(Correlation, '.', '-', True);
  UpgradeCorrelation := Correlation;
  UpgradeStatePath := ExpandConstant('{app}\.NxTools.BrowserHost-upgrade.json');
  DiagnosticPath := ExpandConstant('{tmp}\preparation-') + Correlation + '.jsonl';
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' +
    CleanupPath + '" -InstallDirectory "' + ExpandConstant('{app}') +
    '" -CorrelationId "' + Correlation + '" -StatePath "' + UpgradeStatePath +
    '" -DiagnosticPath "' + DiagnosticPath + '"';
  { The helper's default preparation entry validates/recovers any OLD journal
    before fresh suspension. New sidecars never authorize old recovery deletion. }
  Parameters := Parameters + ' -NewManifestPath "' + UpgradeNewManifestPath +
    '" -NewSignaturePath "' + UpgradeNewSignaturePath + '"';
  if BootstrapMigrationDirectory <> '' then
    Parameters := Parameters + ' -BootstrapMigrationPackage "' + BootstrapMigrationDirectory + '"';
  Log('installer.upgrade_close.started correlation=' + Correlation);
  WizardForm.StatusLabel.Caption := '正在恢复上次安装并准备升级，请稍候…';
  if (BootstrapMigrationDirectory <> '') or
    RegKeyExists(HKLM64, 'SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1') then
  begin
    if BootstrapMigrationDirectory <> '' then
    begin
      WizardForm.StatusLabel.Caption := '正在升级更新组件并准备安装，请在 Windows 提示时允许…';
      Log('installer.bootstrap_migration.elevation_requested correlation=' + Correlation);
    end else
    begin
      WizardForm.StatusLabel.Caption := '正在准备已安装的受保护组件，请在 Windows 提示时允许…';
      Log('installer.upgrade_close.protected_elevation_requested correlation=' + Correlation);
    end;
    { Service presence requests privileges ONLY. The original helper still verifies exact
      paths/hashes and recovery authority before any owned mutation; no validation bypass. }
    Started := ShellExec('runas', PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end else
  begin
    Started := Exec(PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    if Started and (ResultCode = 5) then
    begin
      Log('installer.upgrade_close.elevation_required correlation=' + Correlation);
      Started := ShellExec('runas', PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    end;
  end;
  DiagnosticDirectory := PreservePreparationDiagnostics(DiagnosticPath, Correlation, Started, ResultCode);
  if (not Started) or (ResultCode <> 0) then
  begin
    FailureDetail := IntToStr(ResultCode);
    Log('installer.upgrade_close.failed correlation=' + Correlation + ' code=' + FailureDetail);
    Result := '安装程序未能验证或恢复上次安装，或无法完成本次升级准备；尚未继续替换文件，恢复日志已保留。' +
      Chr(13) + Chr(10) + '请保留以下诊断目录供检查，不要手工删除恢复文件或反复重试。' +
      Chr(13) + Chr(10) + DiagnosticDirectory +
      Chr(13) + Chr(10) + '错误码：' + FailureDetail;
    Exit;
  end;
  Log('installer.upgrade_close.completed correlation=' + Correlation);
  UpgradePreparationReady := True;
end;

procedure FinalizeUpgradeFiles(Committed: Boolean);
var
  Parameters: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  if UpgradeStatePath = '' then Exit;
  if (not FileExists(UpgradeStatePath)) and (not FileExists(UpgradeStatePath + '.pending')) then Exit;
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' +
    ExpandConstant('{tmp}\close-owned-processes.ps1') + '" -InstallDirectory "' +
    ExpandConstant('{app}') + '" -StatePath "' + UpgradeStatePath +
    '" -CorrelationId "' + UpgradeCorrelation + '" -Finalize';
  if Committed then Parameters := Parameters + ' -Committed';
  { All new hashes were captured from extracted package sidecars before suspension.
    Never derive rollback deletion authority from a post-failure installed file. }
  Started := Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if Started and (ResultCode = 5) then
    Started := ShellExec('runas', ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if (not Started) or (ResultCode <> 0) then
  begin
    UpgradeFinalizationFailed := True;
    Log('installer.upgrade_files.failed correlation=' + UpgradeCorrelation + ' code=' + IntToStr(ResultCode));
    RaiseException('无法完成安装文件集合的恢复或清理，已保留恢复日志，错误码：' + IntToStr(ResultCode));
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
    PrepareTrustedUpdate();
    FinalizeUpgradeFiles(True);
    UpgradeFilesCommitted := True;
  end;
end;

procedure DeinitializeSetup();
begin
  if UpgradePreparationReady and (not UpgradeFilesCommitted) and (not UpgradeFinalizationFailed) then FinalizeUpgradeFiles(False);
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
