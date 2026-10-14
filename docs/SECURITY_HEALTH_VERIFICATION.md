# v0.3 安全健檢驗證記錄

追蹤 #28、#29；版本 PR #30；Python PR #21 暫停。此版本尚未發布。

## 已執行證據

- 本機 Windows（SER 使用者目前專案）：前端正式建置 PASS、24 個 UI 回歸 PASS、7 個 SQLite schema 回歸 PASS、3 個版本一致性/負向合約 PASS。
- 新 UI 測試包含外部查詢同意／取消、CRITICAL→HIGH→MEDIUM→一般更新→UNKNOWN 排序、安裝與目標版本、原因/維度去重、舊快照警示。
- #29 首次 Windows Server CI 38500851387：Rust unit tests 與 Clippy PASS；正式 bundle 因來源 dirty 閘門拒絕。增加排除 Tauri 生成 schemas／隔離 acceptance 輸出並重新觸發 CI；未將失敗當作成功。
- 獨立版本 reviewer：發現 MSI prerelease 数值相容性與增量 dirty freshness，已修正。獨立安全 reviewer 發現 SemVer build metadata 比較及 NVD 否定條件排除證據問題，已加入修正/回歸案例；Rust 執行待 CI。

## Windows 執行限制

沙箱命令啟動器在 D:\ 磁碟列舉階段失敗。核准的命令可在實際主機讀取及執行 Node/PowerShell。官方 SHA256 核對的 Rust 僅安裝於忽略的 `.local`，未改系統 PATH；本機缺少 MSVC `link.exe`，Rust 編譯明確失敗，未聲稱通過。

真實本機漏洞資料源掃描、原生 WebView 安全 UI／取消／重啟、EXE/MSI 安裝與內嵌版本：**NOT VERIFIED**。沒有以測試 fixture 冒充本機漏洞。Windows 10 及人工可見驗收未完成。

## 判定及資料範圍

NVD 僅接受官方 WinGet catalog ID 精確對應受控產品：Google.Chrome、Mozilla.Firefox、7zip.7zip、Microsoft.VisualStudioCode、OpenJS.NodeJS／LTS。Registry 名稱、PE 名稱或任意 ID 不建立確定匹配。CPE product、數值版本邊界、簡單 OR 無附加平台/版別條件才確認；複雜、否定、字串版本保持 POTENTIAL。CPE 名稱與官方 manifest 產品關聯是明列於證據中的受控映射，不宣稱真實安裝來源。

OSV 僅查已選定 npm 快照中 registry 宣告的精確名稱／版本，支持 SemVer 範圍和枚舉版本。鎖定檔不證明套件已安裝，因此版本範圍命中仍為 POTENTIAL；不支持的範圍/來源保留未知。修復版本僅来自 OSV 明列 fixed 事件，不將 NVD 上界或 WinGet 新版本推算為修復。

官方公告保留來源提供的 HTTPS 參考連結，未實作通用官方公告解析器。CISA KEV 只在適用性 MATCHED 時提高優先級；列入日期及美國聯邦措施期限不代表此主機修補時限。找不到漏洞不是安全。

背景來源查詢：OSV package+version、NVD CPE+觀測版本、CISA 公共目錄。固定 HTTPS、禁止重導/代理、8 MiB 回應、20 秒請求／8 秒連線、NVD 每 6.2 秒／其他每 250 ms、24 小時快取、最多 80 個新查詢／5 分鐘。超限／逾時／來源失敗及不支持情況為 UNKNOWN／PARTIAL，過期快取不產生安全判定。完整本機清單、路徑、Publisher、專案內容不送出。

## 尚需通過的交付閘門

1. 安全 Rust tests/Clippy、失敗/逾時網路與取消實測、完整 Windows CI；修正所有 blocking 審查。
2. 原生實際掃描與安裝後版本/Build SHA；保留本機清單於本機，只公開聚合證據。
3. 審查及 CI 通過才合併；final main SHA 的成功 CI 產物才發布 EXE/MSI，下載 bytes/hash 重驗。尚無 v0.3 發布包或 SHA256 可提供。
