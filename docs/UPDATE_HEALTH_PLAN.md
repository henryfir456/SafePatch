# v0.2.0 更新健檢設計與實作計畫 — Issue #26

目標：一次唯讀 Windows 掃描，提供可驗證產品 ID、目前／目標版本、可信來源與查詢時間；漏洞風險獨立為 NOT_CHECKED。

架構：既有 Microsoft 簽章 WinGet / 官方 source 驗證與 export JSON 盤點 -> 固定 C# helper 透過 WinGet COM API 查詢 -> 嚴格 JSON / 精確 ID + 版本關聯 -> Rust 版本驗證與正規化 -> 不可變 SQLite scan_assets JSON 快照 -> 型別 IPC -> React 更新優先介面。

helper 以鎖定 NuGet 官方 SDK / CsWinRT 建置，附 self-contained .NET 執行環境；EXE 與配對官方 WinMD 中繼資料嵌入 SafePatch，於每次掃描的自有暫存目錄輸出。無需使用者安裝 .NET、SDK 或 PowerShell 模組。使用已註冊的 Microsoft out-of-process COM 類別，驗證 catalog Id/Name/Type/Argument，再建立僅本地安裝查詢的官方 composite catalog。Connect/Search 失敗、截斷、過大輸出等失敗關閉；未接受來源協議、不修改／刷新來源、不呼叫 install/upgrade/download API。

COM 直接提供 InstalledVersion / IsUpdateAvailable / DefaultInstallVersion / CompareToVersion，名稱与版本為 JSON 字串，不解析人類可讀表格。查詢時間與來源證據保留；catalog 可使用快取，NO_UPDATE_REPORTED 不保證最新。COM 僅提供單一安裝版本，Registry 多重安裝仍獨立保留；未匹配版本 UNKNOWN。Rust 對增加的數字點分版本保守確認，其他 grammar UNKNOWN。

SQLite schema 維持 v3，update / vulnerability_status 擴充不可變 JSON 快照。舊快照 default UNKNOWN / NOT_CHECKED，不重寫歷史。

- [x] 固定、bounded / cancellable helper 查詢及 JSON 協定測試：格式／非語系依賴、字串注入、重複衝突、多版本、不可用、API 失敗、逾時。
- [x] Rust 獨立更新證據、保守版本驗證、來源拒絕、持久化重啟／舊快照測試。
- [x] 四張摘要卡、更新優先首頁、完整盤點次要頁籤、狀態／搜尋篩選、來源／版本／建議；React 資料一致性及篩選回歸。
- [x] Windows CI helper build 與 regression gates；native acceptance / installer lifecycle 頁籤與 timeout 修正；0.2.0 metadata / 繁中 CHANGELOG。
- [ ] 最終 Windows CI、獨立 Review、實際 Rust／native WebView 整合驗證。
- [ ] 通過 gate 才合併 main，建置／發布 digest 驗證的 Preview EXE／MSI；使用者 Windows 11 安裝實測另列。

審查焦點：相似名稱不同產品、順序查詢版本改變、COM 單版本涵蓋、來源／查詢失敗與來源快取、舊歷史不被視為最新／安全、嵌入 helper 的可信輸出與自有程序清理。Python PR #21 不合併。
