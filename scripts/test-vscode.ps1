$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '../src-tauri/adapters/windows_inventory.ps1')
$adapter=Join-Path $PSScriptRoot '../src-tauri/adapters/vscode_inventory.ps1'
if(Test-Path -LiteralPath $adapter){. $adapter}
function Assert($Condition,[string]$Message){if(-not $Condition){throw $Message}}
$manifest='{"publisher":"ExamplePublisher","name":"sample-tool","version":"1.2.3","displayName":"Sample tool","main":"./untrusted.js","scripts":{"postinstall":"never execute"}}' | ConvertFrom-Json
$record=$null
try {$record=Convert-VSCodeExtensionManifest $manifest ('a'*64) 'user' 'default'} catch [System.Management.Automation.CommandNotFoundException] {}
Assert ($null -ne $record) 'Extension ID and version inventory is not implemented'
Assert ($record.normalized_id -eq 'vscode-extension:examplepublisher.sample-tool') 'publisher-qualified extension ID'
Assert ($record.installed_version -eq '1.2.3') 'exact version'
Assert ($record.publisher -eq 'ExamplePublisher') 'manifest publisher identity retained as claimed metadata'
Assert ($record.installation_source -eq 'UNKNOWN') 'manifest cannot establish install origin'
Assert ($record.profile_ref -eq 'default') 'profile association retained'
$manifest.version="1.2.3`0"
$poisoned=Convert-VSCodeExtensionManifest $manifest ('a'*64) 'user' 'default'
Assert ($null -eq $poisoned.installed_version) 'poisoned version is UNKNOWN, not silently repaired'
$manifest.publisher=@('ExamplePublisher','OtherPublisher')
$rejected=$false
try {Convert-VSCodeExtensionManifest $manifest ('a'*64) 'user' 'default'} catch {$rejected=$_.Exception.Message -eq 'VSCODE_MANIFEST_INVALID'}
Assert $rejected 'non-scalar publisher is rejected with a stable code'
$tempBase=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
$testRoot=[IO.Path]::GetFullPath((Join-Path $tempBase ('safepatch-vscode-tests-'+[guid]::NewGuid().ToString('N'))))
if(-not $testRoot.StartsWith($tempBase,[StringComparison]::OrdinalIgnoreCase)){throw 'TEST_ROOT_OUTSIDE_TEMP'}
try {
    $store=Join-Path $testRoot 'extensions'; $userData=Join-Path $testRoot 'CodeUser'; $appRoot=Join-Path $testRoot 'Application'
    foreach($dir in @('extensions/first','extensions/second','extensions/obsolete','CodeUser/profiles/abc123','Application/resources/app')){[void][IO.Directory]::CreateDirectory((Join-Path $testRoot $dir))}
    [IO.File]::WriteAllText((Join-Path $store 'first/package.json'),'{"publisher":"ExamplePublisher","name":"sample-tool","version":"1.2.3","main":"./untrusted.js"}')
    [IO.File]::WriteAllText((Join-Path $store 'second/package.json'),'{"publisher":"ExamplePublisher","name":"sample-tool","version":"1.2.4"}')
    [IO.File]::WriteAllText((Join-Path $store 'obsolete/package.json'),'{"publisher":"ExamplePublisher","name":"old-tool","version":"1.0.0"}')
    [IO.File]::WriteAllText((Join-Path $store '.obsolete'),'{"obsolete":true}')
    [IO.File]::WriteAllText((Join-Path $store 'extensions.json'),'[{"identifier":{"id":"ExamplePublisher.sample-tool"},"version":"1.2.3"}]')
    [IO.File]::WriteAllText((Join-Path $userData 'profiles/abc123/extensions.json'),'[{"identifier":{"id":"ExamplePublisher.sample-tool"},"version":"1.2.4"}]')
    [IO.File]::WriteAllText((Join-Path $appRoot 'resources/app/package.json'),'{"name":"code-oss","version":"1.99.0"}')
    [IO.File]::WriteAllText((Join-Path $appRoot 'resources/app/product.json'),'{"nameShort":"Visual Studio Code","applicationName":"code"}')
    $marker=Join-Path $testRoot 'UNTRUSTED_CODE_EXECUTED'
    [IO.File]::WriteAllText((Join-Path $store 'first/untrusted.js'),('require("fs").writeFileSync('+('"'+$marker.Replace('\','\\')+'"')+',"bad");'))
    $inventory=$null
    try {$inventory=Get-VSCodeInventory @([pscustomobject]@{extension_root=$store;user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})} catch [System.Management.Automation.CommandNotFoundException] {}
    Assert ($null -ne $inventory -and @($inventory.records).Count -eq 3) 'Read-only VS Code file inventory with obsolete exclusion is not implemented'
    $extensions=@($inventory.records | Where-Object scanner -eq 'vscode-extension')
    Assert ($extensions.Count -eq 2) 'separate exact extension versions'
    Assert (@($extensions | Where-Object { $_.installed_version -eq '1.2.3' -and $_.profile_ref -eq 'stable-default' }).Count -eq 1) 'default profile matches indexed version'
    Assert (@($extensions | Where-Object { $_.installed_version -eq '1.2.4' -and $_.profile_ref -eq 'stable-abc123' }).Count -eq 1) 'named profile matches indexed version'
    Assert (@($inventory.records | Where-Object { $_.scanner -eq 'vscode' -and $_.installed_version -eq '1.99.0' }).Count -eq 1) 'application version read from actual manifest'
    Assert (-not [IO.File]::Exists($marker)) 'extension entry point must not execute'
    Assert (@($inventory.coverage | Where-Object { $_.scanner -eq 'vscode-extension' -and $_.status -eq 'PARTIAL' }).Count -eq 1) 'metadata and profile coverage uncertainty stays explicit'
    Assert (($inventory | ConvertTo-Json -Depth 8 -Compress) -notlike ('*'+$testRoot+'*')) 'raw private paths never leave adapter'
} finally {
    $resolved=[IO.Path]::GetFullPath($testRoot)
    if($resolved.StartsWith($tempBase,[StringComparison]::OrdinalIgnoreCase) -and [IO.Directory]::Exists($resolved)){Remove-Item -LiteralPath $resolved -Recurse -Force}
}
Write-Output 'VS Code metadata assertions: PASS (16). Authored files; not device acceptance.'
