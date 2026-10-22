param([string]$DesktopExecutable='')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$core=Get-Content (Join-Path $root 'rust\crates\nxtools-core\src\lib.rs') -Raw
$bootstrap=Get-Content (Join-Path $root 'rust\crates\nxtools-core\src\bundled_extensions.rs') -Raw
$resources=Get-Content (Join-Path $root 'rust\crates\nxtools-extensions\src\bundled.rs') -Raw
if($core -notmatch 'pub fn load_or_create\(paths: &AppPaths\)[\s\S]*?bundled_extensions::load_or_create\(paths\)'){throw 'Production configuration initialization bypasses bundled extensions'}
foreach($required in @('lock_exclusive','bundled_defaults_initialized','bundled_previews','stage_into','extension.bundled.terminal')){if(-not $bootstrap.Contains($required)){throw "Missing bootstrap boundary: $required"}}
foreach($name in @('github-public-proxy','download-link-list','clipboard-text-tools')){if(-not $resources.Contains('package!("'+$name+'")')){throw "Missing embedded package: $name"}}
Write-Output 'PASS production first-run bootstrap and three compiled resource declarations (source boundary only)'
if(-not $DesktopExecutable){return}
Add-Type @'
public static class BundledResourceBytes {
 public static bool Contains(byte[] haystack, byte[] needle) {
  if (needle.Length == 0) return false;
  for(int i=0;i<=haystack.Length-needle.Length;i++) {
   if(haystack[i]!=needle[0]) continue;
   int j=1; while(j<needle.Length && haystack[i+j]==needle[j]) j++;
   if(j==needle.Length) return true;
  }
  return false;
 }
}
'@
$binary=[IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $DesktopExecutable).Path)
$checked=0
foreach($name in @('github-public-proxy','download-link-list','clipboard-text-tools')) {
 $source=Join-Path $root ('docs\extensions\examples\'+$name)
 $manifest=Get-Content (Join-Path $source 'extension.json') -Raw -Encoding UTF8 | ConvertFrom-Json
 foreach($relative in @('extension.json')+@($manifest.files)) {
  $bytes=[IO.File]::ReadAllBytes((Join-Path $source $relative))
  if(-not [BundledResourceBytes]::Contains($binary,$bytes)){throw "Packaged Desktop lacks exact embedded resource: $name/$relative"}
  $checked++
 }
}
Write-Output "PASS packaged production Desktop contains all $checked exact manifest/source resources for three built-in extensions; first-run behavior is separately tested"
