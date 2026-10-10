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
   $row=@()
   for($i=1;$i -le $record.FieldCount;$i++){$row+=$record.StringData($i)}
   $result+=[pscustomobject]@{Fields=@($row)}
  }
 } finally {$null=$view.Close()}
 return ,$result
}
