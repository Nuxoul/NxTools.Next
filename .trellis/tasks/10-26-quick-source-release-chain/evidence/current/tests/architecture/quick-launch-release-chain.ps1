# Source-only contract. Never execute the build/publisher/writer being inspected.
function Test-QuickLaunchReleaseChain {
    param([string]$BuildSource, [string]$PublisherSource, [string]$WriterSource)

    function Parse-ReleaseSource([string]$Text) {
        $sourceTokens = $null; $sourceErrors = $null
        $ast = [Management.Automation.Language.Parser]::ParseInput($Text, [ref]$sourceTokens, [ref]$sourceErrors)
        if ($sourceErrors.Count) { throw 'Release chain source: invalid PowerShell syntax.' }
        return $ast
    }
    function Compact-ReleaseExpression([string]$Text) {
        $expressionTokens = $null; $expressionErrors = $null
        $null = [Management.Automation.Language.Parser]::ParseInput($Text, [ref]$expressionTokens, [ref]$expressionErrors)
        # Ignore syntactic spacing/separators only; preserve every quoted path/value byte.
        return (@($expressionTokens | Where-Object {
            $_.Kind.ToString() -notin @('NewLine', 'LineContinuation', 'Comment', 'EndOfInput', 'Semi')
        } | ForEach-Object { $_.Kind.ToString() + ':' + $_.Text }) -join "`n")
    }
    function Require-ReleaseFact([bool]$Valid, [string]$Reason) {
        if (-not $Valid) { throw "Release chain source: $Reason" }
    }
    function Get-FixedReleaseCall($Ast, [string]$Name, [bool]$InsideBuildTry) {
        $calls = @($Ast.FindAll({ param($node)
            $node -is [Management.Automation.Language.CommandAst] -and
            $node.InvocationOperator -eq [Management.Automation.Language.TokenKind]::Ampersand -and
            $node.CommandElements[0].Extent.Text -ceq "(Join-Path `$PSScriptRoot '$Name')"
        }, $true))
        Require-ReleaseFact ($calls.Count -eq 1) "one fixed sibling invocation required: $Name"
        $call = $calls[0]
        $container = $call.Parent.Parent
        if ($InsideBuildTry) {
            Require-ReleaseFact ($container -is [Management.Automation.Language.StatementBlockAst] -and
                $container.Parent -is [Management.Automation.Language.TryStatementAst] -and
                $container.Parent.Body -eq $container -and
                $container.Parent.Parent -eq $Ast.EndBlock) "build delegation must execute in original top-level try: $Name"
        } else {
            Require-ReleaseFact ($container -eq $Ast.EndBlock) "publisher invocation must execute at top level: $Name"
        }
        return $call
    }
    function Require-ReleaseArgument($Call, [string]$Name, [string]$Value) {
        $elements = $Call.CommandElements
        $argumentIndices = @(for ($index = 1; $index -lt $elements.Count; $index++) {
            if ($elements[$index] -is [Management.Automation.Language.CommandParameterAst] -and
                $elements[$index].ParameterName -ceq $Name) { $index }
        })
        Require-ReleaseFact ($argumentIndices.Count -eq 1 -and $argumentIndices[0] + 1 -lt $elements.Count) "missing/duplicate argument -$Name"
        Require-ReleaseFact ((Compact-ReleaseExpression $elements[$argumentIndices[0] + 1].Extent.Text) -ceq
            (Compact-ReleaseExpression $Value)) "incorrect argument flow -$Name"
    }
    function Get-ReleaseAssignment($Ast, [string]$Name, [string]$Right, [bool]$HostBranch = $false) {
        $assignments = @($Ast.FindAll({ param($node)
            $node -is [Management.Automation.Language.AssignmentStatementAst] -and
            $node.Operator -eq [Management.Automation.Language.TokenKind]::Equals -and
            $node.Left.Extent.Text -ceq ('$' + $Name)
        }, $true))
        Require-ReleaseFact ($assignments.Count -eq 1) "one assignment required: $Name"
        Require-ReleaseFact (($HostBranch -or $assignments[0].Parent -eq $Ast.EndBlock) -and
            (Compact-ReleaseExpression $assignments[0].Right.Extent.Text) -ceq
            (Compact-ReleaseExpression $Right)) "incorrect binding: $Name"
        return $assignments[0]
    }
    function Require-ReleaseStop($Ast) {
        $null = Get-ReleaseAssignment $Ast 'ErrorActionPreference' "'Stop'"
        Require-ReleaseFact (@($Ast.FindAll({ param($node)
            $node -is [Management.Automation.Language.TryStatementAst] -and $node.CatchClauses.Count -gt 0
        }, $true)).Count -eq 0) 'release failures must not be swallowed by catch.'
    }
    $build = Parse-ReleaseSource $BuildSource
    $publisher = Parse-ReleaseSource $PublisherSource
    $writer = Parse-ReleaseSource $WriterSource
    Require-ReleaseStop $build; Require-ReleaseStop $publisher; Require-ReleaseStop $writer
    $delegate = Get-FixedReleaseCall $build 'publish-release-hosts.ps1' $true
    Require-ReleaseArgument $delegate 'Configuration' '$Configuration'
    Require-ReleaseArgument $delegate 'GeneratedDirectory' "(Join-Path `$hostBuildRoot 'generated')"
    Require-ReleaseArgument $delegate 'BuildDirectory' '$hostBuildRoot'
    Require-ReleaseFact ($delegate.CommandElements.Count -eq 7) 'build must not add skip/alternate publisher arguments.'

    $identity = Get-FixedReleaseCall $publisher 'write-shell-selection-identity.ps1' $false
    Require-ReleaseArgument $identity 'DesktopPath' "(Join-Path `$TargetDirectory 'nxtools-desktop.exe')"
    Require-ReleaseArgument $identity 'OutputPath' '$identityPath'
    $manifest = Get-FixedReleaseCall $publisher 'write-release-binary-manifest.ps1' $false
    foreach ($pair in @(@('Profile','$profile'), @('TargetDirectory','$TargetDirectory'),
        @('ShellHelperDirectory','$helperPublish'), @('OutputPath','$releasePath'), @('ArchiveOutputPath','$archivePath'))) {
        Require-ReleaseArgument $manifest $pair[0] $pair[1]
    }
    Require-ReleaseFact ($manifest.CommandElements.Count -eq 11) 'manifest invocation must preserve exact input/output ownership.'
    $profileBinding = Get-ReleaseAssignment $publisher 'profile' '$Configuration.ToLowerInvariant()'
    $identityPath = Get-ReleaseAssignment $publisher 'identityPath' "Join-Path `$GeneratedDirectory 'DesktopIdentity.g.cs'"
    $helperPath = Get-ReleaseAssignment $publisher 'helperPublish' "Join-Path `$BuildDirectory 'shell-publish'"
    $releasePath = Get-ReleaseAssignment $publisher 'releasePath' "Join-Path `$GeneratedDirectory 'ReleaseBinaryManifest.g.cs'"
    $archivePath = Get-ReleaseAssignment $publisher 'archivePath' "Join-Path `$GeneratedDirectory 'ArchiveReleaseManifest.g.cs'"
    $helperCalls = @($publisher.FindAll({ param($node)
        $node -is [Management.Automation.Language.CommandAst] -and $node.GetCommandName() -eq 'dotnet' -and
        $node.Extent.Text.Contains("(Join-Path `$root 'csharp\NxTools.ShellSelectionHelper\NxTools.ShellSelectionHelper.csproj')")
    }, $true))
    Require-ReleaseFact ($helperCalls.Count -eq 1) 'fixed helper publish is required.'
    $helper = $helperCalls[0]
    Require-ReleaseFact ($helper.Parent.Parent -eq $publisher.EndBlock -and
        $helper.CommandElements[1].Extent.Text -ceq 'publish' -and
        $helper.Extent.Text.Contains('"-p:ShellSelectionIdentityPath=$identityPath"')) 'helper must consume actual Desktop identity.'
    Require-ReleaseArgument $helper 'o' '$helperPublish'
    $statements = $publisher.EndBlock.Statements
    $helperIndex = [array]::IndexOf($statements, $helper.Parent)
    Require-ReleaseFact ($helperIndex -ge 0 -and $helperIndex + 1 -lt $statements.Count -and
        (Compact-ReleaseExpression $statements[$helperIndex + 1].Extent.Text) -ceq
        (Compact-ReleaseExpression 'if ($LASTEXITCODE -ne 0) { throw "ShellSelectionHelper publish failed: $LASTEXITCODE" }')) 'helper publish must fail closed before manifest generation.'
    $publishArgs = Get-ReleaseAssignment $publisher 'publishArgs' "@('publish', (Join-Path `$root 'csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj'), '-c', `$Configuration, '-r', 'win-x64', '--self-contained', 'false', '-p:PublishSingleFile=true', `"-p:ReleaseBinaryManifestPath=`$releasePath`", `"-p:ArchiveReleaseManifestPath=`$archivePath`", '--artifacts-path', (Join-Path `$BuildDirectory 'host-build'), '-o', (Join-Path `$BuildDirectory 'host-publish'), '--nologo')" $true
    $hostBlock = $publishArgs.Parent
    Require-ReleaseFact ($hostBlock -is [Management.Automation.Language.StatementBlockAst] -and
        $hostBlock.Parent -is [Management.Automation.Language.IfStatementAst] -and
        $hostBlock.Parent.Parent -eq $publisher.EndBlock -and
        (Compact-ReleaseExpression $hostBlock.Parent.Clauses[0].Item1.Extent.Text) -ceq (Compact-ReleaseExpression '-not $SkipPlatformHost') -and
        $hostBlock.Parent.Clauses[0].Item2 -eq $hostBlock) 'PlatformHost publish must retain its original enabled branch.'
    Require-ReleaseFact ($hostBlock.Statements.Count -eq 4 -and
        (Compact-ReleaseExpression $hostBlock.Statements[2].Extent.Text) -ceq (Compact-ReleaseExpression '& dotnet @publishArgs') -and
        (Compact-ReleaseExpression $hostBlock.Statements[3].Extent.Text) -ceq
        (Compact-ReleaseExpression 'if ($LASTEXITCODE -ne 0) { throw "PlatformHost publish failed: $LASTEXITCODE" }')) 'PlatformHost must consume generated manifests and propagate publish failure.'
    $ordered = @($profileBinding, $identityPath, $identity, $helperPath, $helper, $releasePath, $archivePath, $manifest, $hostBlock.Parent)
    for ($index = 1; $index -lt $ordered.Count; $index++) {
        Require-ReleaseFact ($ordered[$index - 1].Extent.EndOffset -lt $ordered[$index].Extent.StartOffset) 'identity/helper/manifest/PlatformHost dependency order is broken.'
    }
    $hashFunction = @($writer.FindAll({ param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq 'Get-RequiredHash'
    }, $true))
    Require-ReleaseFact ($hashFunction.Count -eq 1 -and
        (Compact-ReleaseExpression $hashFunction[0].Body.Extent.Text) -ceq
        (Compact-ReleaseExpression '{ if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Trusted release binary is missing: $Path" }; return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash }')) 'writer must require each real file and compute SHA256, not accept a hash literal.'
    $hashBindings = @()
    foreach ($pair in @(@('indexerHash','nxtools-indexer.exe'), @('desktopHash','nxtools-desktop.exe'),
        @('browserHostHash','nxtools-browser-host.exe'), @('workerHash','nxtools-archive-worker.exe'))) {
        $hashBindings += Get-ReleaseAssignment $writer $pair[0] "Get-RequiredHash (Join-Path `$TargetDirectory '$($pair[1])')"
    }
    $helperHashLoop = @($writer.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.ForEachStatementAst] })
    Require-ReleaseFact ($helperHashLoop.Count -eq 1 -and
        (Compact-ReleaseExpression $helperHashLoop[0].Condition.Extent.Text) -ceq
        (Compact-ReleaseExpression "@('NxTools.ShellSelectionHelper.exe', 'NxTools.ShellSelectionHelper.dll', 'NxTools.ShellSelectionHelper.deps.json', 'NxTools.ShellSelectionHelper.runtimeconfig.json')") -and
        $helperHashLoop[0].Body.Extent.Text.Contains('Get-RequiredHash (Join-Path $ShellHelperDirectory $name)')) 'all fixed Shell helper members must be hashed.'
    $writes = @($writer.EndBlock.Statements | Where-Object {
        $_ -is [Management.Automation.Language.PipelineAst] -and $_.PipelineElements.Count -eq 1 -and
        $_.PipelineElements[0] -is [Management.Automation.Language.CommandAst] -and
        $_.PipelineElements[0].GetCommandName() -ceq 'Write-Generated'
    })
    Require-ReleaseFact ($writes.Count -eq 2 -and
        $writes[0].PipelineElements[0].CommandElements[1].Extent.Text -ceq '$OutputPath' -and
        $writes[1].PipelineElements[0].CommandElements[1].Extent.Text -ceq '$ArchiveOutputPath' -and
        $writes[0].Extent.Text.Contains('IndexerSha256 = "$indexerHash"') -and
        $writes[0].Extent.Text.Contains('DesktopSha256 = "$desktopHash"') -and
        $writes[0].Extent.Text.Contains('BrowserHostSha256 = "$browserHostHash"') -and
        $writes[1].Extent.Text.Contains('$($archiveRows -join "`n")')) 'writer must publish the computed release/archive identities.'
    foreach ($hashBinding in @($hashBindings) + @($helperHashLoop)) {
        Require-ReleaseFact ($hashBinding.Extent.EndOffset -lt $writes[0].Extent.StartOffset) 'all hashes must validate before either manifest changes.'
    }
    return $true
}
