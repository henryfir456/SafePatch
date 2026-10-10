# Share failure classification between actual lifecycle and negative regression.
function Get-PreviewUiFailureStatus([bool]$ProviderEstablished,[string]$Code) {
 if(-not $ProviderEstablished -and $Code -eq 'PREVIEW_UIA_PROVIDER_UNAVAILABLE'){return 'NOT_VERIFIED'}
 return 'FAILED'
}
