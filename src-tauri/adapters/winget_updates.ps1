function Test-WinGetComIdentity([string]$PackageName,$IndexedPackages,[string]$ServerExecutable,$SystemExecutable,[bool]$ClassOverride) {
    return (-not $ClassOverride -and $PackageName -cmatch '^Microsoft\.DesktopAppInstaller_[0-9.]+_x64__8wekyb3d8bbwe$' -and @($IndexedPackages).Count -eq 1 -and @($IndexedPackages)[0] -ceq $PackageName -and $ServerExecutable -ceq 'WindowsPackageManagerServer.exe' -and $SystemExecutable -is [int] -and $SystemExecutable -eq 0)
}
function Confirm-WinGetComRegistration($Package) {
    $env:SAFEPATCH_WINGET_COM_VERIFIED=$null
    $fullName=[string]$Package.PackageFullName
    $server=Join-Path $Package.InstallLocation 'WindowsPackageManagerServer.exe'
    $signature=Get-AuthenticodeSignature -LiteralPath $server
    if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '^CN=Microsoft Corporation,'){throw 'WINGET_UPDATE_COM_UNTRUSTED'}
    foreach($id in @('{C53A4F16-787E-42A4-B304-29EFFB4BF597}','{526534B8-7E46-47C8-8416-B1685C327D37}','{572DED96-9C60-4526-8F92-EE7D91D38C1A}')){
        $base=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine,[Microsoft.Win32.RegistryView]::Registry64)
        $user=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,[Microsoft.Win32.RegistryView]::Registry64)
        $index=$null;$class=$null;$entry=$null;$override=$null;$userIndex=$null;$userClass=$null;$userServer=$null
        try {
            $override=$user.OpenSubKey(('SOFTWARE\Classes\CLSID\'+$id),$false)
            $userIndex=$user.OpenSubKey(('SOFTWARE\Classes\PackagedCom\ClassIndex\'+$id),$false)
            if($null -ne $userIndex -and (@($userIndex.GetSubKeyNames()).Count -ne 1 -or @($userIndex.GetSubKeyNames())[0] -cne $fullName)){throw 'WINGET_UPDATE_COM_UNTRUSTED'}
            $userClass=$user.OpenSubKey(('SOFTWARE\Classes\PackagedCom\Package\'+$fullName+'\Class'),$false)
            $userServer=$user.OpenSubKey(('SOFTWARE\Classes\PackagedCom\Package\'+$fullName+'\Server'),$false)
            if($null -ne $userClass -or $null -ne $userServer){throw 'WINGET_UPDATE_COM_UNTRUSTED'}
            $index=$base.OpenSubKey(('SOFTWARE\Classes\PackagedCom\ClassIndex\'+$id),$false)
            if($null -eq $index){throw 'WINGET_UPDATE_COM_UNTRUSTED'}
            $class=$base.OpenSubKey(('SOFTWARE\Classes\PackagedCom\Package\'+$fullName+'\Class\'+$id),$false)
            if($null -eq $class){throw 'WINGET_UPDATE_COM_UNTRUSTED'}
            $serverId=$class.GetValue('ServerId')
            if($serverId -isnot [int] -or $serverId -lt 0){throw 'WINGET_UPDATE_COM_UNTRUSTED'}
            $entry=$base.OpenSubKey(('SOFTWARE\Classes\PackagedCom\Package\'+$fullName+'\Server\'+$serverId),$false)
            if($null -eq $entry -or -not (Test-WinGetComIdentity $fullName @($index.GetSubKeyNames()) ([string]$entry.GetValue('Executable')) $entry.GetValue('IsSystemExecutable') ($null -ne $override)) -or $entry.GetValue('ApplicationId') -cne 'WinGetComServer'){throw 'WINGET_UPDATE_COM_UNTRUSTED'}
        } finally {foreach($key in @($userServer,$userClass,$userIndex,$override,$entry,$class,$index,$user,$base)){if($null -ne $key){$key.Dispose()}}}
    }
    # Process-only marker set exclusively after signed CLI/server + packaged registration checks.
    $env:SAFEPATCH_WINGET_COM_VERIFIED='1'
}
# Fixed, read-only update query. Dot-source after windows_inventory functions.
function Convert-WinGetBytes([byte[]]$Bytes) {
    if($Bytes.Length -gt 4194304){throw 'WINGET_UPDATE_LIMIT'}
    try {
        if($Bytes.Length -ge 2 -and $Bytes[0] -eq 255 -and $Bytes[1] -eq 254){return [Text.UnicodeEncoding]::new($false,$true,$true).GetString($Bytes,2,$Bytes.Length-2)}
        if($Bytes.Length -ge 2 -and $Bytes[0] -eq 254 -and $Bytes[1] -eq 255){return [Text.UnicodeEncoding]::new($true,$true,$true).GetString($Bytes,2,$Bytes.Length-2)}
        return [Text.UTF8Encoding]::new($false,$true).GetString($Bytes).TrimStart([char]0xfeff)
    } catch {throw 'WINGET_UPDATE_ENCODING'}
}
function Convert-WinGetUpdateDocument($Document) {
    if($null -eq $Document -or $null -eq $Document.PSObject.Properties['schema'] -or $Document.schema -ne 1 -or $null -eq $Document.PSObject.Properties['status'] -or $null -eq $Document.PSObject.Properties['rows']){throw 'WINGET_UPDATE_FORMAT'}
    if($Document.status -ceq 'ERROR'){
        $code=if($null -ne $Document.PSObject.Properties['code']){[string]$Document.code}else{''}
        if($code -notmatch '^WINGET_UPDATE_[A-Z_]+$'){$code='WINGET_UPDATE_QUERY_FAILED'}
        throw $code
    }
    if($Document.status -cne 'COMPLETE' -or $Document.rows -isnot [array] -or $Document.rows.Count -gt 20000){throw 'WINGET_UPDATE_FORMAT'}
    foreach($row in $Document.rows){
        foreach($field in @('display_name','package_id','installed_version','target_version','update_available')){if($null -eq $row.PSObject.Properties[$field]){throw 'WINGET_UPDATE_FORMAT'}}
        if($row.package_id -isnot [string] -or $row.package_id -notmatch '^[A-Za-z0-9][A-Za-z0-9._+-]{0,255}$' -or $row.installed_version -isnot [string] -or $row.installed_version.Length -gt 128 -or $row.display_name -isnot [string] -or $row.display_name.Length -gt 1024 -or $row.update_available -isnot [bool]){throw 'WINGET_UPDATE_FORMAT'}
        if($row.update_available -and ($row.target_version -isnot [string] -or [string]::IsNullOrWhiteSpace($row.target_version) -or $row.target_version.Length -gt 128)){throw 'WINGET_UPDATE_FORMAT'}
        if(-not $row.update_available -and $null -ne $row.target_version){throw 'WINGET_UPDATE_FORMAT'}
    }
    return @($Document.rows)
}
function Invoke-WinGetUpdateProcess([string]$Executable,[int]$TimeoutMilliseconds=45000) {
    $process=[Diagnostics.Process]::new();$started=$false;$outFile=$null;$errFile=$null;$output=$null;$errors=$null
    try {
        $outFile=Join-Path ([IO.Path]::GetTempPath()) ('safepatch-update-'+[guid]::NewGuid().ToString('N'))
        $errFile=$outFile+'-err'
        $output=[IO.File]::Open($outFile,[IO.FileMode]::CreateNew,[IO.FileAccess]::ReadWrite,[IO.FileShare]::Read)
        $errors=[IO.File]::Open($errFile,[IO.FileMode]::CreateNew,[IO.FileAccess]::ReadWrite,[IO.FileShare]::Read)
        $info=[Diagnostics.ProcessStartInfo]::new();$info.FileName=$Executable
        # Product-owned typed COM helper. Never execute discovered binary paths.
        $info.Arguments='query'
        $info.UseShellExecute=$false;$info.CreateNoWindow=$true;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
        $process.StartInfo=$info;[void]$process.Start();$started=$true
        $outTask=$process.StandardOutput.BaseStream.CopyToAsync($output)
        $errTask=$process.StandardError.BaseStream.CopyToAsync($errors)
        $timer=[Diagnostics.Stopwatch]::StartNew()
        while(-not $process.WaitForExit(100)) {
            if(Test-Cancelled){throw 'CANCELLED'}
            if($timer.ElapsedMilliseconds -gt $TimeoutMilliseconds){throw 'WINGET_UPDATE_TIMEOUT'}
            if($output.Length -gt 4194304 -or $errors.Length -gt 1048576){throw 'WINGET_UPDATE_LIMIT'}
        }
        [void]$outTask.GetAwaiter().GetResult();[void]$errTask.GetAwaiter().GetResult()
        if($process.ExitCode -ne 0){throw 'WINGET_UPDATE_COMMAND_FAILED'}
        if($errors.Length -gt 0){throw 'WINGET_UPDATE_WARNINGS'}
        $output.Dispose();$output=$null
        return Convert-WinGetBytes ([IO.File]::ReadAllBytes($outFile))
    } finally {
        if($started -and -not $process.HasExited){$process.Kill();$process.WaitForExit()}
        if($null -ne $output){$output.Dispose()};if($null -ne $errors){$errors.Dispose()}
        $process.Dispose()
        foreach($file in @($outFile,$errFile)){if($file -and [IO.File]::Exists($file)){[IO.File]::Delete($file)}}
    }
}
function Set-WinGetUpdateEvidence($Records,$Rows,[string]$Failure) {
    $checked=[DateTime]::UtcNow.ToString('o')
    foreach($record in $Records){
        $status='UNKNOWN';$target=$null;$reason='WINGET_UPDATE_ROW_ABSENT'
        if($Failure){$status='ERROR';$reason=$Failure}
        else {
            $same=@($Rows|Where-Object {$_.package_id -ceq $record.normalized_id -and $_.installed_version -ceq $record.installed_version})
            $targets=@($same|ForEach-Object {if($null -eq $_.target_version){'<none>'}else{$_.target_version}}|Select-Object -Unique)
            if($targets.Count -gt 1){$reason='WINGET_UPDATE_AMBIGUOUS'}
            elseif($same.Count -gt 0){$record.display_name=Get-CleanText $same[0].display_name;$target=$same[0].target_version;$status=if($target){'UPDATE_AVAILABLE'}else{'NO_UPDATE_REPORTED'};$reason='WINGET_EXACT_ID_VERSION'}
        }
        $evidence=[pscustomobject]@{status=$status;target_version=$target;package_id=$record.normalized_id;source_url='https://cdn.winget.microsoft.com/cache';checked_at=$checked;reason=$reason}
        $record|Add-Member -NotePropertyName update -NotePropertyValue $evidence -Force
    }
}
function Get-WinGetUpdates($Package,$Records) {
    try {
        Confirm-WinGetComRegistration $Package
        if(-not $env:SAFEPATCH_WINGET_HELPER -or -not [IO.File]::Exists($env:SAFEPATCH_WINGET_HELPER)){throw 'WINGET_UPDATE_HELPER_UNAVAILABLE'}
        $json=Invoke-WinGetUpdateProcess $env:SAFEPATCH_WINGET_HELPER
        try{$document=$json|ConvertFrom-Json}catch{throw 'WINGET_UPDATE_FORMAT'}
        $rows=@(Convert-WinGetUpdateDocument $document)
        Set-WinGetUpdateEvidence $Records $rows $null
        return [pscustomobject]@{scanner='winget-updates';status='PARTIAL';count=@($Records|Where-Object {$_.update.status -eq 'UPDATE_AVAILABLE'}).Count;codes=@('CATALOG_MAY_BE_CACHED','PIN_AND_VERSION_COVERAGE_LIMITED')}
    } catch {
        $code=[string]$_.Exception.Message
        if($code -notmatch '^(WINGET_UPDATE_[A-Z_]+|CANCELLED)$'){$code='WINGET_UPDATE_QUERY_FAILED'}
        Set-WinGetUpdateEvidence $Records @() $code
        return [pscustomobject]@{scanner='winget-updates';status='UNKNOWN';count=0;codes=@($code)}
    }
}
