; NxTools.Next Windows installer (Inno Setup 6)
; Installs the desktop suite into a user-selected directory and registers
; the uninstall entry. The indexer service and protected Service\ binaries
; deploy themselves on first run via the in-app installer (IndexerServiceInstaller).

#define MyAppName "NxTools"
#define MyAppVersion "0.7.5"
#define MyAppPublisher "NxTools"
#define MyAppExeName "NxTools.exe"
#define PackageDir "..\artifacts\win-x64"
#include "..\artifacts\installer\bootstrap-migration-input.iss"

[Setup]
AppId={{8C1E4E9A-52B7-4A0E-9F5D-2A1C3B7D9E10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
VersionInfoVersion={#MyAppVersion}.0
VersionInfoProductVersion={#MyAppVersion}
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
; Uninstall support is installed so the real uninstaller can run exact-image shutdown
; and current-user cleanup before Inno removes {app}.
Source: "close-owned-processes.ps1"; DestDir: "{app}\installer-support"; Flags: ignoreversion
Source: "uninstall-cleanup.ps1"; DestDir: "{app}\installer-support"; Flags: ignoreversion
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
  UninstallCorrelation: String;
  UpgradeNewManifestPath, UpgradeNewSignaturePath: String;
  BootstrapMigrationDirectory: String;
  BootstrapRepairMissing, BootstrapTrustRotation: Boolean;
  UninstallRemovePersonalData: Boolean;
  ArchiveIntegrationFresh: Boolean;
  ArchiveIntegrationExit: Integer;

#include "runtime-dependencies.iss"

function CoCreateGuid(var Value: TGUID): Longint;
  external 'CoCreateGuid@ole32.dll stdcall';
function StringFromGUID2(var Value: TGUID; Buffer: String; Count: Integer): Integer;
  external 'StringFromGUID2@ole32.dll stdcall';
function GetTickCount64(): Int64;
  external 'GetTickCount64@kernel32.dll stdcall';

function NewInstallerCorrelation(): String;
var
  Value: TGUID;
  Buffer: String;
begin
  SetLength(Buffer, 39);
  if (CoCreateGuid(Value) <> 0) or (StringFromGUID2(Value, Buffer, 39) <> 39) then
    RaiseException('无法生成 NxTools 操作标识。');
  Result := Copy(Buffer, 2, 36);
end;

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
  DetectionPath, Parameters, PowerShellPath, MigrationHost, DetectionDiagnostic, DiagnosticDirectory, Prompt: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  Result := '';
  BootstrapMigrationDirectory := '';
  BootstrapRepairMissing := False;
  BootstrapTrustRotation := False;
  DetectionDiagnostic := ExpandConstant('{tmp}\bootstrap-detection-') + UpgradeCorrelation + '.jsonl';
  ExtractTemporaryFile('bootstrap-migration.ps1');
  DetectionPath := ExpandConstant('{tmp}\bootstrap-migration.ps1');
  PowerShellPath := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + DetectionPath +
    '" -ManifestPath "' + UpgradeNewManifestPath + '" -DiagnosticPath "' + DetectionDiagnostic + '" -CorrelationId "' + UpgradeCorrelation + '"' +
    ' -MinimumBootstrapMaintenanceSequence "{#BootstrapMigrationMinimumSequence}"';
  Log('installer.bootstrap_migration.detecting');
  Started := Exec(PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Log('installer.bootstrap_migration.detected started=' + IntToStr(Ord(Started)) + ' exit=' + IntToStr(ResultCode) + ' correlation=' + UpgradeCorrelation);
  DiagnosticDirectory := PreservePreparationDiagnostics(DetectionDiagnostic, UpgradeCorrelation, Started, ResultCode);
  if FileExists(DetectionDiagnostic) then FileCopy(DetectionDiagnostic, DiagnosticDirectory + '\bootstrap-detection.jsonl', False);
  if Started and (ResultCode = 0) then Exit;
  if (not Started) or ((ResultCode <> 10) and (ResultCode <> 11) and (ResultCode <> 12)) then
  begin
    Result := '无法检查更新组件，尚未替换应用文件。诊断日志：' + DiagnosticDirectory;
    Exit;
  end;
  BootstrapRepairMissing := (ResultCode = 11);
  BootstrapTrustRotation := (ResultCode = 12);
  if BootstrapTrustRotation then
    Prompt := '检测到旧发布身份。安装器将以一次明确授权把更新组件迁移到当前发布身份，然后覆盖安装；不会删除用户数据。'
  else if BootstrapRepairMissing then
    Prompt := '已登记的 NxTools 更新组件文件缺失。安装器需要一次管理员授权来修复更新组件，然后继续安装；不会删除已有数据。'
  else
    Prompt := '此旧版本需要升级更新组件才能继续安装。安装器将请求一次管理员授权，完成后自动继续。';
  if MsgBox(Prompt, mbConfirmation, MB_OKCANCEL) <> IDOK then
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
  UpgradeCorrelation := GetDateTimeString('yyyymmddhhnnss', '-', ':') + '-' + ExtractFileName(ExpandConstant('{tmp}'));
  StringChangeEx(UpgradeCorrelation, '.', '-', True);
  Result := CheckRuntimeDependencies(UpgradeCorrelation);
  if Result <> '' then Exit;
  ExtractTemporaryFile('close-owned-processes.ps1');
  ExtractTemporaryFile('NxTools.release.json');
  ExtractTemporaryFile('NxTools.release.sig');
  UpgradeNewManifestPath := ExpandConstant('{tmp}\NxTools.release.json');
  UpgradeNewSignaturePath := ExpandConstant('{tmp}\NxTools.release.sig');
  Result := EnsureBootstrapMigration();
  if Result <> '' then Exit;
  PowerShellPath := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  CleanupPath := ExpandConstant('{tmp}\close-owned-processes.ps1');
  Correlation := UpgradeCorrelation;
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
  if BootstrapRepairMissing then
    Parameters := Parameters + ' -RepairMissingBootstrap';
  if BootstrapTrustRotation then
    Parameters := Parameters + ' -RotateTrustedBootstrap';
  Log('installer.upgrade_close.started correlation=' + Correlation);
  WizardForm.StatusLabel.Caption := '正在恢复上次安装并准备升级，请稍候…';
  if BootstrapMigrationDirectory <> '' then
  begin
    WizardForm.StatusLabel.Caption := '正在升级更新组件并准备安装，请在 Windows 提示时允许…';
    Log('installer.bootstrap_migration.elevation_requested correlation=' + Correlation);
    { Explicit bootstrap maintenance owns its authorization. Ordinary preparation
      first runs as the current user and requests elevation only on access denial. }
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
  ArchiveIntegrationFresh := not FileExists(ExpandConstant('{app}\{#MyAppExeName}')) and
    not RegKeyExists(HKCU, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{8C1E4E9A-52B7-4A0E-9F5D-2A1C3B7D9E10}_is1') and
    not RegKeyExists(HKLM32, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{8C1E4E9A-52B7-4A0E-9F5D-2A1C3B7D9E10}_is1') and
    not RegKeyExists(HKLM64, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{8C1E4E9A-52B7-4A0E-9F5D-2A1C3B7D9E10}_is1');
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

procedure ApplyInstalledArchiveIntegration();
var
  Correlation, Mode, MessageText: String;
  StartedAt: Int64;
  Started: Boolean;
  ResultCode: Integer;
begin
  StartedAt := GetTickCount64();
  Correlation := '';
  ArchiveIntegrationExit := 11;
  if ArchiveIntegrationFresh then Mode := 'fresh' else Mode := 'upgrade';
  try
    Correlation := NewInstallerCorrelation();
    Log('installer.archive_integration.started correlation=' + Correlation +
      ' setup_correlation=' + UpgradeCorrelation + ' mode=' + Mode);
    WizardForm.StatusLabel.Caption := '正在应用当前用户的压缩包集成…';
    Started := ExecAsOriginalUser(ExpandConstant('{app}\{#MyAppExeName}'),
      '--install-archive-integration ' + Correlation + ' ' + Mode,
      '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Log('installer.archive_integration.terminal correlation=' + Correlation +
      ' elapsed_ms=' + IntToStr(GetTickCount64() - StartedAt) +
      ' started=' + IntToStr(Ord(Started)) + ' code=' + IntToStr(ResultCode));
    if Started and ((ResultCode = 0) or (ResultCode = 10) or (ResultCode = 12)) then
      ArchiveIntegrationExit := ResultCode;
  except
    Log('installer.archive_integration.failed correlation=' + Correlation +
      ' elapsed_ms=' + IntToStr(GetTickCount64() - StartedAt));
  end;
  if ArchiveIntegrationExit = 0 then Exit;
  if ArchiveIntegrationExit = 10 then
    MessageText := 'NxTools 已安装，压缩包集成已应用，但 Windows 未确认部分默认关联。可在 Windows“默认应用”中检查。'
  else if ArchiveIntegrationExit = 12 then
    MessageText := 'NxTools 已安装，但无法确认当前进程属于原始安装用户。请在原始用户下打开 NxTools 并在压缩包集成中重试。'
  else
    MessageText := 'NxTools 已安装，但未确认压缩包集成成功。请在 NxTools 的压缩包集成中重试，或检查安装日志。';
  if not WizardSilent then
    SuppressibleMsgBox(MessageText, mbInformation, MB_OK, IDOK);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    PrepareTrustedUpdate();
    FinalizeUpgradeFiles(True);
    UpgradeFilesCommitted := True;
    ApplyInstalledArchiveIntegration();
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
  if UpgradeFinalizationFailed then Result := 9 else Result := ArchiveIntegrationExit;
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

function InitializeUninstall(): Boolean;
var
  Choice: Integer;
begin
  Result := True;
  UninstallRemovePersonalData := True;
  UninstallCorrelation := NewInstallerCorrelation();
  if UninstallSilent then Exit;
  Choice := MsgBox(
    '卸载将默认删除 NxTools 的设置、历史记录、缓存、日志和扩展运行数据，但保留已下载的最终文件。' +
    Chr(13) + Chr(10) + Chr(13) + Chr(10) +
    '选择“是”删除个人数据；选择“否”明确保留个人数据；选择“取消”退出卸载。',
    mbConfirmation, MB_YESNOCANCEL);
  if Choice = IDCANCEL then
    Result := False
  else
    UninstallRemovePersonalData := Choice = IDYES;
end;

procedure RunProtectedUninstallCleanup();
var
  HostPath, Parameters: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  if (not RegKeyExists(HKLM64, 'SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1')) and
    (not RegKeyExists(HKLM64, 'SYSTEM\CurrentControlSet\Services\NxToolsUpdateV1')) and
    (not DirExists(ExpandConstant('{commonappdata}\NxTools\NxTools.Next\Service'))) and
    (not DirExists(ExpandConstant('{commonappdata}\NxTools\NxTools.Next\Updater'))) then Exit;
  HostPath := ExpandConstant('{app}\NxTools.PlatformHost.exe');
  Parameters := '--uninstall-protected-components "' + ExpandConstant('{app}') +
    '" ' + UninstallCorrelation;
  Log('installer.protected_uninstall.authorization_requested correlation=' + UninstallCorrelation);
  Started := ShellExec('runas', HostPath, Parameters, '', SW_HIDE,
    ewWaitUntilTerminated, ResultCode);
  if (not Started) or (ResultCode <> 0) then
  begin
    Log('installer.protected_uninstall.failed correlation=' + UninstallCorrelation +
      ' code=' + IntToStr(ResultCode));
    RaiseException('无法验证或删除 NxTools 受保护服务与部署；恢复现场和未知组件已保留。错误码：' +
      IntToStr(ResultCode));
  end;
  Log('installer.protected_uninstall.completed correlation=' + UninstallCorrelation);
end;

procedure RunUninstallCleanup();
var
  PowerShellPath, CleanupPath, ClosePath, Parameters, Correlation: String;
  ResultCode: Integer;
  Started: Boolean;
begin
  PowerShellPath := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  CleanupPath := ExpandConstant('{app}\installer-support\uninstall-cleanup.ps1');
  ClosePath := ExpandConstant('{app}\installer-support\close-owned-processes.ps1');
  Correlation := UninstallCorrelation;
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + CleanupPath +
    '" -Operation Uninstall -InstallDirectory "' + ExpandConstant('{app}') +
    '" -ConfigRoot "' + ExpandConstant('{userappdata}\NxTools\NxTools.Next\config') +
    '" -DataRoot "' + ExpandConstant('{userappdata}\NxTools\NxTools.Next\data') +
    '" -CacheRoot "' + ExpandConstant('{localappdata}\NxTools\NxTools.Next\cache') +
    '" -LogRoot "' + ExpandConstant('{localappdata}\NxTools\NxTools.Next\data\logs') +
    '" -LocalProductRoot "' + ExpandConstant('{localappdata}\NxTools\NxTools.Next') +
    '" -CloseLibraryPath "' + ClosePath + '" -CorrelationId "' + Correlation + '"';
  if UninstallRemovePersonalData then Parameters := Parameters + ' -RemovePersonalData';
  Log('installer.uninstall_cleanup.started correlation=' + Correlation +
    ' remove_personal=' + IntToStr(Ord(UninstallRemovePersonalData)));
  Started := Exec(PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  if (not Started) or (ResultCode <> 0) then
  begin
    Log('installer.uninstall_cleanup.failed correlation=' + Correlation + ' code=' + IntToStr(ResultCode));
    if Started and (ResultCode = 12) then
      RaiseException('无法确认卸载进程属于原始用户。请在原始用户下卸载；压缩包集成恢复记录已保留。');
    if Started and (ResultCode = 11) then
      RaiseException('未能确认压缩包集成恢复完成。已保留原有选择和恢复记录，请在修复后重试卸载。');
    RaiseException('无法完成 NxTools 归属进程或卸载数据清理；未将失败视为成功。错误码：' + IntToStr(ResultCode));
  end;
  Log('installer.uninstall_cleanup.completed correlation=' + Correlation);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    RunProtectedUninstallCleanup();
    RunUninstallCleanup();
  end;
end;
