$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '../src-tauri/adapters/windows_inventory.ps1')
. (Join-Path $PSScriptRoot '../src-tauri/adapters/winget_updates.ps1')
function Assert($Condition,[string]$Message){if(-not $Condition){throw $Message}}
$doc='{"Sources":[{"SourceDetails":{"Name":"winget"},"Packages":[{"PackageIdentifier":"Vendor.App","Version":"1.2"},{"PackageIdentifier":"Vendor.App","Version":"1.1"},{"PackageIdentifier":"Other.App","Version":"2"}]}]}'|ConvertFrom-Json
$records=@(Convert-WinGetDocument $doc)
$rows=@(Convert-WinGetUpdateTable "名稱  識別碼  版本  可用`n------------------------------------------------------------`n測試軟體  Vendor.App  1.2  2.0`n相似名稱  Other.App  2`n3 個套件`n" $records)
Assert ($rows.Count -eq 2) 'localized header rows'
Set-WinGetUpdateEvidence $records $rows $null
Assert ($records[0].update.status -eq 'UPDATE_AVAILABLE' -and $records[0].update.target_version -eq '2.0') 'confirmed target'
Assert ($records[1].update.status -eq 'UNKNOWN') 'multi installed versions exact only'
Assert ($records[2].update.status -eq 'NO_UPDATE_REPORTED') 'explicit no update row'
Assert ($records[0].display_name -eq '測試軟體') 'actual display name'
$english=@(Convert-WinGetUpdateTable "Name  Id  Version  Available  Source`r`n------------------------------------------------------------`r`nSimilar App  Vendor.App  1.2  2.0  winget`r`nSimilar App  Other.App  2  winget`r`n" $records)
Assert ($english.Count -eq 2 -and $english[1].target_version -eq $null) 'optional source column'
Set-WinGetUpdateEvidence $records @($english[0],$english[0]) $null
Assert ($records[0].update.status -eq 'UPDATE_AVAILABLE') 'identical duplicate rows'
$conflict=[pscustomobject]@{display_name='Similar';package_id='Vendor.App';installed_version='1.2';target_version='3'}
Set-WinGetUpdateEvidence $records @($english[0],$conflict) $null
Assert ($records[0].update.status -eq 'UNKNOWN') 'conflicting duplicate candidates'
foreach($bad in @("Name  ID  Current`n-----`nName  Vendor.Ap…  1.2  2.0",'no table',"Name`n-----`nName  Untrusted.App  1  2")){
 $failed=$false;try{Convert-WinGetUpdateTable $bad $records}catch{$failed=$true};Assert $failed 'format/truncation/unknown ID fail closed'
}
Assert ((Convert-WinGetBytes ([Text.UTF8Encoding]::new($false).GetBytes('測試軟體'))) -ceq '測試軟體') 'strict UTF8'
$utf16=[byte[]]([Text.Encoding]::Unicode.GetPreamble()+[Text.Encoding]::Unicode.GetBytes('測試軟體'))
Assert ((Convert-WinGetBytes $utf16) -ceq '測試軟體') 'UTF16 BOM'
$failed=$false;try{Convert-WinGetBytes ([byte[]]@(255,255,255))}catch{$failed=$true};Assert $failed 'invalid encoding fails closed'
Set-WinGetUpdateEvidence $records @() 'WINGET_UPDATE_TIMEOUT'
Assert (@($records|Where-Object {$_.update.status -eq 'ERROR' -and $_.update.reason -eq 'WINGET_UPDATE_TIMEOUT'}).Count -eq 3) 'timeout preserves inventory with error'
# Owned synthetic child fixtures verify real timeout / nonzero / cancellation cleanup.
$node=(Get-Command node.exe).Source
$temp=Join-Path ([IO.Path]::GetTempPath()) ('safepatch-test-'+[guid]::NewGuid().ToString('N'))
$null=[IO.Directory]::CreateDirectory($temp)
Push-Location $temp
$previousDirectory=[Environment]::CurrentDirectory
[Environment]::CurrentDirectory=$temp
try {
 [IO.File]::WriteAllText((Join-Path $temp 'list'),'setTimeout(()=>{},10000)')
 $failed=$false;try{Invoke-WinGetUpdateProcess $node 200}catch{$failed=$_.Exception.Message -eq 'WINGET_UPDATE_TIMEOUT'};Assert $failed 'owned child timeout'
 [IO.File]::WriteAllText((Join-Path $temp 'list'),'process.exit(7)')
 $failed=$false;try{Invoke-WinGetUpdateProcess $node}catch{$failed=$_.Exception.Message -eq 'WINGET_UPDATE_COMMAND_FAILED'};Assert $failed 'owned child nonzero'
 [IO.File]::WriteAllText((Join-Path $temp 'list'),'process.stdout.write("測試軟體")',[Text.UTF8Encoding]::new($false))
 Assert ((Invoke-WinGetUpdateProcess $node) -ceq '測試軟體') 'actual child stdout encoding'
 [IO.File]::WriteAllText((Join-Path $temp 'list'),'setTimeout(()=>{},10000)')
 $env:SAFEPATCH_CANCEL_FILE=Join-Path $temp 'cancel';[IO.File]::WriteAllText($env:SAFEPATCH_CANCEL_FILE,'cancel')
 $failed=$false;try{Invoke-WinGetUpdateProcess $node}catch{$failed=$_.Exception.Message -eq 'CANCELLED'};Assert $failed 'owned child cancellation'
} finally {
 $env:SAFEPATCH_CANCEL_FILE=$null;[Environment]::CurrentDirectory=$previousDirectory;Pop-Location
 # Only explicitly owned fixture files are removed.
 foreach($name in @('list','cancel')){$file=Join-Path $temp $name;if([IO.File]::Exists($file)){[IO.File]::Delete($file)}}
 [IO.Directory]::Delete($temp)
}
function Invoke-WinGetUpdateProcess {throw 'WINGET_UPDATE_TIMEOUT'}
$result=Get-WinGetUpdates 'fixture-only' $records
Assert ($result.status -eq 'UNKNOWN' -and $records[0].update.status -eq 'ERROR') 'collector fault boundary'
Write-Output 'WinGet update regression tests: PASS; synthetic tables and owned Node children, not device upgrade acceptance.'
