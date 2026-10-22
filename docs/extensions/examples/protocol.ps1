# Compatible with installed Windows PowerShell and pwsh; no execution-policy bypass.
[Console]::InputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$invocation = [Console]::ReadLine() | ConvertFrom-Json
if ($invocation.protocol_version -ne 1 -or $invocation.op -ne 'invoke') { exit 2 }
[Console]::WriteLine((@{ protocol_version=1; op='host_call'; run_id=$invocation.run_id;
    call_id='describe'; method='host.describe'; params=@{} } | ConvertTo-Json -Compress))
$reply = [Console]::ReadLine() | ConvertFrom-Json
if ($reply.op -ne 'host_result' -or $reply.run_id -ne $invocation.run_id -or $reply.call_id -ne 'describe' -or -not $reply.success) { exit 2 }
[Console]::WriteLine((@{ protocol_version=1; op='complete'; run_id=$invocation.run_id;
    status='success'; value=@{ text='fixture' } } | ConvertTo-Json -Compress))
