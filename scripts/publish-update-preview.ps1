# Runs only for the explicitly pushed v0.2.0-preview tag, after reviewed main CI.
$ErrorActionPreference='Stop'
if($env:GITHUB_ACTIONS -ne 'true' -or $env:GITHUB_REF -ne 'refs/tags/v0.2.0-preview' -or $env:GITHUB_REPOSITORY -ne 'henryfir456/SafePatch'){throw 'RELEASE_CONTEXT_INVALID'}
$repo=$env:GITHUB_REPOSITORY
$commit=(& git rev-parse HEAD).Trim()
function Confirm-Tag {
 $lines=@(& git ls-remote origin refs/tags/v0.2.0-preview 'refs/tags/v0.2.0-preview^{}')
 if($LASTEXITCODE -ne 0){throw 'RELEASE_TAG_LOOKUP_FAILED'}
 $tagRefs=@{};foreach($line in $lines){$parts=$line -split '\s+';if($parts.Count -eq 2){$tagRefs[$parts[1]]=$parts[0]}}
 $target=if($tagRefs.ContainsKey('refs/tags/v0.2.0-preview^{}')){$tagRefs['refs/tags/v0.2.0-preview^{}']}else{$tagRefs['refs/tags/v0.2.0-preview']}
 if($target -cne $commit){throw 'RELEASE_TAG_CHANGED'}
}
function Confirm-Main {
 Confirm-Tag
 $remote=(& git ls-remote origin refs/heads/main)
 if($LASTEXITCODE -ne 0 -or ($remote -split '\s+')[0] -cne $commit){throw 'RELEASE_MAIN_CHANGED'}
}
Confirm-Main
$runs=(& gh api "repos/$repo/actions/workflows/windows.yml/runs?branch=main&event=push&status=success&per_page=30"|ConvertFrom-Json)
if($LASTEXITCODE -ne 0){throw 'RELEASE_RUN_LOOKUP_FAILED'}
$run=@($runs.workflow_runs|Where-Object {$_.head_sha -ceq $commit -and $_.conclusion -ceq 'success' -and $_.event -ceq 'push' -and $_.head_branch -ceq 'main'})|Select-Object -First 1
if(-not $run){throw 'RELEASE_MAIN_CI_NOT_VERIFIED'}
$artifacts=(& gh api "repos/$repo/actions/runs/$($run.id)/artifacts"|ConvertFrom-Json)
if($LASTEXITCODE -ne 0){throw 'RELEASE_ARTIFACT_LOOKUP_FAILED'}
$artifact=@($artifacts.artifacts|Where-Object {$_.name -ceq "SafePatch-Windows-Preview-$commit" -and -not $_.expired})
if($artifact.Count -ne 1 -or $artifact[0].digest -notmatch '^sha256:[0-9a-f]{64}$'){throw 'RELEASE_ARTIFACT_IDENTITY_INVALID'}
$root=Join-Path $env:RUNNER_TEMP ('safepatch-release-'+[guid]::NewGuid().ToString('N'))
$null=[IO.Directory]::CreateDirectory($root)
$archive=Join-Path $root 'preview.zip'
# Native byte stream preserves ZIP bytes independent of PowerShell text encoding.
$info=[Diagnostics.ProcessStartInfo]::new();$info.FileName=(Get-Command gh.exe).Source
$info.ArgumentList.Add('api');$info.ArgumentList.Add("repos/$repo/actions/artifacts/$($artifact[0].id)/zip")
$info.UseShellExecute=$false;$info.CreateNoWindow=$true;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
$process=[Diagnostics.Process]::new();$process.StartInfo=$info
$stream=[IO.File]::Open($archive,[IO.FileMode]::CreateNew)
try {
 [void]$process.Start()
 $out=$process.StandardOutput.BaseStream.CopyToAsync($stream);$err=$process.StandardError.BaseStream.CopyToAsync([IO.Stream]::Null)
 if(-not $process.WaitForExit(300000)){$process.Kill();$process.WaitForExit();throw 'RELEASE_DOWNLOAD_TIMEOUT'}
 [void]$out.GetAwaiter().GetResult();[void]$err.GetAwaiter().GetResult()
 if($process.ExitCode -ne 0){throw 'RELEASE_DOWNLOAD_FAILED'}
} finally {$stream.Dispose();$process.Dispose()}
if((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -cne $artifact[0].digest.Substring(7)){throw 'RELEASE_ARCHIVE_HASH_MISMATCH'}
$stage=Join-Path $root 'stage';Expand-Archive -LiteralPath $archive -DestinationPath $stage
$expected=@('SafePatch-Preview-0.2.0-x64-setup.exe','SafePatch-Preview-0.2.0-x64.msi','safepatch.exe','README.md','manifest.json','ci-package-checks.json','ci-lifecycle.json','SHA256SUMS.txt')
$files=@(Get-ChildItem -LiteralPath $stage -File)
if($files.Count -ne $expected.Count -or @(Get-ChildItem -LiteralPath $stage -Directory).Count -ne 0 -or @($files|Where-Object {$_.Name -cnotin $expected}).Count -ne 0){throw 'RELEASE_FILE_SET_INVALID'}
$manifest=Get-Content (Join-Path $stage 'manifest.json') -Raw|ConvertFrom-Json
$checks=Get-Content (Join-Path $stage 'ci-package-checks.json') -Raw|ConvertFrom-Json
$lifecycle=Get-Content (Join-Path $stage 'ci-lifecycle.json') -Raw|ConvertFrom-Json
if($manifest.source_commit -cne $commit -or $manifest.version -cne '0.2.0' -or $manifest.signature -cne 'UNSIGNED' -or $checks.status -cne 'PASS' -or $lifecycle.status -notin @('PASS','NOT_VERIFIED')){throw 'RELEASE_MANIFEST_NOT_VERIFIED'}
$hashes=@{}
foreach($line in Get-Content (Join-Path $stage 'SHA256SUMS.txt')){
 if($line -cnotmatch '^([0-9a-f]{64})  ([A-Za-z0-9._-]+)$' -or $matches[2] -cnotin $expected -or $matches[2] -ceq 'SHA256SUMS.txt' -or $hashes.ContainsKey($matches[2])){throw 'RELEASE_CHECKSUM_SET_INVALID'}
 $hashes[$matches[2]]=$matches[1]
}
if($hashes.Count -ne $expected.Count-1){throw 'RELEASE_CHECKSUM_COUNT_INVALID'}
foreach($name in $hashes.Keys){if((Get-FileHash -LiteralPath (Join-Path $stage $name) -Algorithm SHA256).Hash.ToLowerInvariant() -cne $hashes[$name]){throw 'RELEASE_PAYLOAD_HASH_MISMATCH'}}
if($hashes['safepatch.exe'] -cne $manifest.production_sha256){throw 'RELEASE_PRODUCTION_IDENTITY_INVALID'}
# No overwrites, no execution of downloaded installers or binaries.
& gh release view v0.2.0-preview --repo $repo *> $null
if($LASTEXITCODE -eq 0){throw 'RELEASE_ALREADY_EXISTS'}
$notes=Join-Path $root 'release-notes.md'
@"
SafePatch 0.2.0 Preview：唯讀 Windows 軟體更新健檢。

- 型別化 WinGet COM 查詢、官方來源與精確產品／版本證據；更新狀態和漏洞風險獨立。
- 預覽包尚未簽章（UNSIGNED），需既有 WebView2；不安裝／自動更新軟體、不提升權限、不上傳私人盤點。
- 本次乾淨 Windows Server main CI：$($run.html_url)。來源 commit：$commit。
- NO_UPDATE_REPORTED 不保證最新；UNKNOWN 不代表安全；漏洞尚未檢查。
- 本機 Windows 11 的 adapter／Rust／native 驗證請參考同版本 README 與 repository docs/UPDATE_HEALTH_VERIFICATION.md；使用者安裝實測仍另行驗收。
- 包含 EXE／MSI、SHA256SUMS、manifest 與實際 CI 驗證報告。
"@|Set-Content -LiteralPath $notes -Encoding utf8
Confirm-Main
& gh release create v0.2.0-preview --repo $repo --verify-tag --draft --prerelease --title 'SafePatch 0.2.0 Preview — 更新健檢' --notes-file $notes
if($LASTEXITCODE -ne 0){throw 'RELEASE_CREATE_FAILED'}
& gh release upload v0.2.0-preview @($files.FullName) --repo $repo
if($LASTEXITCODE -ne 0){throw 'RELEASE_UPLOAD_FAILED'}
$download=Join-Path $root 'verify'
& gh release download v0.2.0-preview --repo $repo --dir $download
if($LASTEXITCODE -ne 0 -or @(Get-ChildItem -LiteralPath $download -File).Count -ne $files.Count){throw 'RELEASE_REDOWLOAD_FAILED'}
foreach($file in $files){if((Get-FileHash -LiteralPath (Join-Path $download $file.Name) -Algorithm SHA256).Hash -cne (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash){throw 'RELEASE_UPLOADED_HASH_MISMATCH'}}
Confirm-Main
& gh release edit v0.2.0-preview --repo $repo --draft=false
if($LASTEXITCODE -ne 0){throw 'RELEASE_PUBLISH_FAILED'}
Write-Output 'Published digest-verified v0.2.0 Preview from successful reviewed main CI; installer user acceptance remains separate.'
