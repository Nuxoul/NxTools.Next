param([string]$ImageDirectory='artifacts\ui-quick-find-steward')
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
$canvas=[Drawing.Color]::FromArgb(255,24,24,27).ToArgb()
foreach($mode in @('collapsed','results','actions')) {
    $imagePath=Join-Path $ImageDirectory ($mode+'.png')
    $bitmap=[Drawing.Bitmap]::FromFile((Resolve-Path $imagePath).Path)
    try {
        $expectedHeight=if($mode -eq 'collapsed'){44}else{410}
        if($bitmap.Width -ne 620 -or $bitmap.Height -ne $expectedHeight){throw "$mode client dimensions changed"}
        foreach($sample in @(@(0,0),@(0,20),@(619,20),@(310,0),@(310,42),@(0,($expectedHeight-1)),@(619,($expectedHeight-1)))) {
            $actual=$bitmap.GetPixel($sample[0],$sample[1]).ToArgb()
            if($actual -ne $canvas){throw ($mode+' duplicate shell/search outline at '+$sample[0]+','+$sample[1]+': '+$actual.ToString('X8'))}
        }
        # The Kit input must still have its own boundary; do not make all borders vanish.
        if($bitmap.GetPixel(310,6).ToArgb() -eq $canvas){throw "$mode lost the actual Kit input boundary"}
        Write-Output ("PASS $mode original620x$expectedHeight client: one visible Kit input outline, no enclosing search outline")
    } finally { $bitmap.Dispose() }
}
