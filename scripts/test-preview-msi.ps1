# CI metadata fixture only; never installation or product/device acceptance (#22).
$ErrorActionPreference='Stop'
if($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted'){throw 'PREVIEW_CI_ONLY'}
. (Join-Path $PSScriptRoot 'preview-contracts.ps1')
$file=Join-Path $env:RUNNER_TEMP ('preview-msi-table-fixture-'+[Guid]::NewGuid().ToString('N')+'.msi')
$installer=New-Object -ComObject WindowsInstaller.Installer
$writer=$installer.OpenDatabase($file,3)
foreach($sql in @('CREATE TABLE LaunchCondition (Condition CHAR(255) NOT NULL, Description CHAR(255) PRIMARY KEY Condition)','INSERT INTO LaunchCondition (Condition, Description) VALUES (''NOT ALLUSERS'', ''fixture'')')){
 $view=$writer.OpenView($sql);$null=$view.Execute();$null=$view.Close();$null=[Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
}
$null=$writer.Commit();$null=[Runtime.InteropServices.Marshal]::FinalReleaseComObject($writer)
$reader=$installer.OpenDatabase($file,0)
$view=$reader.OpenView('SELECT Condition FROM LaunchCondition');$null=$view.Execute();$record=$view.Fetch()
if($null -eq $record){throw 'PREVIEW_MSI_FIXTURE_RECORD_MISSING'}
Write-Output ('PREVIEW_MSI_RECORD_TYPE='+$record.GetType().FullName)
Write-Output ('PREVIEW_MSI_DIRECT_FIELDCOUNT='+$record.FieldCount)
Write-Output ('PREVIEW_MSI_DIRECT_FIELD='+$record.StringData(1))
$reflected=$record.GetType().InvokeMember('FieldCount',[Reflection.BindingFlags]::GetProperty,$null,$record,$null)
Write-Output ('PREVIEW_MSI_REFLECTED_FIELDCOUNT='+$reflected)
$null=$view.Close();$null=[Runtime.InteropServices.Marshal]::FinalReleaseComObject($record);$null=[Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
$rows=Read-PreviewMsiRows $reader 'SELECT Condition FROM LaunchCondition'
Write-Output ('PREVIEW_MSI_FIXTURE_ROWS='+($rows|ConvertTo-Json -Depth 5 -Compress))
if($rows.Count -ne 1 -or $rows[0].Fields[0] -ne 'NOT ALLUSERS'){throw 'PREVIEW_MSI_REAL_TABLE_READ_REGRESSION'}
$two=Read-PreviewMsiRows $reader 'SELECT Condition, Description FROM LaunchCondition'
if($two.Count -ne 1 -or $two[0].Fields.Count -ne 2 -or $two[0].Fields[1] -ne 'fixture'){throw 'PREVIEW_MSI_REAL_TWO_COLUMN_REGRESSION'}
$empty=Read-PreviewMsiRows $reader "SELECT Condition FROM LaunchCondition WHERE Condition='missing'"
if($empty.Count -ne 0){throw 'PREVIEW_MSI_REAL_EMPTY_REGRESSION'}
$null=[Runtime.InteropServices.Marshal]::FinalReleaseComObject($reader)
Write-Output 'Actual Windows Installer table fixture: single/two/empty queries PASS; not product/device acceptance.'
