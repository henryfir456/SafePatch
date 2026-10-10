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
    [IO.File]::WriteAllText((Join-Path $store 'extensions.json'),'[{"identifier":{"id":"ExamplePublisher.sample-tool"},"version":"1.2.3","relativeLocation":"first"}]')
    [IO.File]::WriteAllText((Join-Path $userData 'profiles/abc123/extensions.json'),'[{"identifier":{"id":"ExamplePublisher.sample-tool"},"version":"1.2.4","relativeLocation":"second"}]')
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
    Assert (-not ($inventory | ConvertTo-Json -Depth 8 -Compress).Contains($testRoot.Replace('\','\\'))) 'raw private paths never leave adapter'
    $brokenData=Join-Path $testRoot 'BrokenUserData'
    [void][IO.Directory]::CreateDirectory($brokenData)
    [IO.File]::WriteAllText((Join-Path $brokenData 'profiles'),'not a directory')
    $isolated=$null
    try{$isolated=Get-VSCodeInventory @([pscustomobject]@{extension_root=$store;user_data=$brokenData;application_roots=@($appRoot);channel='stable';scope='user'})}catch{}
    Assert ($null -ne $isolated -and @($isolated.records).Count -eq 3) 'profile enumeration error preserves extensions and applications'
    [void][IO.Directory]::CreateDirectory((Join-Path $store 'collision'))
    [IO.File]::WriteAllText((Join-Path $store 'collision/package.json'),'{"publisher":"ExamplePublisher","name":"sample-tool","version":"1.2.3"}')
    $collision=Get-VSCodeInventory @([pscustomobject]@{extension_root=$store;user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})
    Assert (@($collision.records | Where-Object {$_.installed_version -eq '1.2.3' -and $_.profile_ref -eq 'stable-default'}).Count -eq 1) 'only indexed location can claim a profile'
    Assert (@($collision.records | Where-Object {$_.installed_version -eq '1.2.3' -and $_.profile_ref -eq 'UNKNOWN'}).Count -eq 1) 'same ID and version at another location has UNKNOWN profile'
    [IO.File]::WriteAllText((Join-Path $store '.obsolete'),'{"obsolete":true,"second":[true,false]}')
    $uncertain=Get-VSCodeInventory @([pscustomobject]@{extension_root=$store;user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})
    Assert (@($uncertain.records | Where-Object scanner -eq 'vscode-extension').Count -eq 3) 'non-boolean obsolete metadata cannot hide an extension'
    Assert (-not (Test-LocalMetadataPath '\\private.example.invalid\share')) 'UNC metadata roots are rejected without traversal'
    $brokenStore=Join-Path $testRoot 'BrokenStore'
    [IO.File]::WriteAllText($brokenStore,'not a directory')
    $badStore=Get-VSCodeInventory @([pscustomobject]@{extension_root=$brokenStore;user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})
    Assert (@($badStore.records | Where-Object scanner -eq 'vscode').Count -eq 1) 'extension enumeration error preserves applications'
    $malformed=Join-Path $testRoot 'malformed.json'
    [IO.File]::WriteAllText($malformed,'{invalid json}')
    $rejected=$false
    try{Read-LocalMetadataJson $malformed}catch{$rejected=$_.Exception.Message -eq 'METADATA_JSON_INVALID'}
    Assert $rejected 'corrupt JSON has stable failure code'
    [IO.File]::WriteAllText($malformed,('x'*1048577))
    $rejected=$false
    try{Read-LocalMetadataJson $malformed}catch{$rejected=$_.Exception.Message -eq 'METADATA_SIZE_LIMIT'}
    Assert $rejected 'oversized metadata is bounded'
    $uriEntry=@([pscustomobject]@{identifier=@{id='ExamplePublisher.sample-tool'};version='1.2.3';location=@{scheme='file';path=('/'+(Join-Path $store 'first').Replace('\','/'))}})
    [IO.File]::WriteAllText((Join-Path $store 'extensions.json'),(ConvertTo-Json -InputObject $uriEntry -Depth 5))
    $uriInventory=Get-VSCodeInventory @([pscustomobject]@{extension_root=$store;user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})
    Assert (@($uriInventory.records | Where-Object {$_.installed_version -eq '1.2.3' -and $_.profile_ref -eq 'stable-default'}).Count -eq 1) 'URI profile location works without optional authority'
    $versionedRoot=Join-Path $testRoot 'VersionedApplication'
    $versionedMetadata=Join-Path $versionedRoot 'c0ffee/resources/app'
    [void][IO.Directory]::CreateDirectory($versionedMetadata)
    foreach($name in @('package','product')){[IO.File]::Copy((Join-Path $appRoot ('resources/app/'+$name+'.json')),(Join-Path $versionedMetadata ($name+'.json')))}
    $versioned=Get-VSCodeInventory @([pscustomobject]@{extension_root=$brokenStore;user_data=$userData;application_roots=@($versionedRoot);channel='stable';scope='user'})
    Assert (@($versioned.records | Where-Object {$_.scanner -eq 'vscode' -and $_.installed_version -eq '1.99.0'}).Count -eq 1) 'versioned VS Code installation layout is inventoried'
    $mixed=Get-VSCodeInventory @([pscustomobject]@{extension_root=$store.Replace('\','/');user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})
    Assert (@($mixed.records | Where-Object {$_.installed_version -eq '1.2.3' -and $_.profile_ref -eq 'stable-default'}).Count -eq 1) 'mixed path separators retain exact profile identity'
    [IO.File]::WriteAllText((Join-Path $store 'first/package.json'),'{"publisher":"ExamplePublisher","name":"sample-tool","version":"1.2.3-beta"}')
    [IO.File]::WriteAllText((Join-Path $store 'extensions.json'),'[{"identifier":{"id":"ExamplePublisher.sample-tool"},"version":"1.2.3-Beta","relativeLocation":"first"}]')
    $caseMismatch=Get-VSCodeInventory @([pscustomobject]@{extension_root=$store;user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})
    Assert (@($caseMismatch.records | Where-Object {$_.installed_version -eq '1.2.3-beta' -and $_.profile_ref -eq 'UNKNOWN'}).Count -eq 1) 'prerelease version casing mismatch cannot claim a profile'
    [IO.File]::WriteAllText((Join-Path $appRoot 'resources/app/product.json'),'{"nameShort":"UNKNOWN","applicationName":"code"}')
    $unknownName=Get-VSCodeInventory @([pscustomobject]@{extension_root=$brokenStore;user_data=$userData;application_roots=@($appRoot);channel='stable';scope='user'})
    Assert (@($unknownName.records | Where-Object {$_.scanner -eq 'vscode' -and $_.display_name -ne 'UNKNOWN'}).Count -eq 1) 'unknown app name falls back before Rust normalization'
} finally {
    $resolved=[IO.Path]::GetFullPath($testRoot)
    if($resolved.StartsWith($tempBase,[StringComparison]::OrdinalIgnoreCase) -and [IO.Directory]::Exists($resolved)){Remove-Item -LiteralPath $resolved -Recurse -Force}
}
Write-Output 'VS Code metadata assertions: PASS (29). Authored files; not device acceptance.'
