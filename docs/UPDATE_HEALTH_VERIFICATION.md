# 更新健檢驗證紀錄（Issue #26）

## 最終設計與已執行的 Windows 11 驗證

2026-10-13，本機 Windows 11 Pro 10.0.26300.0、Windows PowerShell 5.1.26100.9444、Node 24.20.0。程序的 Administrator role 為 false，未提升權限。

- `scripts/test-winget-updates.ps1`：型別 JSON 更新／無回報／多版本／相似名不同 ID／重複衝突／格式變化／注入名稱／API 失敗、UTF-8／UTF-16 BOM／壞編碼，及真實自有 Node 子程序逾時、失敗、取消、清理通過。
- 固定 typed COM helper 以 pinned Microsoft.WindowsPackageManager.ComInterop 1.29.380 / CsWinRT 2.3.1 / self-contained .NET8 建置成功；補上配對官方 WinMD 後，本機非管理員查詢 COMPLETE、41 個可信產品、18 個 API 更新候選。此數字在 Rust 保守數字版本驗證之前，不代表最終 UI 確認更新數。
- 本機 Registry 共 97 筆觀察、WinGet export 41 筆。實際完整 Rust / native UI 結果見下節。
- Renderer 16 項合成 IPC 測試、正式 production build、SQLite schema 6 項已通過。typed COM 版 e962a5e 的 Windows CI 38384252519 已成功：Rust47/47、Clippy、EXE／MSI與編譯／解包雜湊檢查通過；hosted lifecycle 為 NOT_VERIFIED。

先前表格解析因獨立 Review 指出的名稱注入與 stdout 來源失敗警告問題而被移除。先前 19 個表格更新候選數字不是交付證據；正式掃描不解析任何人類可讀 WinGet 表格。

## 更新資料語意與限制

更新來源是 WinGet 官方 catalog（https://cdn.winget.microsoft.com/cache）。UI 提供精確 ID 與官方 winget-pkgs manifest 位置，使用者可核對 vendor installer 來源。不從不可信文字提取任意下載網址，不執行下載／安裝／更新。

COM helper 驗證來源 Id/Name/Type/Argument；直接讀取 package 物件的 InstalledVersion、IsUpdateAvailable、DefaultInstallVersion，並用 CompareToVersion 檢查目標版本。Connect/Search 失敗與截斷結果回報 ERROR。資料透過 JSON，不依賴 stdout 語系或欄位排版；不接受協議、不修改來源、不要求管理員。官方 catalog 可能使用快取，NO_UPDATE_REPORTED 不保證最新；pin／多重安裝限制明示。

只有 export 與 COM 的精確 ID／安裝版本一致時才關聯。Rust 對可比較且增加的數字點分版本確認 UPDATE_AVAILABLE，其他版本 grammar UNKNOWN。Registry、VS Code、Node、npm 不透過名稱猜測 WinGet 身分；沒有可信 catalog 證據時 UNKNOWN。WinGet 缺失時保留其他資料及原因；已確認產品的查詢逾時／失敗標 ERROR。漏洞引擎尚未實作，NOT_CHECKED 獨立於更新狀態。

SQLite schema 維持 v3；不可變 JSON 快照新增 update 與 vulnerability_status。舊 JSON 讀為 UNKNOWN / NOT_CHECKED，不重寫歷史。

## 尚待驗證

- 本機 Rust 工具鏈未安裝；以 CI 的 digest 驗證 artifact 執行 Rust / 原生 WebView，不能宣稱本機編譯。
- 最終 production EXE／MSI 在使用者 Windows11 尚未安裝；自有測試用原生 WebView已驗證，仍不等於安裝器／人工可見 UI 驗收。
- 使用者 Windows 11 安裝、實際已知更新產品的版本／來源核對、取消、再掃描、重啟、移除仍需正式包交付後驗收。
- Windows 10、其他架構、非數字版本 grammar、非 WinGet 目錄軟體均不保證更新涵蓋。

## 獨立 Review

review_typed 已重新檢查 P1 修正：explicit CLSCTX_LOCAL_SERVER、Microsoft server 簽章／packaged 登錄、使用者 override 拒絕、清除繼承 marker，並以非管理員 Windows 11 重新執行 adapter／regression，認定啟用邊界已修正、沒有新 blocker。整體 e962a5e 最後 Review 另指出零個更新時的 acceptance 等待競態；已改為等待更新與 UNKNOWN 兩個摘要值，並補延遲 unknown-only UI 回歸。Review 實際查詢為 41 個產品、18 個更新候選；16 項 renderer 通過。

發布流程另經獨立 Review、PowerShell parse 0 errors；它只在明確推送 v0.2.0-preview tag 後取用同一 main commit 的成功 CI 產物，驗證 archive digest、八檔清單／雜湊／manifest，先建 draft，上傳後重新下載比對，再確認 main／tag 未變才發布。不執行下載的安裝包，不上傳私人盤點。發布尚未執行。

## 2026-10-14 實際 Rust／原生 WebView 整合

使用 PR head e962a5e833f611eab143b3e9157f58a31d08b310 的成功 Windows CI [38384252519](https://github.com/henryfir456/SafePatch/actions/runs/38384252519)，CI checkout 為 GitHub merge-preview 2bd61cead16e067c68dd36e009f9df14bb62b8f9；此 artifact 用於驗證，正式交付仍須取合併後 main 產物。native artifact 11806860019 的 ZIP SHA256 為 ee6e9c878faf0633a955c57c892287f5135bd5f77db64a44a6e365b2cbf16030，已在下載後比對再執行。

本機 Windows11 Pro10.0.26300.0，Administrator role=false；所有驗收資料在自有 .local 目錄／暫存 SQLite，私人盤點沒有提交。

- inventory_probe --require-updates：PASS，145 正規化資產、18 UPDATE_AVAILABLE、20 NO_UPDATE_REPORTED、0 ERROR、14 候選重複、143 有版本；43秒，restart_preserved=true。
- inventory_probe --cancel：PASS，CANCELLED，3秒，restart_preserved=true。
- acceptance-feature safepatch --acceptance-check：PASS，18 更新行逐列核對目前／目標版本、ID及來源；145完整盤點行、107 UNKNOWN／ERROR摘要、143版本；cancellation=true。
- --acceptance-restart：PASS，scan_id保持、上述行數與版本／更新證據保持。

這是實際 Windows 11 程式／WebView／IPC／WinGet／Rust／SQLite 執行，並非 fixture。尚未聲稱使用者已安裝 EXE／MSI、人工可見 UI 審閱或 Windows10 驗收。其後只有驗收等待修正、合成回歸測試與文件變更；仍需最新 head CI 與合併後 main 包驗證。

最終等待修正後本機驗證：Renderer17/17、production build、schema6/6 通過；只是合成 IPC／UI 回歸，不冒充裝置掃描。
