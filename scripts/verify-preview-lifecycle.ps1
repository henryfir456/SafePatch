# Owned clean CI only; never a user-machine cleanup script.
param([Parameter(Mandatory=$true)][string]$StagePath)
$ErrorActionPreference='Stop'
if($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted'){throw 'PREVIEW_CI_ONLY'}
$report=@{status='NOT_VERIFIED';host='GitHub Windows Server';user_windows_10_11='NOT_VERIFIED';token='restricted same-user token';is_admin=$null;exe_install='NOT_VERIFIED';msi_install='NOT_VERIFIED';payload_identity='NOT_VERIFIED';launch='NOT_VERIFIED';windows_scan='NOT_VERIFIED';cancel='NOT_VERIFIED';npm_scan='NOT_VERIFIED';restart='NOT_VERIFIED';uninstall_retention='NOT_VERIFIED';code='UNREACHED'}
$uiaEstablished=$false
. (Join-Path $env:GITHUB_WORKSPACE 'scripts\preview-contracts.ps1')
$historyHash=$null
$exeApp=$null;$msiApp=$null;$exeInstalled=$false;$msiInstalled=$false;$sentinel=$null
$exeDir=Join-Path $StagePath 'owned-exe-install';$msiDir=Join-Path $StagePath 'owned-msi-install'
$dataRoot=Join-Path $env:LOCALAPPDATA 'io.github.henryfir456.safepatch';$db=Join-Path $dataRoot 'inventory.sqlite'
function Wait-OwnProcess($p){if(-not $p.WaitForExit(120000)){$p.Kill();$p.WaitForExit();throw 'PREVIEW_INSTALL_TIMEOUT'};if($p.ExitCode -ne 0){throw 'PREVIEW_INSTALL_FAILED'}}
function Read-Stats { $raw=& $env:PREVIEW_NODE (Join-Path $env:GITHUB_WORKSPACE 'scripts\preview-db.mjs') $db 2>$null;if($LASTEXITCODE -ne 0){throw 'PREVIEW_DATABASE_CHECK_FAILED'};return ($raw|ConvertFrom-Json) }
function Wait-Stats($scope,$statuses){$end=[DateTime]::UtcNow.AddSeconds(210);do{$s=Read-Stats;if($s.$scope -and $statuses -contains $s.$scope.status){return $s};Start-Sleep -Milliseconds 150}while([DateTime]::UtcNow -lt $end);throw 'PREVIEW_SCAN_TIMEOUT'}
function App-Root($p){$end=[DateTime]::UtcNow.AddSeconds(30);do{$p.Refresh();if($p.HasExited){throw 'PREVIEW_APP_EXITED'};if($p.MainWindowHandle -ne [IntPtr]::Zero){return [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)};Start-Sleep -Milliseconds 250}while([DateTime]::UtcNow -lt $end);throw 'PREVIEW_WINDOW_UNAVAILABLE'}
function Control($root,$name){$condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,$name);return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)}
function Click($root,$name){$end=[DateTime]::UtcNow.AddSeconds(20);do{$c=Control $root $name;if($c -and $c.Current.IsEnabled){$pattern=$c.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern);$pattern.Invoke();return};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $end);throw 'PREVIEW_UIA_CONTROL_UNAVAILABLE'}
function Close-Own($p){if($null -eq $p){return};$p.Refresh();if(-not $p.HasExited){$null=$p.CloseMainWindow();if(-not $p.WaitForExit(10000)){$p.Kill();$p.WaitForExit()}}}
try {
 $identity=[Security.Principal.WindowsIdentity]::GetCurrent();$principal=New-Object Security.Principal.WindowsPrincipal($identity)
 $report.is_admin=$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
 if($report.is_admin){$report.code='TOKEN_STILL_ADMIN';return}
 if((Test-Path -LiteralPath $dataRoot) -or (Test-Path -LiteralPath $exeDir) -or (Test-Path -LiteralPath $msiDir)){$report.code='PREEXISTING_DATA_REFUSED';return}
 $manifest=Get-Content -LiteralPath (Join-Path $StagePath 'manifest.json') -Raw|ConvertFrom-Json
 $setup=Get-ChildItem -LiteralPath $StagePath -Filter '*-setup.exe';$msi=Get-ChildItem -LiteralPath $StagePath -Filter '*.msi'
 if(@($setup).Count -ne 1 -or @($msi).Count -ne 1){throw 'PREVIEW_PACKAGE_COUNT'}
 $p=Start-Process -FilePath $setup.FullName -ArgumentList @('/S',("/D="+$exeDir)) -PassThru;Wait-OwnProcess $p;$exeInstalled=$true;$report.exe_install='PASS'
 $msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
 $args='/i "'+$msi.FullName+'" /qn /norestart INSTALLDIR="'+$msiDir+'"'
 $p=Start-Process -FilePath $msiexec -ArgumentList $args -PassThru;Wait-OwnProcess $p;$msiInstalled=$true;$report.msi_install='PASS'
 foreach($dir in @($exeDir,$msiDir)){if((Get-FileHash -LiteralPath (Join-Path $dir 'safepatch.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.production_sha256){throw 'PREVIEW_PAYLOAD_MISMATCH'}}
 $report.payload_identity='PASS'
 # A command-line ALLUSERS override must be refused by the compiled MSI.
 $p=Start-Process -FilePath $msiexec -ArgumentList ('/i "'+$msi.FullName+'" /qn /norestart ALLUSERS=1') -PassThru
 if(-not $p.WaitForExit(120000)){$p.Kill();throw 'PREVIEW_OVERRIDE_TIMEOUT'}
 if($p.ExitCode -ne 1603){throw 'PREVIEW_ALLUSERS_OVERRIDE_INDETERMINATE'};$report.allusers_override='REFUSED'
 $null=New-Item -ItemType Directory -Path $dataRoot
 $sentinel=Join-Path $dataRoot 'owned-ci-retention-sentinel';[IO.File]::WriteAllText($sentinel,[Guid]::NewGuid().ToString());$sentinelHash=(Get-FileHash -LiteralPath $sentinel).Hash
 $runtime=$false
 foreach($key in @('HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}','HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}','HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}')){if((Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue).pv){$runtime=$true}}
 $report.webview2_registered=$runtime
 if(-not $runtime){$report.code='WEBVIEW2_PREREQUISITE_MISSING';return}
 Add-Type -AssemblyName UIAutomationClient
 Add-Type -AssemblyName UIAutomationTypes
 $exeApp=Start-Process -FilePath (Join-Path $exeDir 'safepatch.exe') -PassThru
 $root=App-Root $exeApp;$end=[DateTime]::UtcNow.AddSeconds(15)
 while(-not(Test-Path -LiteralPath $db)){if([DateTime]::UtcNow -gt $end){throw 'PREVIEW_DATABASE_NOT_CREATED'};Start-Sleep -Milliseconds 100}
 $report.launch='PASS'
 try {
  $end=[DateTime]::UtcNow.AddSeconds(25);while(-not(Control $root '開始唯讀掃描')){if([DateTime]::UtcNow -gt $end){throw 'PREVIEW_UIA_PROVIDER_UNAVAILABLE'};Start-Sleep -Milliseconds 100}
  $uiaEstablished=$true
  Click $root '開始唯讀掃描';$s=Wait-Stats 'windows' @('COMPLETE','PARTIAL')
  if($s.schema -ne 3 -or $s.fk_errors -ne 0 -or $s.assets -le 0){throw 'PREVIEW_WINDOWS_SNAPSHOT_INVALID'}
  $report.windows_scan='PASS';$report.assets=$s.assets;$scanId=$s.windows.id
  Click $root '開始唯讀掃描';Click $root '取消掃描';$cancel=Wait-Stats 'windows' @('CANCELLED')
  $report.cancel='PASS'
  Click $root '完整軟體清單'
  # Npm declaration inventory uses this public, owned checkout's existing lockfile.
  $edit=Control $root 'npm project directory'
  if(-not $edit){throw 'PREVIEW_UIA_CONTROL_UNAVAILABLE'}
  $value=$edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern);$value.SetValue($env:GITHUB_WORKSPACE)
  $fingerprints=@{};foreach($name in @('package.json','package-lock.json','npm-shrinkwrap.json')){$file=Join-Path $env:GITHUB_WORKSPACE $name;if(Test-Path -LiteralPath $file){$fingerprints[$name]=(Get-FileHash -LiteralPath $file).Hash}}
  Click $root '開始 npm 唯讀掃描';$n=Wait-Stats 'npm' @('PARTIAL')
  if($n.packages -le 0 -or $n.fk_errors -ne 0){throw 'PREVIEW_NPM_SNAPSHOT_INVALID'}
  foreach($name in $fingerprints.Keys){if((Get-FileHash -LiteralPath (Join-Path $env:GITHUB_WORKSPACE $name)).Hash -ne $fingerprints[$name]){throw 'PREVIEW_INPUT_CHANGED'}}
  $report.npm_scan='PASS';$report.packages=$n.packages;$report.edges=$n.edges;$npmId=$n.npm.id
  Close-Own $exeApp;$exeApp=$null
  $msiApp=Start-Process -FilePath (Join-Path $msiDir 'safepatch.exe') -PassThru;$root=App-Root $msiApp
  Click $root '完整軟體清單'
  $end=[DateTime]::UtcNow.AddSeconds(25);while(-not(Control $root 'npm: PARTIAL')){if([DateTime]::UtcNow -gt $end){throw 'PREVIEW_UIA_RESTORED_RESULT_UNAVAILABLE'};Start-Sleep -Milliseconds 100}
  $reopened=Read-Stats
  if($reopened.npm.id -ne $npmId -or $reopened.windows.status -ne 'CANCELLED' -or $reopened.packages -ne $n.packages){throw 'PREVIEW_RESTART_MISMATCH'}
  $report.restart='PASS';$report.status='PASS';$report.code=$null
 }catch {
  if((Get-PreviewUiFailureStatus $uiaEstablished $_.Exception.Message) -eq 'NOT_VERIFIED'){$report.code='UIA_UNAVAILABLE';$report.status='NOT_VERIFIED'}else{throw}
 }
} catch {$report.status='FAILED';$code=$_.Exception.Message;$report.code=if($code -match '^PREVIEW_[A-Z_]+$'){$code}else{'PREVIEW_LIFECYCLE_FAILED'}}
finally {
 Close-Own $exeApp;Close-Own $msiApp
 try {
  if(Test-Path -LiteralPath $db){$historyHash=(Get-FileHash -LiteralPath $db).Hash}
  if($exeInstalled){$p=Start-Process -FilePath (Join-Path $exeDir 'uninstall.exe') -ArgumentList ('/S _?='+$exeDir) -PassThru;Wait-OwnProcess $p}
  if($msiInstalled){
   if(-not(Test-Path -LiteralPath (Join-Path $msiDir 'safepatch.exe'))){throw 'PREVIEW_CROSS_FORMAT_REMOVAL'}
   if((Get-FileHash -LiteralPath (Join-Path $msiDir 'safepatch.exe')).Hash.ToLowerInvariant() -ne $manifest.production_sha256){throw 'PREVIEW_CROSS_FORMAT_REMOVAL'}
   $p=Start-Process -FilePath $msiexec -ArgumentList ('/x "'+$msi.FullName+'" /qn /norestart') -PassThru;Wait-OwnProcess $p
  }
  if($historyHash -and (-not(Test-Path -LiteralPath $db) -or (Get-FileHash -LiteralPath $db).Hash -ne $historyHash)){throw 'PREVIEW_HISTORY_REMOVED'}
  if($historyHash){$report.history_retention='PASS'}
  if($sentinel -and (Test-Path -LiteralPath $sentinel) -and (Get-FileHash -LiteralPath $sentinel).Hash -eq $sentinelHash){$report.uninstall_retention='PASS'}elseif($sentinel){throw 'PREVIEW_DATA_REMOVED'}
 }catch{$report.status='FAILED';$report.code='PREVIEW_UNINSTALL_OR_RETENTION_FAILED'}
 $report|ConvertTo-Json -Depth 4|Set-Content -LiteralPath (Join-Path $StagePath 'ci-lifecycle.json') -Encoding utf8
 # Deliberately do not remove application data, even in this disposable CI.
}
