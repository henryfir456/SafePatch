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
