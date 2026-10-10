. (Join-Path $PSScriptRoot 'preview-contracts.ps1')
$ErrorActionPreference='Stop'
foreach($case in @(
 @($false,'PREVIEW_UIA_PROVIDER_UNAVAILABLE','NOT_VERIFIED'),
 @($true,'PREVIEW_UIA_PROVIDER_UNAVAILABLE','FAILED'),
 @($true,'PREVIEW_UIA_RESTORED_RESULT_UNAVAILABLE','FAILED'),
 @($true,'PREVIEW_UIA_CONTROL_UNAVAILABLE','FAILED'),
 @($false,'PREVIEW_RESTART_MISMATCH','FAILED')
)){
 if((Get-PreviewUiFailureStatus $case[0] $case[1]) -ne $case[2]){throw 'PREVIEW_FAILURE_CLASSIFICATION_REGRESSION'}
}
$bytes=[IO.File]::ReadAllBytes((Join-Path $PSScriptRoot 'verify-preview-lifecycle.ps1'))
if($bytes.Length -lt 3 -or [BitConverter]::ToString($bytes,0,3) -ne 'EF-BB-BF'){throw 'PREVIEW_WINDOWS_PS_ENCODING_REGRESSION'}
$tokens=$null;$errors=$null
$null=[Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'verify-preview-lifecycle.ps1'),[ref]$tokens,[ref]$errors)
if($errors.Count){throw 'PREVIEW_WINDOWS_PS_SYNTAX_REGRESSION'}
$expected=-join (@(0x958B,0x59CB,0x552F,0x8B80,0x6383,0x63CF)|ForEach-Object{[char]$_})
$literal=@($tokens|Where-Object{$_.Value -eq $expected})
if($literal.Count -lt 2){throw 'PREVIEW_WINDOWS_PS_UI_LITERAL_REGRESSION'}
Write-Output 'Preview contracts: 5 failure cases, BOM, selected-shell parse/literals PASS'
