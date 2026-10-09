# Product-owned, fixed adapter. Never invoke discovered application binaries.
# stdout is private inventory JSON; errors are stable codes, never raw exception text.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-PathHash([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Value.Trim().ToLowerInvariant())))).Replace('-','').ToLowerInvariant() }
    finally { $sha.Dispose() }
}
function Get-CleanText($Value) {
    if ($null -eq $Value -or $Value -isnot [string]) { return $null }
    $text = [regex]::Replace($Value, '[\x00-\x1f\x7f]', '').Trim()
    if ($text.Length -gt 1024) { $text = $text.Substring(0,1024) }
    if ($text.Length -eq 0) { return $null }
    return $text
}
function Test-Cancelled {
    return ($env:SAFEPATCH_CANCEL_FILE -and [IO.File]::Exists($env:SAFEPATCH_CANCEL_FILE))
}
function Get-RegistryInventory {
    $records = [Collections.Generic.List[object]]::new()
    $coverage = [Collections.Generic.List[object]]::new()
    $timer = [Diagnostics.Stopwatch]::StartNew()
    foreach ($hive in @('LocalMachine','CurrentUser')) {
        foreach ($view in @('Registry64','Registry32')) {
            $codes = [Collections.Generic.HashSet[string]]::new()
            $count = 0; $base = $null; $key = $null
            try {
                $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::$hive,[Microsoft.Win32.RegistryView]::$view)
                $key = $base.OpenSubKey('SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',$false)
                if ($null -eq $key) { [void]$codes.Add('REGISTRY_KEY_ABSENT') }
                else {
                    foreach ($name in $key.GetSubKeyNames()) {
                        if (Test-Cancelled) { [void]$codes.Add('CANCELLED'); break }
                        if ($timer.Elapsed.TotalSeconds -gt 30 -or $records.Count -ge 20000) { [void]$codes.Add('REGISTRY_LIMIT'); break }
                        $entry = $null
                        try {
                            $entry = $key.OpenSubKey($name,$false)
                            if ($null -eq $entry) { [void]$codes.Add('REGISTRY_ENTRY_UNAVAILABLE'); continue }
                            $display = Get-CleanText ($entry.GetValue('DisplayName',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames))
                            if ($null -eq $display) { continue }
                            $version = Get-CleanText ($entry.GetValue('DisplayVersion',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames))
                            $publisher = Get-CleanText ($entry.GetValue('Publisher',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames))
                            $location = Get-CleanText ($entry.GetValue('InstallLocation',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames))
                            $packageId = Get-CleanText ($entry.GetValue('WinGetPackageIdentifier',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames))
                            $scope = if($hive -eq 'LocalMachine'){'machine'}else{'user'}
                            $records.Add([pscustomobject]@{display_name=$display;publisher=$publisher;installed_version=$version;normalized_id=$packageId;install_path_hash=(Get-PathHash $location);installation_source='UNKNOWN';scanner='registry';source_id=(Get-PathHash ($hive+'\'+$name));scope=$scope;view=$view;catalog_source=$null})
                            $count++
                        } catch { [void]$codes.Add('REGISTRY_ENTRY_READ_FAILED') }
                        finally { if($null -ne $entry){$entry.Dispose()} }
                    }
                }
            } catch { [void]$codes.Add('REGISTRY_VIEW_READ_FAILED') }
            finally { if($null -ne $key){$key.Dispose()}; if($null -ne $base){$base.Dispose()} }
            $status = if($codes.Count -eq 0){'COMPLETE'}elseif($count -gt 0){'PARTIAL'}else{'UNKNOWN'}
            $coverage.Add([pscustomobject]@{scanner=('registry/'+$hive+'/'+$view);status=$status;count=$count;codes=@($codes)})
        }
    }
    return [pscustomobject]@{records=@($records);coverage=@($coverage)}
}
function Convert-WinGetDocument($Document) {
    $records = [Collections.Generic.List[object]]::new()
    if ($null -eq $Document -or $null -eq $Document.PSObject.Properties['Sources']) { throw 'WINGET_INVALID_JSON' }
    foreach ($source in @($Document.Sources)) {
        if ($null -eq $source.PSObject.Properties['SourceDetails'] -or $null -eq $source.PSObject.Properties['Packages']) { throw 'WINGET_INVALID_JSON' }
        $sourceName = Get-CleanText $source.SourceDetails.Name
        foreach ($package in @($source.Packages)) {
            if ($records.Count -ge 20000) { throw 'WINGET_LIMIT' }
            $id = Get-CleanText $package.PackageIdentifier
            if ($null -eq $id) { throw 'WINGET_INVALID_ID' }
            $version = if($null -ne $package.PSObject.Properties['Version']){Get-CleanText $package.Version}else{$null}
            if ($version -eq 'Unknown') { $version = $null }
            $records.Add([pscustomobject]@{display_name=$id;publisher=$null;installed_version=$version;normalized_id=$id;install_path_hash=$null;installation_source='UNKNOWN';scanner='winget';source_id=(Get-PathHash ($sourceName+'\'+$id));scope='unknown';view='catalog';catalog_source=$sourceName})
        }
    }
    return @($records)
}
function Get-WinGetInventory {
    $process = $null; $temp = $null
    try {
        # Only the registered Microsoft App Installer and valid Microsoft signature.
        $packages = @(Get-AppxPackage -Name Microsoft.DesktopAppInstaller -ErrorAction Stop | Where-Object { $_.Publisher -match '^CN=Microsoft Corporation,' } | Sort-Object Version -Descending)
        if ($packages.Count -eq 0) { throw 'WINGET_UNAVAILABLE' }
        $exe = Join-Path $packages[0].InstallLocation 'winget.exe'
        $signature = Get-AuthenticodeSignature -LiteralPath $exe
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '^CN=Microsoft Corporation,') { throw 'WINGET_UNTRUSTED' }
        # WinGet list has no stable JSON mode. Export provides catalog ID/version only.
        # No source agreements are accepted, no install/upgrade command is invoked.
        $temp = Join-Path ([IO.Path]::GetTempPath()) ('safepatch-'+[guid]::NewGuid().ToString('N')+'.json')
        $info = [Diagnostics.ProcessStartInfo]::new()
        $info.FileName=$exe
        $info.Arguments='export --output "'+$temp+'" --include-versions --source winget --disable-interactivity'
        $info.UseShellExecute=$false; $info.CreateNoWindow=$true
        $info.RedirectStandardOutput=$true; $info.RedirectStandardError=$true
        $process=[Diagnostics.Process]::new();$process.StartInfo=$info
        [void]$process.Start()
        # Drain streams concurrently; retain no output that could disclose inventory.
        $outTask=$process.StandardOutput.BaseStream.CopyToAsync([IO.Stream]::Null)
        $errTask=$process.StandardError.BaseStream.CopyToAsync([IO.Stream]::Null)
        $timer=[Diagnostics.Stopwatch]::StartNew()
        while (-not $process.WaitForExit(100)) {
            if(Test-Cancelled){$process.Kill();$process.WaitForExit();throw 'CANCELLED'}
            if($timer.Elapsed.TotalSeconds -gt 45){$process.Kill();$process.WaitForExit();throw 'WINGET_TIMEOUT'}
        }
        [void]$outTask.GetAwaiter().GetResult();[void]$errTask.GetAwaiter().GetResult()
        if($process.ExitCode -ne 0){throw 'WINGET_COMMAND_FAILED'}
        if(-not [IO.File]::Exists($temp) -or ([IO.FileInfo]$temp).Length -gt 4194304){throw 'WINGET_INVALID_OUTPUT'}
        $document=[IO.File]::ReadAllText($temp) | ConvertFrom-Json
        $records=@(Convert-WinGetDocument $document)
        return [pscustomobject]@{records=$records;coverage=@([pscustomobject]@{scanner='winget';status='PARTIAL';count=$records.Count;codes=@('CATALOG_MATCHES_ONLY','PUBLISHER_NOT_EXPORTED','NON_WINGET_SOURCES_NOT_SCANNED')})}
    } catch {
        $code=[string]$_.Exception.Message
        if($code -notmatch '^(WINGET_[A-Z_]+|CANCELLED)$'){$code='WINGET_READ_FAILED'}
        return [pscustomobject]@{records=@();coverage=@([pscustomobject]@{scanner='winget';status='UNKNOWN';count=0;codes=@($code)})}
    } finally {
        if($null -ne $process){if(-not $process.HasExited){$process.Kill();$process.WaitForExit()};$process.Dispose()}
        if($null -ne $temp -and [IO.File]::Exists($temp)){[IO.File]::Delete($temp)}
    }
}
function Invoke-WindowsInventory {
    [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
    $registry=Get-RegistryInventory
    $winget=if(Test-Cancelled){[pscustomobject]@{records=@();coverage=@([pscustomobject]@{scanner='winget';status='UNKNOWN';count=0;codes=@('CANCELLED')})}}else{Get-WinGetInventory}
    [pscustomobject]@{records=@($registry.records)+@($winget.records);coverage=@($registry.coverage)+@($winget.coverage)} | ConvertTo-Json -Depth 8 -Compress
}
if($MyInvocation.InvocationName -ne '.') { Invoke-WindowsInventory }
