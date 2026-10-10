# Product-owned metadata readers. Extension entry points and scripts are data only.
function Convert-VSCodeExtensionManifest($Manifest,[string]$LocationHash,[string]$Scope,[string]$ProfileRef) {
    foreach($field in @('publisher','name')) {
        if($null -eq $Manifest -or $null -eq $Manifest.PSObject.Properties[$field] -or $Manifest.$field -isnot [string] -or
            $Manifest.$field -notmatch '\A[A-Za-z0-9][A-Za-z0-9_-]{0,127}\z'){throw 'VSCODE_MANIFEST_INVALID'}
    }
    if($LocationHash -notmatch '\A[a-f0-9]{64}\z' -or $Scope -notin @('user','machine','unknown') -or
        $ProfileRef -notmatch '\A[A-Za-z0-9_-]{1,128}\z'){throw 'VSCODE_PROVENANCE_INVALID'}
    $version=if($null -ne $Manifest.PSObject.Properties['version'] -and $Manifest.version -is [string]){$Manifest.version}else{$null}
    if($null -ne $version -and ($version.Length -gt 256 -or $version -notmatch '\A\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\z')){$version=$null}
    $id=($Manifest.publisher+'.'+$Manifest.name).ToLowerInvariant()
    return [pscustomobject]@{
        display_name=$id;publisher=$Manifest.publisher;installed_version=$version;normalized_id=('vscode-extension:'+$id)
        install_path_hash=$LocationHash;installation_source='UNKNOWN';scanner='vscode-extension'
        source_id=(Get-PathHash ($LocationHash+'\'+$ProfileRef));scope=$Scope;view='metadata';catalog_source='manifest-unverified';profile_ref=$ProfileRef
    }
}
function Test-LocalMetadataPath([string]$Path) {
    # Walk from root to leaf so a junction is rejected before traversing children.
    try {
        if($Path -notmatch '\A[A-Za-z]:[\\/]'){return $false}
        $full=[IO.Path]::GetFullPath($Path).Replace('/','\');$root=[IO.Path]::GetPathRoot($full)
        if(([IO.DriveInfo]::new($root)).DriveType -eq [IO.DriveType]::Network){return $false}
        $current=$root
        foreach($part in $full.Substring($root.Length).Split('\')) {
            if($part.Length -eq 0){continue}
            if($part.Contains(':')){return $false}
            $current=[IO.Path]::Combine($current,$part)
            if(([IO.File]::GetAttributes($current) -band [IO.FileAttributes]::ReparsePoint) -ne 0){return $false}
        }
        return $true
    } catch {return $false}
}
function Read-LocalMetadataJson([string]$Path,[string]$Kind='object') {
    if(-not (Test-LocalMetadataPath $Path)){throw 'METADATA_PATH_UNAVAILABLE'}
    if(([IO.FileInfo]$Path).Length -gt 1048576){throw 'METADATA_SIZE_LIMIT'}
    $text=[IO.File]::ReadAllText($Path).TrimStart()
    $opening=if($Kind -eq 'array'){'['}else{'{'}
    if(-not $text.StartsWith($opening,[StringComparison]::Ordinal)){throw 'METADATA_FORMAT_UNSUPPORTED'}
    try {$document=$text | ConvertFrom-Json} catch {throw 'METADATA_JSON_INVALID'}
    return ,$document
}
function Get-VSCodeMetadataRoots {
    $profile=[Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    $local=[Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
    $roaming=[Environment]::GetFolderPath([Environment+SpecialFolder]::ApplicationData)
    $programs=[Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
    foreach($channel in @('stable','insiders')) {
        $folder=if($channel -eq 'stable'){'Microsoft VS Code'}else{'Microsoft VS Code Insiders'}
        $dataFolder=if($channel -eq 'stable'){'Code/User'}else{'Code - Insiders/User'}
        $store=if($channel -eq 'stable'){'.vscode/extensions'}else{'.vscode-insiders/extensions'}
        [pscustomobject]@{extension_root=[IO.Path]::Combine($profile,$store);user_data=[IO.Path]::Combine($roaming,$dataFolder);channel=$channel;scope='user';application_roots=@(
            [pscustomobject]@{path=[IO.Path]::Combine($local,'Programs',$folder);scope='user'},
            [pscustomobject]@{path=[IO.Path]::Combine($programs,$folder);scope='machine'})}
    }
}
function Get-VSCodeIndexedLocation($Entry,[string]$ExtensionRoot) {
    $location=$null
    if($null -ne $Entry.PSObject.Properties['relativeLocation'] -and $Entry.relativeLocation -is [string] -and
        $Entry.relativeLocation -match '\A[^\\/:.][^\\/:]*\z') {
        $location=[IO.Path]::Combine($ExtensionRoot,$Entry.relativeLocation)
    } elseif($null -ne $Entry.PSObject.Properties['location'] -and $Entry.location -isnot [string] -and
        $null -ne $Entry.location.PSObject.Properties['scheme'] -and $Entry.location.scheme -ceq 'file' -and
        ($null -eq $Entry.location.PSObject.Properties['authority'] -or [string]::IsNullOrEmpty($Entry.location.authority)) -and
        $null -ne $Entry.location.PSObject.Properties['path'] -and $Entry.location.path -is [string]) {
        $location=$Entry.location.path
        if($location -match '\A/[A-Za-z]:/'){$location=$location.Substring(1)}
    }
    if($null -eq $location -or -not (Test-LocalMetadataPath $location)){throw 'VSCODE_PROFILE_LOCATION_UNKNOWN'}
    return Get-PathHash ([IO.Path]::GetFullPath($location).Replace('/','\'))
}
function Add-VSCodeProfileIndex($Map,[string]$Path,[string]$ProfileRef,[string]$ExtensionRoot) {
    $entries=Read-LocalMetadataJson $Path 'array'
    if(@($entries).Count -gt 5000){throw 'VSCODE_PROFILE_LIMIT'}
    $incomplete=$false
    foreach($entry in @($entries)) {try{
        if($null -eq $entry -or $null -eq $entry.PSObject.Properties['identifier'] -or
            $null -eq $entry.identifier.PSObject.Properties['id'] -or $entry.identifier.id -isnot [string] -or
            $entry.identifier.id -notmatch '\A[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\z' -or
            $null -eq $entry.PSObject.Properties['version'] -or $entry.version -isnot [string]){throw 'VSCODE_PROFILE_INDEX_INVALID'}
        $key=$entry.identifier.id.ToLowerInvariant()+'|'+$entry.version+'|'+(Get-VSCodeIndexedLocation $entry $ExtensionRoot)
        if(-not $Map.ContainsKey($key)){$Map[$key]=[Collections.Generic.List[string]]::new()}
        if(-not $Map[$key].Contains($ProfileRef)){$Map[$key].Add($ProfileRef)}
    }catch{$incomplete=$true}}
    if($incomplete){throw 'VSCODE_PROFILE_INDEX_PARTIAL'}
}
function Get-VSCodeInventory([object[]]$Roots=$null) {
    if($null -eq $Roots){$Roots=@(Get-VSCodeMetadataRoots)}
    $records=[Collections.Generic.List[object]]::new()
    $appCodes=[Collections.Generic.HashSet[string]]::new();$extensionCodes=[Collections.Generic.HashSet[string]]::new()
    foreach($code in @('CUSTOM_PORTABLE_INSTALLATIONS_NOT_SCANNED','APPLICATION_ACTIVATION_NOT_VERIFIED')){[void]$appCodes.Add($code)}
    foreach($code in @('CUSTOM_EXTENSION_LOCATIONS_NOT_SCANNED','MANIFEST_PROVENANCE_UNVERIFIED','ACTIVATION_NOT_VERIFIED','PROFILE_INHERITANCE_UNKNOWN')){[void]$extensionCodes.Add($code)}
    $timer=[Diagnostics.Stopwatch]::StartNew();$appCount=0;$extensionCount=0
    foreach($rootInfo in $Roots) {
        if(Test-Cancelled){[void]$extensionCodes.Add('CANCELLED');break}
        try {
        $profiles=@{}
        $defaultIndex=[IO.Path]::Combine($rootInfo.extension_root,'extensions.json')
        if(Test-LocalMetadataPath $defaultIndex){try{Add-VSCodeProfileIndex $profiles $defaultIndex ($rootInfo.channel+'-default') $rootInfo.extension_root}catch{[void]$extensionCodes.Add('VSCODE_PROFILE_INDEX_UNAVAILABLE')}}
        $profileRoot=[IO.Path]::Combine($rootInfo.user_data,'profiles')
        if(Test-LocalMetadataPath $profileRoot){
            $profileCount=0
            try { foreach($directory in [IO.Directory]::EnumerateDirectories($profileRoot)) {
                if(Test-Cancelled -or $timer.Elapsed.TotalSeconds -gt 15 -or $profileCount -ge 100){[void]$extensionCodes.Add('VSCODE_PROFILE_LIMIT');break}
                $profileCount++;$basename=[IO.Path]::GetFileName($directory)
                $profileId=if($basename -match '\A[a-fA-F0-9-]{1,64}\z'){$basename}else{'profile-'+(Get-PathHash $basename).Substring(0,16)}
                $index=[IO.Path]::Combine($directory,'extensions.json')
                try{Add-VSCodeProfileIndex $profiles $index ($rootInfo.channel+'-'+$profileId) $rootInfo.extension_root}catch{[void]$extensionCodes.Add('VSCODE_PROFILE_INDEX_UNAVAILABLE')}
            }}catch{[void]$extensionCodes.Add('VSCODE_PROFILE_ENUMERATION_UNAVAILABLE')}
        }
        $obsolete=@{}
        $obsoleteFile=[IO.Path]::Combine($rootInfo.extension_root,'.obsolete')
        if(Test-LocalMetadataPath $obsoleteFile){try{$old=Read-LocalMetadataJson $obsoleteFile;foreach($property in $old.PSObject.Properties){if($property.Value -isnot [bool]){[void]$extensionCodes.Add('VSCODE_OBSOLETE_INDEX_UNAVAILABLE')}elseif($property.Value){$obsolete[$property.Name]=$true}}}catch{[void]$extensionCodes.Add('VSCODE_OBSOLETE_INDEX_UNAVAILABLE')}}
        if(Test-LocalMetadataPath $rootInfo.extension_root){
            try { foreach($directory in [IO.Directory]::EnumerateDirectories($rootInfo.extension_root)) {
                if(Test-Cancelled){[void]$extensionCodes.Add('CANCELLED');break}
                if($timer.Elapsed.TotalSeconds -gt 15 -or $extensionCount -ge 5000){[void]$extensionCodes.Add('VSCODE_METADATA_LIMIT');break}
                if($obsolete.ContainsKey([IO.Path]::GetFileName($directory))){continue}
                try {
                    $manifest=Read-LocalMetadataJson ([IO.Path]::Combine($directory,'package.json'))
                    $candidate=Convert-VSCodeExtensionManifest $manifest (Get-PathHash $directory) $rootInfo.scope 'UNKNOWN'
                    $key=$candidate.display_name+'|'+$candidate.installed_version+'|'+$candidate.install_path_hash
                    $references=if($profiles.ContainsKey($key)){@($profiles[$key])}else{@('UNKNOWN')}
                    foreach($reference in $references){
                        if($extensionCount -ge 5000){[void]$extensionCodes.Add('VSCODE_METADATA_LIMIT');break}
                        $record=Convert-VSCodeExtensionManifest $manifest (Get-PathHash $directory) $rootInfo.scope $reference;$records.Add($record);$extensionCount++
                    }
                    if($null -eq $candidate.installed_version){[void]$extensionCodes.Add('VSCODE_VERSION_UNKNOWN')}
                }catch{[void]$extensionCodes.Add('VSCODE_EXTENSION_METADATA_UNAVAILABLE')}
            }}catch{[void]$extensionCodes.Add('VSCODE_EXTENSION_ENUMERATION_UNAVAILABLE')}
        }
        foreach($application in @($rootInfo.application_roots)) {
            if(Test-Cancelled){[void]$appCodes.Add('CANCELLED');break}
            if($timer.Elapsed.TotalSeconds -gt 15){[void]$appCodes.Add('VSCODE_METADATA_LIMIT');break}
            $applicationPath=if($application -is [string]){$application}else{$application.path}
            $scope=if($application -is [string]){$rootInfo.scope}else{$application.scope}
            if(-not (Test-LocalMetadataPath $applicationPath)){continue}
            $locations=[Collections.Generic.List[string]]::new();$locations.Add($applicationPath)
            try{
                $childCount=0
                foreach($childPath in [IO.Directory]::EnumerateDirectories($applicationPath)){
                    if(Test-Cancelled -or $timer.Elapsed.TotalSeconds -gt 15 -or $childCount -ge 20){[void]$appCodes.Add('VSCODE_METADATA_LIMIT');break}
                    $childCount++
                    if(Test-LocalMetadataPath $childPath){$locations.Add($childPath)}
                }
            }catch{[void]$appCodes.Add('VSCODE_APPLICATION_ENUMERATION_UNAVAILABLE')}
            $foundApplication=$false
            foreach($metadataLocation in $locations){
            if(Test-Cancelled -or $timer.Elapsed.TotalSeconds -gt 15){[void]$appCodes.Add('VSCODE_METADATA_LIMIT');break}
            $packagePath=[IO.Path]::Combine($metadataLocation,'resources/app/package.json')
            if(-not (Test-LocalMetadataPath $packagePath)){continue}
            $foundApplication=$true
            try {
                $appManifest=Read-LocalMetadataJson $packagePath
                $product=Read-LocalMetadataJson ([IO.Path]::Combine($metadataLocation,'resources/app/product.json'))
                if($product.applicationName -isnot [string] -or $product.applicationName -notmatch '\A[A-Za-z0-9_-]{1,128}\z'){throw 'VSCODE_PRODUCT_INVALID'}
                $version=if($null -ne $appManifest.PSObject.Properties['version'] -and $appManifest.version -is [string] -and $appManifest.version -match '\A\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?\z'){$appManifest.version}else{$null}
                $name=Get-CleanText $product.nameShort
                if($null -eq $name){$name='VS Code metadata'}
                $pathHash=Get-PathHash $metadataLocation
                $records.Add([pscustomobject]@{display_name=$name;publisher=$null;installed_version=$version;normalized_id=('vscode-app:'+$product.applicationName.ToLowerInvariant());install_path_hash=$pathHash;installation_source='UNKNOWN';scanner='vscode';source_id=$pathHash;scope=$scope;view='metadata';catalog_source='manifest-unverified';profile_ref=$null})
                $appCount++
            }catch{
                [void]$appCodes.Add('VSCODE_APPLICATION_METADATA_UNAVAILABLE')
                if($_.Exception.Message -match '\A(?:METADATA_[A-Z_]+|VSCODE_PRODUCT_INVALID)\z'){[void]$appCodes.Add($_.Exception.Message)}
            }
            }
            if(-not $foundApplication){[void]$appCodes.Add('VSCODE_APPLICATION_METADATA_UNAVAILABLE')}
        }
        }catch{[void]$extensionCodes.Add('VSCODE_ROOT_UNAVAILABLE');[void]$appCodes.Add('VSCODE_ROOT_UNAVAILABLE')}
    }
    return [pscustomobject]@{records=@($records);coverage=@(
        [pscustomobject]@{scanner='vscode';status=if($appCount -gt 0){'PARTIAL'}else{'UNKNOWN'};count=$appCount;codes=@($appCodes)},
        [pscustomobject]@{scanner='vscode-extension';status=if($extensionCount -gt 0){'PARTIAL'}else{'UNKNOWN'};count=$extensionCount;codes=@($extensionCodes)})}
}
