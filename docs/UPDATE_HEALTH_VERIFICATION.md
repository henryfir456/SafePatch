# 更新健檢驗證紀錄（Issue #26）

## 最終設計與已執行的 Windows 11 驗證

2026-10-13，本機 Windows 11 Pro 10.0.26300.0、Windows PowerShell 5.1.26100.9444、Node 24.20.0。程序的 Administrator role 為 false，未提升權限。

- `scripts/test-winget-updates.ps1`：型別 JSON 更新／無回報／多版本／相似名不同 ID／重複衝突／格式變化／注入名稱／API 失敗、UTF-8／UTF-16 BOM／壞編碼，及真實自有 Node 子程序逾時、失敗、取消、清理通過。
- 固定 typed COM helper 以 pinned Microsoft.WindowsPackageManager.ComInterop 1.29.380 / CsWinRT 2.3.1 / self-contained .NET8 建置成功；補上配對官方 WinMD 後，本機非管理員查詢 COMPLETE、41 個可信產品、18 個 API 更新候選。此數字在 Rust 保守數字版本驗證之前，不代表最終 UI 確認更新數。
- 本機 Registry 共 97 筆觀察、WinGet export 41 筆。實際完整 adapter / Rust / native UI 的最終驗證結果需由後續紀錄補上。
- Renderer 16 項合成 IPC 測試、正式 production build、SQLite schema 6 項已通過。最終版 Rust／Clippy 需重新確認 Windows CI，不能使用被取代的表格版本 CI 宣稱最終成功。

先前表格解析因独立 Review 指出的名稱注入與 stdout 來源失敗警告問題而被移除。先前 19 個表格更新候選數字不是交付證據；正式掃描不解析任何人類可讀 WinGet 表格。

## 更新資料語意與限制

更新來源是 WinGet 官方 catalog（https://cdn.winget.microsoft.com/cache）。UI 提供精確 ID 與官方 winget-pkgs manifest 位置，使用者可核對 vendor installer 來源。不從不可信文字提取任意下載網址，不執行下載／安裝／更新。

COM helper 驗證來源 Id/Name/Type/Argument；直接讀取 package 物件的 InstalledVersion、IsUpdateAvailable、DefaultInstallVersion，並用 CompareToVersion 檢查目標版本。Connect/Search 失敗與截斷結果回報 ERROR。資料透過 JSON，不依賴 stdout 語系或欄位排版；不接受協議、不修改來源、不要求管理員。官方 catalog 可能使用快取，NO_UPDATE_REPORTED 不保證最新；pin／多重安裝限制明示。

只有 export 與 COM 的精確 ID／安裝版本一致時才關聯。Rust 對可比較且增加的數字點分版本確認 UPDATE_AVAILABLE，其他版本 grammar UNKNOWN。Registry、VS Code、Node、npm 不透過名稱猜測 WinGet 身分；没有可信 catalog 證據時 UNKNOWN。WinGet 缺失時保留其他資料及原因；已確認產品的查詢逾時／失敗標 ERROR。漏洞引擎尚未實作，NOT_CHECKED 獨立於更新狀態。

SQLite schema 維持 v3；不可變 JSON 快照新增 update 與 vulnerability_status。舊 JSON 讀為 UNKNOWN / NOT_CHECKED，不重寫歷史。

## 尚待驗證

- 本機 Rust 工具鏈未安裝；最終 Rust 測試／Clippy、Tauri EXE／MSI build 由 Windows CI 驗證。
- 最終 production EXE／MSI 尚未安裝，Rust / native UI 整合驗證待取得本次 compiled artifact。不能以 PowerShell 合成測試或舊 0.1.0 證據代替。
- 使用者 Windows 11 安裝、實際已知更新產品的版本／來源核對、取消、再掃描、重啟、移除仍需正式包交付後驗收。
- Windows 10、其他架構、非數字版本 grammar、非 WinGet 目錄軟體均不保證更新涵蓋。
