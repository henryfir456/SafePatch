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
$official='{"Name":"winget","Arg":"https://cdn.winget.microsoft.com/cache","Type":"Microsoft.PreIndexed.Package","Identifier":"Microsoft.Winget.Source_8wekyb3d8bbwe","Data":"Microsoft.Winget.Source_8wekyb3d8bbwe"}' | ConvertFrom-Json
Assert (Test-OfficialSourceDocument $official) 'official source accepted'
$official.Arg='https://private.example.invalid/cache'
Assert (-not (Test-OfficialSourceDocument $official)) 'private source alias rejected'
$official.Arg='https://cdn.winget.microsoft.com/cache'; $official.Type='Microsoft.Rest'
Assert (-not (Test-OfficialSourceDocument $official)) 'REST alias rejected'
Assert (-not (Test-OfficialSourceDocument ([pscustomobject]@{Name='winget'}))) 'missing source metadata rejected'
$official.Type='Microsoft.PreIndexed.Package'; $official.Arg=@('https://cdn.winget.microsoft.com/cache','https://private.example.invalid/cache')
Assert (-not (Test-OfficialSourceDocument $official)) 'array source metadata rejected'
Write-Output 'Adapter regression assertions: PASS (13). Synthetic inputs; not device acceptance.'
