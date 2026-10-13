$ErrorActionPreference='Stop'
$project=Join-Path $PSScriptRoot '../src-tauri/winget-helper/WinGetHelper.csproj'
$output=Join-Path $PSScriptRoot '../src-tauri/target/winget-helper'
dotnet restore $project --locked-mode
if($LASTEXITCODE -ne 0){throw 'WINGET_HELPER_RESTORE_FAILED'}
dotnet publish $project --no-restore -c Release -o $output
if($LASTEXITCODE -ne 0){throw 'WINGET_HELPER_BUILD_FAILED'}
$helper=Join-Path $output 'SafePatch.WinGetHelper.exe'
if(-not [IO.File]::Exists($helper)){throw 'WINGET_HELPER_OUTPUT_MISSING'}
Write-Output ('WINGET_HELPER_SHA256='+(Get-FileHash -LiteralPath $helper -Algorithm SHA256).Hash.ToLowerInvariant())

# Unpackaged COM clients require this paired official marshaling metadata.
$assets=Get-Content (Join-Path $PSScriptRoot '../src-tauri/winget-helper/obj/project.assets.json') -Raw|ConvertFrom-Json
$metadata=$null
foreach($folder in $assets.packageFolders.PSObject.Properties.Name){
 $candidate=Join-Path $folder 'microsoft.windowspackagemanager.cominterop/1.29.380/lib/uap10.0/Microsoft.Management.Deployment.winmd'
 if([IO.File]::Exists($candidate)){$metadata=$candidate;break}
}
if(-not $metadata){throw 'WINGET_HELPER_METADATA_MISSING'}
Copy-Item -LiteralPath $metadata -Destination (Join-Path $output 'Microsoft.Management.Deployment.winmd')
