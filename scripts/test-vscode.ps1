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
Write-Output 'VS Code parser assertions: PASS (8). Authored metadata; not device acceptance.'
