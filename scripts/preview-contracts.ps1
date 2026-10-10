# Share failure classification between actual lifecycle and negative regression.
function Get-PreviewUiFailureStatus([bool]$ProviderEstablished,[string]$Code) {
 if(-not $ProviderEstablished -and $Code -eq 'PREVIEW_UIA_PROVIDER_UNAVAILABLE'){return 'NOT_VERIFIED'}
 return 'FAILED'
}

function Read-PreviewMsiRows($Database,[string]$Sql) {
 $view=$Database.OpenView($Sql)
 $result=@()
 try {
  $null=$view.Execute()
  while($record=$view.Fetch()){
   $isCom=[Runtime.InteropServices.Marshal]::IsComObject($record)
   try {
    $count=if($isCom){[int]$record.GetType().InvokeMember('FieldCount',[Reflection.BindingFlags]::GetProperty,$null,$record,$null)}else{[int]$record.FieldCount}
    if($count -lt 1){throw 'PREVIEW_MSI_INVALID_FIELD_COUNT'}
    $row=@()
    for($i=1;$i -le $count;$i++){$row+=$record.StringData($i)}
    $result+=[pscustomobject]@{Fields=@($row)}
   } finally {if($isCom){$null=[Runtime.InteropServices.Marshal]::ReleaseComObject($record)}}
  }
 } finally {$null=$view.Close()}
 return ,$result
}
