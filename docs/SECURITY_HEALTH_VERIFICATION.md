# v0.3 安全健檢驗證記錄

追蹤 #28、#29；版本 PR #30、安全 PR #31。Python PR #21 維持暫停。發布狀態與 final main SHA 以 [v0.3.0-preview Release](https://github.com/henryfir456/SafePatch/releases/tag/v0.3.0-preview) 的 manifest、SHA256SUMS 及 CI 報告為準；本文件不將 PR 產物稱為發布包。

## 已執行的自動化驗證

- 實際主機：Windows 11 Pro 10.0.26300 build 26300，目前使用者 token 非管理員。前端正式建置 PASS；26 個 UI、7 個 SQLite schema、3 個版本一致性/負向測試 PASS。
- PowerShell Registry adapter 13、WinGet 更新證據負向、VS Code 29、Node 29、Preview 合約/編碼/失敗、模組挾持回歸 PASS。
- 安全分支 Windows CI [38518442103](https://github.com/henryfir456/SafePatch/actions/runs/38518442103) 與最新 UI 分支 [38520141926](https://github.com/henryfir456/SafePatch/actions/runs/38520141926) 全部成功。76 個 Rust tests、Clippy、EXE/MSI、真正 MSI table-reader fixture、內嵌 metadata 與 payload 檢查均通過；restricted installer lifecycle 另列 NOT VERIFIED。
- 三個獨立 reviewer 已審閱版本、安全適用性與最終交付。發現的 MSI 數值版本、dirty freshness、SemVer build metadata、NVD negate/NA 證據缺口已修正並加回歸。最終 source review 無剩餘具體 P0/P1/P2 阻塞；來源測試仍不取代實機驗收。
- 網路負向測試包含 503/429、重導拒絕、超量回應、逾時、取消、過期/未來快取、錯誤資料型別、複雜 CPE、無效 OSV events、KEV 失敗維持 UNKNOWN。

## 真實 Windows 安全掃描

核對 GitHub 原生 artifact 11873297138 的 archive SHA256 `41cd6330f218838f51e0392fe57c78f65d47e54435e54ef9822594496952d162` 後，在本機執行。該 artifact 來源為 PR merge-preview `49c3138c9f740c23fb3770747a09aaf84e52d84c`，不是 final-main 發布包。二進位內嵌 `0.3.0-preview`、Preview、dirty=false。

- 真實 service/SQLite 探針以本專案既有 npm lockfile 加 Windows 盤點產生 305 個項目；NVD COMPLETE 1、OSV COMPLETE 79、OSV COVERAGE_PARTIAL 80。查詢上限正常降級，狀態 PARTIAL；取消及重啟保存 PASS。沒有以 fixture 或樣本填入裝置掃描。
- 隔離原生 WebView 安全 UI：146 個真實 Windows 項目／146 列完全對齊 Rust/SQLite，18 個一般更新、128 個優先級未定；NVD COMPLETE 1。標題／頁尾／關於 metadata、未檢查、同意、取消不發布結果、優先級排序、版本欄位、詳情開啟與重啟保存 PASS。scan ID `278a1c77-6257-4505-b25f-9455ceb9c06f`。
- 本次沒有 MATCHED 項目，不表示全機安全，也未將目前版本無命中等同 NOT_AFFECTED。KEV 提升與正確受影響版本以自動化正負向案例驗證，未捏造本機 KEV 命中。
- 先前版本階段 artifact 11872172545（SHA256 `c98366fe5b25240cc7db582cc87b0673d4224d54e098c8be71138f3558ff0527`）已在本機通過真實盤點／取消／重啟，當時 144 項／18 更新／106 更新未知。這些數字是各次裝置實測，不是產品常數。

完整清單、路徑、原始 DB 均留在本機忽略目錄；公開證據只有聚合值。隔離資料庫不存取既有 SafePatch 私人資料。

## 安裝與環境限制

本機無 MSVC link.exe，因此本機 Rust 編譯 NOT VERIFIED；實際執行的是成功 Windows CI 編譯並核對 digest 的原生程式。Windows Installer fixture 在乾淨 CI 通過，本機腳本明確回傳 PREVIEW_CI_ONLY，未當作本機安裝成功。

Windows Server restricted-child lifecycle 由 CI 報告保留 NOT_VERIFIED/RESTRICTED_CHILD_UNAVAILABLE。本機已有 SafePatch 使用者資料，未以自動安裝覆蓋；EXE/MSI 安裝、升級／移除後資料保存、Windows 10、人工可見 UI 驗收仍為 **NOT VERIFIED**。未完成項目不因建置成功而改為 PASS。

## 判定及資料範圍

NVD 僅接受官方 WinGet catalog ID 精確對應受控產品：Google.Chrome、Mozilla.Firefox、7zip.7zip、Microsoft.VisualStudioCode、OpenJS.NodeJS／LTS。Registry 名稱、PE 名稱或任意 ID 不建立確定匹配。CPE product、數值版本邊界、簡單 OR 無附加平台/版別條件才確認；複雜、否定、字串版本保持 POTENTIAL。未觀測的 CPE '-' NA 不視為 '*' ANY。受控映射保留證據，不宣稱真實安裝來源。

OSV 僅查選定 npm 快照中 registry 宣告的精確名稱／版本，支援 SemVer 範圍與枚舉版本。lockfile 不證明已安裝，命中仍為 POTENTIAL；不支援的範圍/來源保留未知。修復版本僅來自 OSV 明列 fixed 事件，不將 NVD 上界或 WinGet 新版本推算為修復。

官方公告保留來源提供的 HTTPS 參考連結，未實作通用官方公告解析器。CISA KEV 只在 MATCHED 時提高優先級；列入日期及美國聯邦措施期限不代表此主機修補期限。找不到漏洞不是安全。

背景來源查詢：OSV package+version、NVD CPE+觀測版本、CISA 公共目錄。固定 HTTPS、禁止重導/代理、8 MiB 回應、20 秒請求／8 秒連線、NVD 每 6.2 秒／其他每 250 ms、24 小時快取、最多 80 個新查詢／5 分鐘、16 MiB 總證據容量。超限／逾時／來源失敗及不支援情況為 UNKNOWN／PARTIAL，過期快取不產生安全判定。完整本機清單、路徑、Publisher、專案內容不送出。

## 發布閘門與後續驗收

只有審查及 CI 通過才合併；只有 final main SHA 的成功 push CI 產物可供 explicit v0.3.0-preview tag 工作流程發布。工作流程核對 artifact digest、每個 payload SHA256、嵌入 metadata、tag/main 未變更，draft 上傳後重新下載比對才公開 prerelease。PR artifact 不供正式發布。下載包應以實際 Release 的 manifest 與 checksum 再核對。

後續優先在乾淨 Windows 10/11 使用者環境驗收 EXE/MSI 安裝／升級／移除及可見 UI，並擴充有可信產品識別證據的漏洞覆蓋；不得將現有限制隱藏為安全。
