; Explicit updater bootstrap maintenance. Never entered by the normal installer.
#define MyAppVersion "0.7.4"
#define PackageDir "..\artifacts\win-x64"
[Setup]
AppName=NxTools 更新服务维护
AppVersion={#MyAppVersion}
VersionInfoVersion={#MyAppVersion}.0
VersionInfoProductVersion={#MyAppVersion}
AppPublisher=NxTools
DefaultDirName={tmp}\NxTools-maintenance
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\artifacts\installer
OutputBaseFilename=NxTools.Next-maintenance-{#MyAppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=no
DisableWelcomePage=no
CloseApplications=no
RestartApplications=no
[Files]
Source: "{#PackageDir}\*"; DestDir: "{tmp}\NxTools-maintenance-package"; Flags: recursesubdirs createallsubdirs ignoreversion deleteafterinstall
[Code]
var
  MaintenanceSucceeded: Boolean;
#include "runtime-dependencies.iss"

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  NeedsRestart := False;
  Result := CheckRuntimeDependencies(GetDateTimeString('yyyymmddhhnnss', '-', ':') + '-' +
    ExtractFileName(ExpandConstant('{tmp}')));
end;

procedure InitializeWizard();
begin
  WizardForm.WelcomeLabel2.Caption := '此工具只维护现有 NxTools 可信更新服务，以支持新增发布文件。' + Chr(13) + Chr(10) +
    '将保留原发布公钥、版本序号和已安装业务版本；失败会恢复旧更新服务。' + Chr(13) + Chr(10) +
    '需要管理员授权，不会注册压缩右键菜单或启动桌面。';
end;
procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
  Started: Boolean;
begin
  if CurStep = ssPostInstall then
  begin
    WizardForm.StatusLabel.Caption := '正在验证并维护可信更新服务…';
    Started := Exec(ExpandConstant('{tmp}\NxTools-maintenance-package\NxTools.PlatformHost.exe'),
      '--maintain-trusted-bootstrap', '', SW_HIDE, ewWaitUntilTerminated, Code);
    if (not Started) or (Code <> 0) then
      RaiseException('更新服务维护未完成。旧版本或恢复记录已保留，请不要手工替换 ProgramData 文件。错误码：' + IntToStr(Code));
    MaintenanceSucceeded := True;
    WizardForm.FinishedLabel.Caption := '更新服务维护完成。现在请运行配套的 NxTools.Next-setup 安装包升级应用。此工具未升级业务版本。';
  end;
end;
function GetCustomSetupExitCode(): Integer;
begin
  if MaintenanceSucceeded then Result := 0 else Result := 1;
end;
