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

# COM-like optional-method returns must not leak nulls into the row collection.
class PreviewFakeRecord {
 [int]$FieldCount=2
 [string]StringData([int]$index){if($this.FieldCount -eq 1){return 'NOT ALLUSERS'};return @('ALLUSERS','')[$index-1]}
}
class PreviewFakeView {
 [int]$Cursor=0
 [int]$Columns=2
 [object]Execute(){return $null}
 [object]Fetch(){if($this.Cursor -eq 0){$this.Cursor=1;$r=[PreviewFakeRecord]::new();$r.FieldCount=$this.Columns;return $r};return $null}
 [object]Close(){return $null}
}
class PreviewFakeDatabase {
 [PreviewFakeView]OpenView([string]$sql){$v=[PreviewFakeView]::new();if($sql -eq 'empty'){$v.Cursor=1};if($sql -eq 'single-column'){$v.Columns=1};return $v}
}
$fake=[PreviewFakeDatabase]::new()
$rows=Read-PreviewMsiRows $fake 'one'
if($rows.Count -ne 1 -or $rows[0].Fields.Count -ne 2 -or $rows[0].Fields[0] -ne 'ALLUSERS' -or $rows[0].Fields[1] -ne ''){throw 'PREVIEW_MSI_ROW_SHAPE_REGRESSION'}
$empty=Read-PreviewMsiRows $fake 'empty'
if($empty.Count -ne 0){throw 'PREVIEW_MSI_EMPTY_ROWS_REGRESSION'}
Write-Output 'Preview MSI row contracts: populated/empty COM-like views PASS'

$single=Read-PreviewMsiRows $fake 'single-column'
if($single.Count -ne 1 -or $single[0].Fields.Count -ne 1 -or $single[0].Fields[0] -ne 'NOT ALLUSERS'){throw 'PREVIEW_MSI_SINGLE_COLUMN_REGRESSION'}
if(@($single|Where-Object{$_.Fields[0] -eq 'NOT ALLUSERS'}).Count -ne 1){throw 'PREVIEW_MSI_SINGLE_COLUMN_PIPELINE_REGRESSION'}
Write-Output 'Preview MSI single-column pipeline contract PASS'
