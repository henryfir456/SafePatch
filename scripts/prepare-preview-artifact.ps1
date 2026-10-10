# Stage only production installers and safe aggregate metadata (#22).
param([switch]$Seal)
$ErrorActionPreference='Stop'
if($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted'){throw 'PREVIEW_CI_ONLY'}
$stage=Join-Path $env:RUNNER_TEMP ("safepatch-preview-"+$env:GITHUB_RUN_ID+"-"+$env:GITHUB_RUN_ATTEMPT)
if($Seal){
 $manifest=Get-Content -LiteralPath (Join-Path $stage 'manifest.json') -Raw|ConvertFrom-Json
 $report=Get-Content -LiteralPath (Join-Path $stage 'ci-lifecycle.json') -Raw|ConvertFrom-Json
 if($report.status -eq 'FAILED'){throw 'PREVIEW_LIFECYCLE_FAILED'}
 $manifest|Add-Member -NotePropertyName ci_lifecycle -NotePropertyValue $report -Force
 $manifest|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $stage 'manifest.json') -Encoding utf8
 $lines=Get-ChildItem -LiteralPath $stage -File|Where-Object Name -ne 'SHA256SUMS.txt'|Sort-Object Name|ForEach-Object{(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()+'  '+$_.Name}
 $lines|Set-Content -LiteralPath (Join-Path $stage 'SHA256SUMS.txt') -Encoding ascii
 Write-Output 'Preview artifact sealed; production-only payload and aggregate reports.'
 return
}
if(Test-Path -LiteralPath $stage){throw 'PREVIEW_STAGE_ALREADY_EXISTS'}
$null=New-Item -ItemType Directory -Path $stage
$conf=Get-Content src-tauri/tauri.conf.json -Raw|ConvertFrom-Json
$version=$conf.version
if($conf.identifier -ne 'io.github.henryfir456.safepatch' -or $conf.bundle.windows.webviewInstallMode.type -ne 'skip' -or $conf.bundle.windows.nsis.installMode -ne 'currentUser'){throw 'PREVIEW_POLICY_INVALID'}
$production=Resolve-Path 'src-tauri/target/release/safepatch.exe'
$hash=(Get-FileHash -LiteralPath $production.Path -Algorithm SHA256).Hash.ToLowerInvariant()
$files=@()
foreach($format in @('nsis','msi')){
 $extension=if($format -eq 'nsis'){'*.exe'}else{'*.msi'}
 $matches=@(Get-ChildItem -LiteralPath ('src-tauri/target/release/bundle/'+$format) -Filter $extension -File)
 if($matches.Count -ne 1){throw 'PREVIEW_BUNDLE_COUNT'}
 $name=if($format -eq 'nsis'){"SafePatch-Preview-$version-x64-setup.exe"}else{"SafePatch-Preview-$version-x64.msi"}
 Copy-Item -LiteralPath $matches[0].FullName -Destination (Join-Path $stage $name)
 $files+=@{name=$name;sha256=(Get-FileHash -LiteralPath (Join-Path $stage $name)).Hash.ToLowerInvariant();size_bytes=$matches[0].Length}
}
Copy-Item -LiteralPath $production.Path -Destination (Join-Path $stage 'safepatch.exe')
Copy-Item -LiteralPath 'docs/WINDOWS_PREVIEW.md' -Destination (Join-Path $stage 'README.md')
$commit=(& git rev-parse HEAD).Trim()
if($commit -notmatch '^[0-9a-f]{40}$'){throw 'PREVIEW_COMMIT_INVALID'}
@{schema=1;product='SafePatch Preview';version=$version;channel='Preview';architecture='x86_64';source_commit=$commit;ci_event=$env:GITHUB_EVENT_NAME;ci_run_id=$env:GITHUB_RUN_ID;ci_run_attempt=$env:GITHUB_RUN_ATTEMPT;host='GitHub Windows Server';os_build=[Environment]::OSVersion.Version.ToString();signature='UNSIGNED';production_sha256=$hash;installers=$files;features=@('Windows Registry/WinGet','VS Code/Extensions','Node resource metadata','npm lockfile declarations');user_windows_10_11='NOT_VERIFIED'}|ConvertTo-Json -Depth 6|Set-Content -LiteralPath (Join-Path $stage 'manifest.json') -Encoding utf8
# Read compiled MSI tables, not just source configuration.
$msi=Get-ChildItem -LiteralPath $stage -Filter '*.msi'
$installer=New-Object -ComObject WindowsInstaller.Installer
$db=$installer.OpenDatabase($msi.FullName,0)
function Rows($sql){$view=$db.OpenView($sql);$view.Execute();$result=@();while($record=$view.Fetch()){$row=@();for($i=1;$i -le $record.FieldCount;$i++){$row+=$record.StringData($i)};$result+=,@($row)};$view.Close();return ,$result}
$properties=Rows 'SELECT Property, Value FROM Property'
if(@($properties|Where-Object{$_[0] -eq 'ALLUSERS' -and $_[1]}).Count -ne 0){throw 'PREVIEW_MSI_ALLUSERS'}
$privileges=[int]$db.SummaryInformation(0).Property(15)
if(($privileges -band 8) -ne 8){throw 'PREVIEW_MSI_ELEVATION_REQUIRED'}
$conditions=Rows 'SELECT Condition FROM LaunchCondition'
if(-not($conditions|Where-Object{$_[0] -match 'NOT ALLUSERS'})){throw 'PREVIEW_MSI_OVERRIDE_UNGUARDED'}
$registry=Rows 'SELECT Root FROM Registry'
if(@($registry|Where-Object{$_[0] -ne '1'}).Count -ne 0){throw 'PREVIEW_MSI_MACHINE_REGISTRY'}
$actions=Rows 'SELECT Type FROM CustomAction'
if(@($actions|Where-Object{([int]$_[0] -band 63) -ne 51}).Count -ne 0){throw 'PREVIEW_MSI_EXECUTION_ACTION'}
# Extract both real compiled payloads using the runner's existing 7-Zip.
$seven=(Get-Command 7z.exe -ErrorAction Stop).Source
foreach($package in @((Get-ChildItem -LiteralPath $stage -Filter '*-setup.exe'),$msi)){
 $extract=Join-Path $env:RUNNER_TEMP ('preview-extract-'+[Guid]::NewGuid().ToString('N'))
 $null=New-Item -ItemType Directory -Path $extract
 & $seven x $package.FullName ("-o"+$extract) -y *> $null
 if($LASTEXITCODE -ne 0){throw 'PREVIEW_EXTRACTION_FAILED'}
 # MSI CAB extraction may leave an embedded cabinet; expand only in this owned directory.
 foreach($cab in @(Get-ChildItem -LiteralPath $extract -Recurse -Filter '*.cab' -File)){
  & $seven x $cab.FullName ("-o"+$extract) -y *> $null
  if($LASTEXITCODE -ne 0){throw 'PREVIEW_CAB_EXTRACTION_FAILED'}
 }
 $payloads=@(Get-ChildItem -LiteralPath $extract -Recurse -File|Where-Object{(Get-FileHash -LiteralPath $_.FullName).Hash.ToLowerInvariant() -eq $hash})
 if($payloads.Count -ne 1){throw 'PREVIEW_EXTRACTED_PAYLOAD_MISMATCH'}
}
"PREVIEW_STAGE=$stage"|Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
@{status='PASS';payload_sha256=$hash;compiled_msi_per_user=$true;compiled_msi_limited=$true;allusers_guard=$true;no_executable_custom_actions=$true;extracted_payloads_match=$true}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $stage 'ci-package-checks.json') -Encoding utf8
