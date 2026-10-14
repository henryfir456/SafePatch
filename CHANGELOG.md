# CHANGELOG

## 0.3.0 Preview（開發中，尚未發布）

- 新增獨立唯讀安全掃描：明確啟用後，以受控產品 CPE／版本查詢 NVD，以 npm registry lockfile 宣告名稱／版本查詢 OSV，使用 CISA KEV 補充已遭利用證據。
- 漏洞適用性 MATCHED／POTENTIAL／NOT_AFFECTED／UNKNOWN 與修補優先級 CRITICAL／HIGH／MEDIUM／INFORMATIONAL／UNDETERMINED 分開。KEV 只提升已確認適用項目；不以名稱、CVSS 或有新版直接斷言存在漏洞／已修補。
- 首頁顯示緊急修補、高風險、一般更新、未確認與安全掃描時間；尚未執行顯示「未檢查」。結果含版本、CVE、範圍、來源、查詢時間、快取狀態、KEV 措施及判定證據，依優先級排序。
- UNKNOWN 身分、更新、漏洞分维度依 ID 去重與原因篩選；WinGet 冗長網址移至更新來源詳情。
- 主畫面、底部及「關於」顯示二進位實際內嵌版本、Build SHA 與發布通道；單一 package.json 版本來源與 Cargo／Tauri／lockfile／MSI 數值版本對齊檢查。
- 最小必要外部查詢、固定 HTTPS 端點、不重導、限流、逾時、快取與取消；安全快照與既有盤點歷史獨立保存。來源或覆蓋不足保持 PARTIAL。
- npm 宣告不視為安裝證明；Windows 產品對應僅支援受控清單；不自動抓取任意公告、不自動更新或提權。Python PR #21 維持暫停。
- 實際驗證、剩餘阻塞與發布狀態參見 docs/SECURITY_HEALTH_VERIFICATION.md、#28、#29。未通過的原生/安裝驗收維持 NOT VERIFIED。

## 0.2.0 Preview（2026-10-14）

- 新增唯讀 WinGet 更新健檢：以 Microsoft 簽章程式、官方目錄、型別化 WinGet COM API 與精確產品 ID／目前版本，顯示可用目標版本、來源與查詢時間。
- 更新狀態分為 UPDATE_AVAILABLE、NO_UPDATE_REPORTED、UNKNOWN、ERROR。未回報更新不保證最新；漏洞風險獨立標示「未檢查」。
- 首頁優先呈現需要更新的軟體、摘要、搜尋與狀態篩選；完整盤點及 npm 功能移至次要頁籤。
- 保留多版本與候選重複，不以相似名稱關聯；無法比較版本、缺少可信比對、格式變化及失敗均保守處理。
- 更新證據與既有 SQLite 掃描快照一起保存；舊快照保持 UNKNOWN，不會被視為最新或安全。
- 不安裝／更新任何軟體、不要求管理員權限、不上傳本機私人清單。Python PR #21 仍暫停。
- 驗證與交付狀態以 docs/UPDATE_HEALTH_VERIFICATION.md 及 Issue #26 為準；本條目不代表未進行的 UI 或安裝實測已通過。
