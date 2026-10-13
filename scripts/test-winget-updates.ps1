$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '../src-tauri/adapters/windows_inventory.ps1')
. (Join-Path $PSScriptRoot '../src-tauri/adapters/winget_updates.ps1')
function Assert($Condition,[string]$Message){if(-not $Condition){throw $Message}}
$doc='{"Sources":[{"SourceDetails":{"Name":"winget"},"Packages":[{"PackageIdentifier":"Vendor.App","Version":"1.2"},{"PackageIdentifier":"Vendor.App","Version":"1.1"},{"PackageIdentifier":"Other.App","Version":"2"}]}]}'|ConvertFrom-Json
$records=@(Convert-WinGetDocument $doc)
Assert ($records.Count -eq 3 -and @($records|Where-Object {$null -ne $_.PSObject.Properties['catalog_source']}).Count -eq 3) 'fixture export records'
$json='{"schema":1,"status":"COMPLETE","code":null,"rows":[{"display_name":"Example App","package_id":"Vendor.App","installed_version":"1.2","target_version":"2.0","update_available":true},{"display_name":"Similar App","package_id":"Other.App","installed_version":"2","target_version":null,"update_available":false}]}'
$document=$json|ConvertFrom-Json
$rows=@(Convert-WinGetUpdateDocument $document)
Set-WinGetUpdateEvidence $records $rows $null
Assert ($records[0].update.status -eq 'UPDATE_AVAILABLE' -and $records[0].update.target_version -eq '2.0') 'typed available upgrade'
Assert ($records[1].update.status -eq 'UNKNOWN') 'other installed version is not assumed'
Assert ($records[2].update.status -eq 'NO_UPDATE_REPORTED') 'explicit typed no update'
Assert ($records[0].display_name -eq 'Example App') 'actual typed display name'
Set-WinGetUpdateEvidence $records @($rows[0],$rows[0]) $null
Assert ($records[0].update.status -eq 'UPDATE_AVAILABLE') 'identical duplicate candidates'
$conflict=[pscustomobject]@{display_name='Similar App';package_id='Vendor.App';installed_version='1.2';target_version='3';update_available=$true}
Set-WinGetUpdateEvidence $records @($rows[0],$conflict) $null
Assert ($records[0].update.status -eq 'UNKNOWN') 'conflicting duplicate candidates'
# Localized/poisoned human tables have no production parser at all.
foreach($bad in @('{"schema":2,"status":"COMPLETE","rows":[]}','{"schema":1,"status":"COMPLETE","rows":[{"package_id":"Vendor.App"}]}','{"schema":1,"status":"COMPLETE","rows":[{"display_name":"Name","package_id":"Vendor.App","installed_version":"1","target_version":"2","update_available":"true"}]}','Name Id Version Available')){
 $failed=$false;try{Convert-WinGetUpdateDocument ($bad|ConvertFrom-Json)}catch{$failed=$true};Assert $failed 'format and false typed booleans fail closed'
}
foreach($code in @('WINGET_UPDATE_CONNECT_FAILED','WINGET_UPDATE_SEARCH_FAILED','WINGET_UPDATE_LIMIT')){
 $failed=$false;try{Convert-WinGetUpdateDocument ([pscustomobject]@{schema=1;status='ERROR';code=$code;rows=@()})}catch{$failed=$_.Exception.Message -eq $code};Assert $failed 'typed API failure never becomes no report'
}
$poisoned=$json|ConvertFrom-Json;$poisoned.rows[0].display_name="Fake`nVendor.Other 1 99`n-----"
$rows=@(Convert-WinGetUpdateDocument $poisoned)
Assert ($rows[0].package_id -eq 'Vendor.App' -and $rows[0].target_version -eq '2.0') 'multiline display names cannot inject fields'
Assert ((Convert-WinGetBytes ([Text.UTF8Encoding]::new($false).GetBytes([string][char]0x6e2c))) -ceq [string][char]0x6e2c) 'strict UTF8 non-ASCII'
$utf16=[byte[]]([Text.Encoding]::Unicode.GetPreamble()+[Text.Encoding]::Unicode.GetBytes([string][char]0x6e2c))
Assert ((Convert-WinGetBytes $utf16) -ceq [string][char]0x6e2c) 'UTF16 BOM'
$failed=$false;try{Convert-WinGetBytes ([byte[]]@(255,255,255))}catch{$failed=$true};Assert $failed 'invalid encoding'
Set-WinGetUpdateEvidence $records @() 'WINGET_UPDATE_TIMEOUT'
Assert (@($records|Where-Object {$_.update.status -eq 'ERROR'}).Count -eq 3) 'query faults preserve inventory'
# Actual owned child execution tests timeout/nonzero/stdout/cancellation cleanup.
$node=(Get-Command node.exe).Source
$temp=Join-Path ([IO.Path]::GetTempPath()) ('safepatch-test-'+[guid]::NewGuid().ToString('N'))
$null=[IO.Directory]::CreateDirectory($temp);Push-Location $temp
$previousDirectory=[Environment]::CurrentDirectory;[Environment]::CurrentDirectory=$temp
try {
 [IO.File]::WriteAllText((Join-Path $temp 'query'),'setTimeout(()=>{},10000)')
 $failed=$false;try{Invoke-WinGetUpdateProcess $node 500}catch{$failed=$_.Exception.Message -eq 'WINGET_UPDATE_TIMEOUT'};Assert $failed 'owned child timeout'
 [IO.File]::WriteAllText((Join-Path $temp 'query'),'process.exit(7)')
 $failed=$false;try{Invoke-WinGetUpdateProcess $node}catch{$failed=$_.Exception.Message -eq 'WINGET_UPDATE_COMMAND_FAILED'};Assert $failed 'owned child nonzero'
 [IO.File]::WriteAllText((Join-Path $temp 'query'),'process.stdout.write(String.fromCodePoint(0x6e2c))',[Text.UTF8Encoding]::new($false))
 Assert ((Invoke-WinGetUpdateProcess $node) -ceq [string][char]0x6e2c) 'real child UTF8 bytes'
 [IO.File]::WriteAllText((Join-Path $temp 'query'),'setTimeout(()=>{},10000)')
 $env:SAFEPATCH_CANCEL_FILE=Join-Path $temp 'cancel';[IO.File]::WriteAllText($env:SAFEPATCH_CANCEL_FILE,'cancel')
 $failed=$false;try{Invoke-WinGetUpdateProcess $node}catch{$failed=$_.Exception.Message -eq 'CANCELLED'};Assert $failed 'owned child cancellation'
} finally {
 $env:SAFEPATCH_CANCEL_FILE=$null;[Environment]::CurrentDirectory=$previousDirectory;Pop-Location
 foreach($name in @('query','cancel')){$file=Join-Path $temp $name;if([IO.File]::Exists($file)){[IO.File]::Delete($file)}}
 [IO.Directory]::Delete($temp)
}
$env:SAFEPATCH_WINGET_HELPER=$node
function Invoke-WinGetUpdateProcess {throw 'WINGET_UPDATE_TIMEOUT'}
$result=Get-WinGetUpdates 'signature-verified-exe-placeholder' $records
Assert ($result.status -eq 'UNKNOWN' -and $records[0].update.status -eq 'ERROR') 'collector fault boundary'
$env:SAFEPATCH_WINGET_HELPER=$null
function Get-AppxPackage {return @()}
$unavailable=Get-WinGetInventory
Assert ($unavailable.records.Count -eq 0 -and $unavailable.coverage[0].codes[0] -eq 'WINGET_UNAVAILABLE') 'unavailable classified'
Write-Output 'WinGet typed update regressions: PASS; synthetic JSON and owned children, not device acceptance.'
