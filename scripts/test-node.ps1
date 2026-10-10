$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '../src-tauri/adapters/windows_inventory.ps1')
. (Join-Path $PSScriptRoot '../src-tauri/adapters/vscode_inventory.ps1')
$adapter=Join-Path $PSScriptRoot '../src-tauri/adapters/node_inventory.ps1'
if(Test-Path -LiteralPath $adapter){. $adapter}
function Assert($Condition,[string]$Message){if(-not $Condition){throw $Message}}
$metadata=[pscustomobject]@{OriginalFilename='node.exe';ProductName='Node.js';ProductVersion='24.20.0';CompanyName='Node.js'}
$record=$null
try{$record=Convert-NodeFileMetadata $metadata ('a'*64) 'user' 'nvm'}catch [System.Management.Automation.CommandNotFoundException]{}
Assert ($null -ne $record) 'Read-only Node PE metadata conversion is not implemented'
Assert ($record.installed_version -ceq '24.20.0') 'exact product resource version'
Assert ($record.normalized_id -eq 'nodejs:node') 'claimed Node identity is namespaced'
Assert ($record.installation_source -eq 'UNKNOWN') 'directory hint cannot establish install source'
Assert ($record.source_id -eq ('a'*64)) 'runtime path hash is distinct identity'
$metadata.ProductVersion="24.20.0`0"
$poisoned=Convert-NodeFileMetadata $metadata ('b'*64) 'user' 'path'
Assert ($null -eq $poisoned.installed_version) 'invalid version cannot be silently repaired'
$metadata.ProductName='Other program'
$mismatch=Convert-NodeFileMetadata $metadata ('b'*64) 'unknown' 'path'
Assert ($null -eq $mismatch.normalized_id -and $null -eq $mismatch.installed_version) 'node.exe name alone cannot establish runtime identity'
$missing=Convert-NodeFileMetadata $null ('c'*64) 'machine' 'default'
Assert ($null -eq $missing.installed_version) 'missing resource metadata remains UNKNOWN'
$discovered=@(Get-NodeMetadataRoots)
$fnmDefault=[IO.Path]::Combine([Environment]::GetFolderPath([Environment+SpecialFolder]::ApplicationData),'fnm/node-versions')
$fnmLegacy=[IO.Path]::Combine([Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile),'.fnm/node-versions')
Assert (@($discovered | Where-Object {$_.hint -eq 'fnm' -and $_.path -eq $fnmDefault}).Count -gt 0) 'fnm Windows default uses roaming application data'
Assert (@($discovered | Where-Object {$_.hint -eq 'fnm' -and $_.path -eq $fnmLegacy}).Count -gt 0) 'legacy fnm home directory remains discoverable'
$schedule=[pscustomobject]@{checked_at='2026-10-10';releases=@{
    v24=@{start='2025-05-06';lts='2025-10-28';maintenance='2026-10-20';end='2028-04-30'}
    v22=@{start='2024-04-24';lts='2024-10-29';maintenance='2025-10-21';end='2027-04-30'}
    v20=@{start='2023-04-18';lts='2023-10-24';maintenance='2024-10-22';end='2026-04-30'}
    v26=@{start='2026-05-05';lts='2026-10-28';maintenance='2027-10-20';end='2029-04-30'}
}}
$support=$null
try{$support=Get-NodeSupport '24.20.0' $schedule '2026-10-10'}catch [System.Management.Automation.CommandNotFoundException]{}
Assert ($null -ne $support -and $support.status -eq 'ACTIVE_LTS') 'official schedule support lifecycle is not implemented'
Assert ((Get-NodeSupport '22.0.0' $schedule '2026-10-10').status -eq 'MAINTENANCE_LTS') 'maintenance LTS remains distinct'
Assert ((Get-NodeSupport '20.0.0' $schedule '2026-10-10').status -eq 'EOL') 'ended release remains EOL, independent of vulnerabilities'
Assert ((Get-NodeSupport '26.0.0' $schedule '2026-10-10').status -eq 'CURRENT') 'current release is distinct from LTS'
Assert ((Get-NodeSupport '99.0.0' $schedule '2026-10-10').status -eq 'UNKNOWN') 'missing release schedule is UNKNOWN'
Assert ((Get-NodeSupport '24.0.0-beta' $schedule '2026-10-10').status -eq 'UNKNOWN') 'prerelease support is UNKNOWN'
Assert ((Get-NodeSupport '24.20.0' $schedule '2026-12-10').status -eq 'UNKNOWN') 'stale schedule cannot establish support'
$originalCulture=[Threading.Thread]::CurrentThread.CurrentCulture
try{
    [Threading.Thread]::CurrentThread.CurrentCulture=[Globalization.CultureInfo]::GetCultureInfo('th-TH')
    $invariant=Get-NodeSupport '24.20.0' $schedule '2026-10-10'
    Assert ($invariant.checked_at -ceq '2026-10-10' -and $invariant.end_date -ceq '2028-04-30') 'lifecycle source dates use invariant Gregorian calendar'
}finally{[Threading.Thread]::CurrentThread.CurrentCulture=$originalCulture}
$tempBase=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
$testRoot=[IO.Path]::GetFullPath((Join-Path $tempBase ('safepatch-node-tests-'+[guid]::NewGuid().ToString('N'))))
if(-not $testRoot.StartsWith($tempBase,[StringComparison]::OrdinalIgnoreCase)){throw 'TEST_ROOT_OUTSIDE_TEMP'}
try{
    foreach($dir in @('direct','versions/v22','versions/v24')){[void][IO.Directory]::CreateDirectory((Join-Path $testRoot $dir));[IO.File]::WriteAllText((Join-Path $testRoot ($dir+'/node.exe')),'not executable; never invoke')}
    $roots=@([pscustomobject]@{path=(Join-Path $testRoot 'direct');layout='direct';scope='user';hint='default'},[pscustomobject]@{path=(Join-Path $testRoot 'versions');layout='versions';scope='user';hint='nvm'})
    $inventory=$null
    try{$inventory=Get-NodeInventory $roots}catch [System.Management.Automation.CommandNotFoundException]{}
    Assert ($null -ne $inventory -and @($inventory.records).Count -eq 3) 'distinct runtime candidate locations are inventoried without executing files'
    Assert (@($inventory.records | Where-Object {$null -ne $_.installed_version}).Count -eq 0) 'non-PE candidates never fabricate versions'
    Assert (@($inventory.records.source_id | Select-Object -Unique).Count -eq 3) 'multiple locations remain distinct'
    Assert ($inventory.coverage[0].status -eq 'PARTIAL') 'known roots are explicitly incomplete'
    Assert (-not ($inventory | ConvertTo-Json -Depth 8 -Compress).Contains($testRoot.Replace('\','\\'))) 'raw private runtime paths do not leave adapter'
    foreach($file in @('versions/v22/node32.exe','versions/v22/node64.exe','nvs/24.0.0/x64/node.exe','nvs/24.0.0/arm64/node.exe','fnm/v24/installation/node.exe')){
        $target=Join-Path $testRoot $file
        [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($target))
        [IO.File]::WriteAllText($target,'not executable; never invoke')
    }
    $managers=@($roots[1],[pscustomobject]@{path=(Join-Path $testRoot 'nvs');layout='nvs';scope='user';hint='nvs'},[pscustomobject]@{path=(Join-Path $testRoot 'fnm');layout='fnm';scope='user';hint='fnm'})
    $managed=Get-NodeInventory $managers $schedule
    Assert (@($managed.records).Count -eq 7) 'inactive NVM architectures, nested NVS architectures and fnm installation directories must be read'
    Assert (@($managed.records.source_id | Select-Object -Unique).Count -eq 7) 'manager architectures keep distinct identities'
    $mixed=[pscustomobject]@{path=$roots[0].path.Replace('\','/');layout='direct';scope='unknown';hint='path'}
    Assert (@((Get-NodeInventory @($roots[0],$mixed) $schedule).records).Count -eq 1) 'same mixed-separator location deduplicates'
    $bad=[pscustomobject]@{path=(Join-Path $testRoot 'direct/node.exe');layout='versions';scope='unknown';hint='nvm'}
    $isolated=Get-NodeInventory @($bad,$roots[0]) $schedule
    Assert (@($isolated.records).Count -eq 1 -and $isolated.coverage[0].codes -contains 'NODE_ROOT_UNAVAILABLE') 'failed manager root preserves healthy runtime'
    $network=[pscustomobject]@{path='\\invalid.example\never-access';layout='direct';scope='unknown';hint='path'}
    Assert (@((Get-NodeInventory @($network,$roots[0]) $schedule).records).Count -eq 1) 'UNC input is rejected while local collection continues'
    $oldCancel=$env:SAFEPATCH_CANCEL_FILE
    try{
        $env:SAFEPATCH_CANCEL_FILE=Join-Path $testRoot 'owned-cancel'
        [IO.File]::WriteAllText($env:SAFEPATCH_CANCEL_FILE,'cancel')
        $cancelled=Get-NodeInventory $roots $schedule
        Assert (@($cancelled.records).Count -eq 0 -and $cancelled.coverage[0].codes -contains 'CANCELLED') 'owned cancellation stops metadata collection'
    }finally{$env:SAFEPATCH_CANCEL_FILE=$oldCancel}
}finally{
    $resolved=[IO.Path]::GetFullPath($testRoot)
    if($resolved.StartsWith($tempBase,[StringComparison]::OrdinalIgnoreCase) -and [IO.Directory]::Exists($resolved)){Remove-Item -LiteralPath $resolved -Recurse -Force}
}
Write-Output 'Node metadata assertions: PASS (29). Authored metadata/files; not device acceptance.'
