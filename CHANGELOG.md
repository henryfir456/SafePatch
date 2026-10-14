# CHANGELOG

## 0.2.0 Preview（2026-10-14）

- 新增唯讀 WinGet 更新健檢：以 Microsoft 簽章程式、官方目錄、型別化 WinGet COM API 與精確產品 ID／目前版本，顯示可用目標版本、來源與查詢時間。
- 更新狀態分為 UPDATE_AVAILABLE、NO_UPDATE_REPORTED、UNKNOWN、ERROR。未回報更新不保證最新；漏洞風險獨立標示「未檢查」。
- 首頁優先呈現需要更新的軟體、摘要、搜尋與狀態篩選；完整盤點及 npm 功能移至次要頁籤。
- 保留多版本與候選重複，不以相似名稱關聯；無法比較版本、缺少可信比對、格式變化及失敗均保守處理。
- 更新證據與既有 SQLite 掃描快照一起保存；舊快照保持 UNKNOWN，不會被視為最新或安全。
- 不安裝／更新任何軟體、不要求管理員權限、不上傳本機私人清單。Python PR #21 仍暫停。
- 驗證與交付狀態以 docs/UPDATE_HEALTH_VERIFICATION.md 及 Issue #26 為準；本條目不代表未進行的 UI 或安裝實測已通過。
