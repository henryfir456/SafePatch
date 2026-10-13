# 更新健檢驗證紀錄（Issue #26）

## 已執行的 Windows 11 adapter 驗證

2026-10-13，本機 Windows 11 Pro 10.0.26300.0、Windows PowerShell 5.1.26100.9444、Node 24.20.0。

- `scripts/test-winget-updates.ps1`：合成資料語系／UTF-8／UTF-16 BOM／壞編碼、更新／無回報／多版本／相似名不同 ID／重複衝突／格式變化，及真實自有 Node 子程序逾時、失敗、取消、清理通過。
- `scripts/verify-windows-adapter.ps1`：真實唯讀 Registry 97 筆觀察、WinGet export 41 筆；list 精確比對提供 19 個更新候選。此數字在 Rust 保守版本驗證之前，不代表最終 UI 確認更新數。
- `scripts/verify-adapter-pipe.mjs`：與 Rust 相同的固定 PowerShell stdin/stdout 傳輸，實際 150 筆觀察、9 個覆蓋項、stderr 0 bytes，JSON 67354 bytes。只输出合計與固定碼；私人盤點沒有提交。
- Renderer 15 項合成 IPC 測試、正式 production build、SQLite schema 6 項通過（後續 commit 需重新確認）。

## 更新資料語意與限制

更新來源是 WinGet 官方 catalog（https://cdn.winget.microsoft.com/cache）。UI 提供精確 ID 與官方 winget-pkgs manifest 位置，使用者可核對 vendor installer 來源。不從不可信 stdout 提取任意下載網址，不執行下載／安裝／更新。

WinGet `list --source winget --disable-interactivity` 本身唯讀，與 upgrade 列表共用 Microsoft 的 ReportListResult 版本／pin 評估。解析不認語系標題，只接受 export 的精確 ID／相同版本尾欄。只有可比較且增加的數字點分版本可確定 UPDATE_AVAILABLE；其他 grammar 是 UNKNOWN。部分列表可能截斷 ID、版本帶空格，或產品未被 export 涵蓋，該項保持 UNKNOWN。空列表或整份格式不支援是 ERROR。來源可能使用快取，NO_UPDATE_REPORTED 不保證最新；pin／多重安裝限制明示。

Registry、VS Code、Node、npm 觀察不透過名稱猜測 WinGet 身分；沒有可信 catalog 證據時 UNKNOWN。WinGet 缺失時保留其他資料及原因；查詢逾時／失败對已確認 WinGet 产品標 ERROR。漏洞引擎尚未實作，NOT_CHECKED 獨立於更新狀態。

SQLite schema 維持 v3；scan_assets 的不可變 JSON 快照新增 update 與 vulnerability_status。舊 JSON 由 serde default 讀為 UNKNOWN / NOT_CHECKED，不重寫歷史。

## 尚待驗證

- 本機未安裝 Rust 工具鏈，Rust 新測試／Clippy、Tauri native UI、EXE／MSI build 由 Windows CI 驗證；尚未取得本次 CI 結果。
- 本次 production EXE／MSI 尚未安裝與實機 UI 實測；不能以 PowerShell 合成測試或舊 0.1.0 證據代替。
- 使用者 Windows 11 安裝、實際已知更新產品的版本／來源核對、取消、再掃描、重啟、移除仍需正式包交付後驗收。
- Windows 10、其他架構、所有非數字版本 grammar、非 WinGet 目錄軟體均不保證更新涵蓋。
