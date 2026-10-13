# Fixed, read-only update query. Dot-source after windows_inventory functions.
function Convert-WinGetBytes([byte[]]$Bytes) {
    if($Bytes.Length -gt 4194304){throw 'WINGET_UPDATE_LIMIT'}
    try {
        if($Bytes.Length -ge 2 -and $Bytes[0] -eq 255 -and $Bytes[1] -eq 254){return [Text.UnicodeEncoding]::new($false,$true,$true).GetString($Bytes,2,$Bytes.Length-2)}
        if($Bytes.Length -ge 2 -and $Bytes[0] -eq 254 -and $Bytes[1] -eq 255){return [Text.UnicodeEncoding]::new($true,$true,$true).GetString($Bytes,2,$Bytes.Length-2)}
        return [Text.UTF8Encoding]::new($false,$true).GetString($Bytes).TrimStart([char]0xfeff)
    } catch {throw 'WINGET_UPDATE_ENCODING'}
}
function Convert-WinGetUpdateTable([string]$Text, $Records) {
    $ids=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach($r in $Records){if($r.catalog_source -ceq 'winget'){[void]$ids.Add($r.normalized_id)}}
    $rows=[Collections.Generic.List[object]]::new()
    $inTable=$false; $sawTable=$false
    # Headers, names, footer prose are deliberately never interpreted.
    $plain=[regex]::Replace($Text,'\x1b\[[0-?]*[ -/]*[@-~]','')
    foreach($line in ($plain -split '\r?\n')) {
        if($line.Trim() -match '^-{5,}$'){$inTable=$true;$sawTable=$true;continue}
        if(-not $inTable){continue}
        if([string]::IsNullOrWhiteSpace($line)){continue}
        $parts=@($line.Trim() -split '\s+')
        if($parts.Length -gt 0 -and $parts[-1] -ceq 'winget'){$parts=@($parts[0..($parts.Length-2)])}
        $positions=[Collections.Generic.List[int]]::new();$structural=[Collections.Generic.List[int]]::new()
        for($i=1;$i -lt $parts.Length;$i++){
            if(($parts.Length-$i -eq 2 -or $parts.Length-$i -eq 3) -and $parts[$i] -match '^[A-Za-z0-9][A-Za-z0-9._+-]*\.[A-Za-z0-9._+-]+(?:\u2026)?$' -and $parts[$i] -match '[A-Za-z]'){$structural.Add($i);if($ids.Contains($parts[$i])){$positions.Add($i)}}
        }
        if($positions.Count -ne 1){
            # Fixed-width data rows have padding; reject changed/truncated IDs.
            if($structural.Count -ge 1 -and $positions.Count -eq 0){continue}
            if($line -match '^\d+\s+[^\s]+(?: [^\s]+)*$'){continue}
            throw 'WINGET_UPDATE_FORMAT'
        }
        $at=$positions[0]
        $current=$parts[$at+1]
        $target=if($parts.Length-$at -eq 3){$parts[$at+2]}else{$null}
        if($current.Length -gt 128 -or ($null -ne $target -and $target.Length -gt 128)){throw 'WINGET_UPDATE_FORMAT'}
        $rows.Add([pscustomobject]@{display_name=($parts[0..($at-1)] -join ' ');package_id=$parts[$at];installed_version=$current;target_version=$target})
        if($rows.Count -gt 20000){throw 'WINGET_UPDATE_LIMIT'}
    }
    if(-not $sawTable -or $rows.Count -eq 0){throw 'WINGET_UPDATE_FORMAT'}
    return @($rows)
}
function Invoke-WinGetUpdateProcess([string]$Executable,[int]$TimeoutMilliseconds=45000) {
    $process=[Diagnostics.Process]::new();$started=$false;$outFile=$null;$errFile=$null;$output=$null;$errors=$null
    try {
        $outFile=Join-Path ([IO.Path]::GetTempPath()) ('safepatch-update-'+[guid]::NewGuid().ToString('N'))
        $errFile=$outFile+'-err'
        $output=[IO.File]::Open($outFile,[IO.FileMode]::CreateNew,[IO.FileAccess]::ReadWrite,[IO.FileShare]::Read)
        $errors=[IO.File]::Open($errFile,[IO.FileMode]::CreateNew,[IO.FileAccess]::ReadWrite,[IO.FileShare]::Read)
        $info=[Diagnostics.ProcessStartInfo]::new();$info.FileName=$Executable
        # list emits available upgrades using WinGet's version/pin evaluator; never installs.
        $info.Arguments='list --source winget --disable-interactivity'
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
function Get-WinGetUpdates([string]$Executable,$Records) {
    try {
        $rows=@(Convert-WinGetUpdateTable (Invoke-WinGetUpdateProcess $Executable) $Records)
        Set-WinGetUpdateEvidence $Records $rows $null
        return [pscustomobject]@{scanner='winget-updates';status='PARTIAL';count=@($Records|Where-Object {$_.update.status -eq 'UPDATE_AVAILABLE'}).Count;codes=@('CATALOG_MAY_BE_CACHED','PIN_AND_VERSION_COVERAGE_LIMITED')}
    } catch {
        $code=[string]$_.Exception.Message
        if($code -notmatch '^(WINGET_UPDATE_[A-Z_]+|CANCELLED)$'){$code='WINGET_UPDATE_QUERY_FAILED'}
        Set-WinGetUpdateEvidence $Records @() $code
        return [pscustomobject]@{scanner='winget-updates';status='UNKNOWN';count=0;codes=@($code)}
    }
}
