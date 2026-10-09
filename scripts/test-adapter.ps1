$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '../src-tauri/adapters/windows_inventory.ps1')
function Assert($Condition,[string]$Message){if(-not $Condition){throw $Message}}
Assert ((Get-PathHash 'C:\Private\Example') -eq (Get-PathHash 'c:\private\example')) 'path hash normalization'
Assert ((Get-CleanText "bad`0name") -eq 'badname') 'untrusted control character'
Assert ((Get-CleanText ('x'*2048)).Length -eq 1024) 'bounded field'
$doc='{"Sources":[{"SourceDetails":{"Name":"winget"},"Packages":[{"PackageIdentifier":"Vendor.Example","Version":"1.2.3"},{"PackageIdentifier":"Vendor.Other","Version":"Unknown"}]}]}' | ConvertFrom-Json
$items=@(Convert-WinGetDocument $doc)
Assert ($items.Count -eq 2) 'catalog count'
Assert ($items[0].installed_version -eq '1.2.3') 'exact version'
Assert ($null -eq $items[1].installed_version) 'unknown version is unknown'
Assert ($items[0].installation_source -eq 'UNKNOWN') 'catalog does not prove installation source'
$failed=$false
try { Convert-WinGetDocument ('{"bad":true}' | ConvertFrom-Json) } catch {$failed=$true}
Assert $failed 'malformed JSON rejected'
Write-Output 'Adapter regression assertions: PASS (8). Synthetic inputs; not device acceptance.'
