# PE version resources are read as data. Never invoke a discovered node.exe.
function Get-NodeScheduleField($Document,[string]$Field) {
    if($null -eq $Document){return $null}
    if($Document -is [Collections.IDictionary]){if($Document.Contains($Field)){return $Document[$Field]};return $null}
    $property=$Document.PSObject.Properties[$Field]
    if($null -ne $property){return $property.Value}
    return $null
}
function Get-NodeScheduleDate($Document,[string]$Field) {
    $value=Get-NodeScheduleField $Document $Field
    if($null -eq $value){return $null}
    if($value -isnot [string]){throw 'NODE_SCHEDULE_INVALID'}
    return [DateTime]::ParseExact($value,'yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture)
}
function Get-NodeSupport($Version,$Schedule,[string]$Today=([DateTime]::UtcNow.ToString('yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture))) {
    $result=[pscustomobject]@{status='UNKNOWN';end_date=$null;source_url='https://github.com/nodejs/Release/blob/main/schedule.json';checked_at=$null;evaluated_at=$Today}
    try{
        $now=[DateTime]::ParseExact($Today,'yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture)
        $checked=Get-NodeScheduleDate $Schedule 'checked_at'
        if($null -eq $checked){return $result}
        $result.checked_at=$checked.ToString('yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture)
        if(($now-$checked).TotalDays -lt 0 -or ($now-$checked).TotalDays -gt 30){return $result}
        if($Version -isnot [string] -or $Version.Length -gt 256 -or $Version -cnotmatch '\Av?([0-9]+)\.[0-9]+\.[0-9]+(?:\+[0-9A-Za-z.-]+)?\z'){return $result}
        $majorKey='v'+$Matches[1]
        $release=Get-NodeScheduleField (Get-NodeScheduleField $Schedule 'releases') $majorKey
        $start=Get-NodeScheduleDate $release 'start';$end=Get-NodeScheduleDate $release 'end'
        $lts=Get-NodeScheduleDate $release 'lts';$maintenance=Get-NodeScheduleDate $release 'maintenance'
        if($null -eq $start -or $null -eq $end -or $start -gt $end -or $start -gt $now){return $result}
        if(($null -ne $lts -and ($lts -lt $start -or $lts -gt $end)) -or
            ($null -ne $maintenance -and ($maintenance -lt $start -or $maintenance -gt $end)) -or
            ($null -ne $lts -and $null -ne $maintenance -and $maintenance -lt $lts)){return $result}
        $result.end_date=$end.ToString('yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture)
        $result.status=if($now -ge $end){'EOL'}elseif($null -ne $maintenance -and $now -ge $maintenance){if($null -ne $lts -and $now -ge $lts){'MAINTENANCE_LTS'}else{'MAINTENANCE'}}elseif($null -ne $lts -and $now -ge $lts){'ACTIVE_LTS'}else{'CURRENT'}
    }catch{$result.status='UNKNOWN';$result.end_date=$null}
    return $result
}
function Convert-NodeFileMetadata($Metadata,[string]$LocationHash,[string]$Scope,[string]$Hint) {
    if($LocationHash -notmatch '\A[a-f0-9]{64}\z' -or $Scope -notin @('user','machine','unknown') -or $Hint -notin @('default','path','nvm','fnm','volta','nvs')){throw 'NODE_PROVENANCE_INVALID'}
    $identified=$null -ne $Metadata -and $null -ne $Metadata.PSObject.Properties['OriginalFilename'] -and
        $Metadata.OriginalFilename -is [string] -and $Metadata.OriginalFilename -ieq 'node.exe' -and
        $null -ne $Metadata.PSObject.Properties['ProductName'] -and $Metadata.ProductName -is [string] -and $Metadata.ProductName -ceq 'Node.js'
    $version=$null;$publisher=$null
    if($identified){
        if($null -ne $Metadata.PSObject.Properties['ProductVersion'] -and $Metadata.ProductVersion -is [string] -and
            $Metadata.ProductVersion.Length -le 256 -and $Metadata.ProductVersion -cmatch '\Av?[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\z'){$version=$Metadata.ProductVersion}
        if($null -ne $Metadata.PSObject.Properties['CompanyName']){$publisher=Get-CleanText $Metadata.CompanyName}
    }
    return [pscustomobject]@{
        display_name=if($identified){'Node.js'}else{'Node.js candidate'};publisher=$publisher;installed_version=$version
        normalized_id=if($identified){'nodejs:node'}else{$null};install_path_hash=$LocationHash;installation_source='UNKNOWN'
        scanner='node';source_id=$LocationHash;scope=$Scope;view='metadata';catalog_source=('pe-version-unverified:'+ $Hint);profile_ref=$null
    }
}
function Get-NodeMetadataRoots {
    $local=[Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
    $roaming=[Environment]::GetFolderPath([Environment+SpecialFolder]::ApplicationData)
    foreach($folder in @([Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles),[Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFilesX86))){
        [pscustomobject]@{path=[IO.Path]::Combine($folder,'nodejs');layout='direct';scope='machine';hint='default'}
    }
    [pscustomobject]@{path=[IO.Path]::Combine($local,'Programs/nodejs');layout='direct';scope='user';hint='default'}
    foreach($root in @(
        [pscustomobject]@{path=[IO.Path]::Combine($roaming,'nvm');layout='versions';scope='user';hint='nvm'},
        [pscustomobject]@{path=[IO.Path]::Combine($local,'fnm/node-versions');layout='fnm';scope='user';hint='fnm'},
        [pscustomobject]@{path=[IO.Path]::Combine($local,'Volta/tools/image/node');layout='versions';scope='user';hint='volta'},
        [pscustomobject]@{path=[IO.Path]::Combine($local,'nvs/node');layout='versions';scope='user';hint='nvs'})){$root}
    foreach($manager in @(@('NVM_HOME','nvm','versions'),@('FNM_DIR','fnm','fnm'),@('VOLTA_HOME','volta','volta'),@('NVS_HOME','nvs','nvs'))){
        $configured=[Environment]::GetEnvironmentVariable($manager[0])
        if([string]::IsNullOrWhiteSpace($configured)){continue}
        $suffix=if($manager[1] -eq 'fnm'){'node-versions'}elseif($manager[1] -eq 'volta'){'tools/image/node'}elseif($manager[1] -eq 'nvs'){'node'}else{''}
        [pscustomobject]@{path=[IO.Path]::Combine($configured,$suffix);layout=if($manager[1] -eq 'fnm'){'fnm'}else{'versions'};scope='unknown';hint=$manager[1]}
    }
    $pathValue=[Environment]::GetEnvironmentVariable('PATH')
    if($null -ne $pathValue){$count=0;foreach($directory in $pathValue.Split(';')){if($count++ -ge 256){break};if($directory.Length -gt 0 -and $directory.Length -le 4096){[pscustomobject]@{path=$directory;layout='direct';scope='unknown';hint='path'}}}}
}
function Get-NodeInventory([object[]]$Roots=$null,$Schedule=$null) {
    if($null -eq $Roots){$Roots=@(Get-NodeMetadataRoots)}
    $records=[Collections.Generic.List[object]]::new();$codes=[Collections.Generic.HashSet[string]]::new()
    $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach($code in @('PATH_AND_KNOWN_MANAGERS_ONLY','PE_METADATA_UNVERIFIED','REPARSE_NETWORK_PATHS_NOT_SCANNED')){[void]$codes.Add($code)}
    if($null -eq $Schedule){try{
        if(-not$env:SAFEPATCH_NODE_SCHEDULE_FILE){throw 'NODE_SUPPORT_SOURCE_UNAVAILABLE'}
        $Schedule=Read-LocalMetadataJson $env:SAFEPATCH_NODE_SCHEDULE_FILE
    }catch{[void]$codes.Add('NODE_SUPPORT_SOURCE_UNAVAILABLE')}}
    $timer=[Diagnostics.Stopwatch]::StartNew()
    foreach($root in $Roots){
        if(Test-Cancelled){[void]$codes.Add('CANCELLED');break}
        if($timer.Elapsed.TotalSeconds -gt 15 -or $records.Count -ge 200){[void]$codes.Add('NODE_METADATA_LIMIT');break}
        try{
            if(-not(Test-LocalMetadataPath $root.path)){continue}
            $directories=[Collections.Generic.List[string]]::new()
            if($root.layout -eq 'direct'){$directories.Add($root.path)}
            elseif($root.layout -in @('versions','fnm')){
                $count=0
                foreach($directory in [IO.Directory]::EnumerateDirectories($root.path)){
                    if(Test-Cancelled -or $timer.Elapsed.TotalSeconds -gt 15 -or $count++ -ge 64){[void]$codes.Add('NODE_METADATA_LIMIT');break}
                    if(-not(Test-LocalMetadataPath $directory)){continue}
                    $directories.Add($(if($root.layout -eq 'fnm'){[IO.Path]::Combine($directory,'installation')}else{$directory}))
                }
            }else{[void]$codes.Add('NODE_LAYOUT_UNSUPPORTED');continue}
            foreach($directory in $directories){
                if(Test-Cancelled){[void]$codes.Add('CANCELLED');break}
                if($timer.Elapsed.TotalSeconds -gt 15 -or $records.Count -ge 200){[void]$codes.Add('NODE_METADATA_LIMIT');break}
                $binary=[IO.Path]::Combine($directory,'node.exe')
                if(-not(Test-LocalMetadataPath $binary)){continue}
                $binary=[IO.Path]::GetFullPath($binary).Replace('/','\')
                if(-not$seen.Add($binary)){continue}
                $metadata=$null
                try{
                    if(([IO.FileInfo]$binary).Length -gt 268435456){throw 'NODE_FILE_SIZE_LIMIT'}
                    $metadata=[Diagnostics.FileVersionInfo]::GetVersionInfo($binary)
                }catch{[void]$codes.Add('NODE_RESOURCE_UNAVAILABLE')}
                $record=Convert-NodeFileMetadata $metadata (Get-PathHash $binary) $root.scope $root.hint
                if($null -eq $record.installed_version){[void]$codes.Add('NODE_VERSION_UNKNOWN')}
                if($null -eq $record.normalized_id){[void]$codes.Add('NODE_IDENTITY_UNKNOWN')}
                $record | Add-Member -NotePropertyName support -NotePropertyValue (Get-NodeSupport $record.installed_version $Schedule)
                if($record.support.status -eq 'UNKNOWN'){[void]$codes.Add('NODE_SUPPORT_UNKNOWN')}
                $records.Add($record)
            }
        }catch{[void]$codes.Add('NODE_ROOT_UNAVAILABLE')}
    }
    return [pscustomobject]@{records=@($records);coverage=@([pscustomobject]@{scanner='node';status=if($records.Count -gt 0){'PARTIAL'}else{'UNKNOWN'};count=$records.Count;codes=@($codes)})}
}
