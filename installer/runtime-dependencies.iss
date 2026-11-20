{ Shared by normal and maintenance Setup before any installed-file/service changes. }
function RuntimeDependencyTicks(): Int64;
  external 'GetTickCount64@kernel32.dll stdcall';

function CheckRuntimeDependencies(Correlation: String): String;
var
  ProbePath, DependencyPrompt, DownloadUrl: String;
  ResultCode, BrowserCode: Integer;
  Started: Boolean;
  StartTicks: Int64;
begin
  Result := '';
  StartTicks := RuntimeDependencyTicks();
  DownloadUrl := 'https://dotnet.microsoft.com/zh-cn/download/dotnet/8.0';
  Log('installer.dependency.started correlation=' + Correlation + ' dependency=dotnet_desktop_8_x64');
  ResultCode := 0;
  try
    { Run the bundled host, not an unrelated dotnet.exe or a registry-only presence check.
      The runtime resolves both required frameworks before the side-effect-free entry. }
    ExtractTemporaryFile('NxTools.PlatformHost.exe');
    ProbePath := ExpandConstant('{tmp}\NxTools.PlatformHost.exe');
    Started := Exec(ProbePath, '--check-runtime', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  except
    Log('installer.dependency.failed correlation=' + Correlation +
      ' dependency=dotnet_desktop_8_x64 stage=extraction elapsed_ms=' + IntToStr(RuntimeDependencyTicks() - StartTicks));
    Result := '无法解压安装包内的运行环境检查组件，尚未更改应用文件。请重新下载安装包后重试。';
    Exit;
  end;
  if Started and (ResultCode = 0) then
  begin
    Log('installer.dependency.completed correlation=' + Correlation +
      ' dependency=dotnet_desktop_8_x64 elapsed_ms=' + IntToStr(RuntimeDependencyTicks() - StartTicks));
    Exit;
  end;
  Log('installer.dependency.failed correlation=' + Correlation +
    ' dependency=dotnet_desktop_8_x64 stage=launch started=' + IntToStr(Ord(Started)) +
    ' code=' + IntToStr(ResultCode) + ' elapsed_ms=' + IntToStr(RuntimeDependencyTicks() - StartTicks));
  if Started and ((ResultCode = -2147450730) or (ResultCode = -2147450749)) then
    Result := '缺少或无法加载必需的运行环境：Microsoft .NET 8 桌面运行时（Windows x64）。'
  else
    Result := '无法启动运行环境检查组件。请检查安装包是否完整，以及 Microsoft .NET 8 桌面运行时（Windows x64）是否正确安装。';
  Result := Result + Chr(13) + Chr(10) +
    '请从微软官网下载 .NET Desktop Runtime 8.0，选择 Windows x64。安装后重新运行本安装程序。' + Chr(13) + Chr(10) +
    '仅安装 x86、ASP.NET Core Runtime 或其他 .NET 大版本不能替代此依赖。' + Chr(13) + Chr(10) +
    '官方下载：' + DownloadUrl + Chr(13) + Chr(10) +
    '尚未更改应用文件。错误码：' + IntToStr(ResultCode);
  if not WizardSilent then
  begin
    DependencyPrompt := Result + Chr(13) + Chr(10) + Chr(13) + Chr(10) + '是否打开微软官方下载页面？';
    if SuppressibleMsgBox(DependencyPrompt, mbError, MB_YESNO, IDNO) = IDYES then
    begin
      if not ShellExec('open', DownloadUrl, '', '', SW_SHOWNORMAL, ewNoWait, BrowserCode) then
      begin
        Log('installer.dependency.download_failed correlation=' + Correlation + ' code=' + IntToStr(BrowserCode));
        Result := Result + Chr(13) + Chr(10) + '无法打开浏览器，请复制上面的地址手动访问。';
      end;
    end;
  end;
end;
