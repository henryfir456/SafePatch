param([switch]$RequireWinGet)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '../src-tauri/adapters/windows_inventory.ps1')
. (Join-Path $PSScriptRoot '../src-tauri/adapters/winget_updates.ps1')
$helper=Join-Path $PSScriptRoot '../src-tauri/target/winget-helper/SafePatch.WinGetHelper.exe'
if([IO.File]::Exists($helper)){$env:SAFEPATCH_WINGET_HELPER=[IO.Path]::GetFullPath($helper)}
$timer=[Diagnostics.Stopwatch]::StartNew()
$registry=Get-RegistryInventory
$winget=Get-WinGetInventory
$records=@($registry.records)+@($winget.records)
if($registry.records.Count -eq 0){throw 'Real Registry acceptance failed: no records'}
if($RequireWinGet -and $winget.records.Count -eq 0){throw ('Real WinGet acceptance failed: '+($winget.coverage[0].codes -join ','))}
$summary=[pscustomobject]@{os_build=[Environment]::OSVersion.Version.ToString();powershell=$PSVersionTable.PSVersion.ToString();registry_count=$registry.records.Count;winget_count=$winget.records.Count;versioned_count=@($records | Where-Object {$null -ne $_.installed_version}).Count;publisher_count=@($records | Where-Object {$null -ne $_.publisher}).Count;raw_paths_stored=$false;elapsed_seconds=[Math]::Round($timer.Elapsed.TotalSeconds,2);coverage=@($registry.coverage)+@($winget.coverage);verification='ADAPTER_ONLY; TAURI_END_TO_END_NOT_VERIFIED'}
$summary | ConvertTo-Json -Depth 6
