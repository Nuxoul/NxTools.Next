$ErrorActionPreference='Stop'
$root=(Get-Location).Path
$gate=Get-Content -Raw -LiteralPath 'tests\architecture\quick-launch-lifecycle.ps1'
# Execute only the original source-loading assignments, never the product scripts they read.
$readStart=$gate.IndexOf('$desktopMainPath =');$readEnd=$gate.IndexOf('if ($source -notmatch', $readStart)
Invoke-Expression $gate.Substring($readStart,$readEnd-$readStart)
$quickLaunchSource=(@(Get-Content -Raw 'rust\apps\nxtools-desktop\src\quick_launch.rs')+@(Get-ChildItem 'rust\apps\nxtools-desktop\src\quick_launch' -Filter '*.rs'|ForEach-Object{Get-Content -Raw $_.FullName})) -join "`n"
$start=$gate.IndexOf('if ($protocolSource -notmatch ''ElevatedDirectoryHelperRequested''')
$end=$gate.IndexOf("throw @'",$start)
$condition=$gate.Substring($start,$end-$start)
$results=@()
foreach($m in [regex]::Matches($condition,'\$(\w+)\s+(-notmatch|-match)\s+''([^'']*)''')){
 $value=Get-Variable -Name $m.Groups[1].Value -ValueOnly
 $violated=if($m.Groups[2].Value -eq '-notmatch'){$value -notmatch $m.Groups[3].Value}else{$value -match $m.Groups[3].Value}
 $results+=@{source=$m.Groups[1].Value;operator=$m.Groups[2].Value;pattern=$m.Groups[3].Value;violated=[bool]$violated}
}
$results|ConvertTo-Json -Depth 4|Set-Content (Join-Path $PSScriptRoot 'original-compound-matrix.json') -Encoding UTF8
$results|Where-Object violated|ForEach-Object{Write-Output ('FAIL '+$_.source+' '+$_.operator+' '+$_.pattern)}
Write-Output "checked=$($results.Count) violated=$(@($results|Where-Object violated).Count)"
# Execute the unchanged compound assertion alone after loading its original source values.
try{Invoke-Expression ($gate.Substring($start,$gate.IndexOf("Write-Host 'PASS elevated navigation",$start)-$start));throw 'Expected original compound failure was not observed'}catch{$_|Out-String|Set-Content (Join-Path $PSScriptRoot 'original-compound-failure.log') -Encoding UTF8;Write-Output $_.Exception.Message}
