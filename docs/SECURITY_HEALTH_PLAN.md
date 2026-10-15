# v0.3 安全健檢設計與執行計畫

追蹤：#28（#10–#13）、#29（#16）。Python PR #21 維持暫停。

## 設計

沿用 Rust 背景服務與單一掃描槽；新增 security 範圍，引用使用者已完成的 Windows/npm 快照而不傳送完整清單。使用者勾選外部查詢後才開始。安全結果單獨持久化，取消或失败不覆蓋上次完成结果。每個檢查保留獨立來源狀態、時間、證據、適用性、優先級。UI 預設安全風險清單，保留更新及盤點頁籤。

OSV 只查 npm 公開 registry 宣告的精確名稱與版本；lockfile 不當作已安裝證明。NVD 僅查版本化受控 WinGet ID/CPE 表，Registry 名称不建立匹配。僅單純 OR、無平台附加條件且精確 CPE 產品／版本範圍可比較時確認；複雜 AND、否定、平台或未知版本保留 POTENTIAL。NVD 不提供確切修復版本時不推算。CISA KEV 是利用與優先級證據，不是適用性證據。

固定 HTTPS 端點、禁止重導、8 MiB 回應、20 秒請求超時、5 分鐘總掃描界限、來源節流、每掃描查詢上限、24 小時快取。取消中止請求及後續處理。過期快取不得轉成安全結論。來源失敗/截斷/不支援為 PARTIAL。官方公告僅保留驗證 HTTPS 參照，不自動抓取任意遠端網址。

單一 release.json 定義版本與通道，package/Cargo/Tauri 為受測試檢查的鏡像。Rust build.rs 檢查一致性，建置時嵌入 Git SHA（無 Git 明確 unavailable）；CI 檢查 SHA/tag。UI 透過 IPC 取得二進位中繼資料，不以字串假冒版本。不提供沒有可靠來源的 Build 時間。

## 分階段執行

1. #29：版本來源、build metadata IPC、標題/footer/about、一致性/負向測試，獨立 PR。
2. #28：純資料判定與負向範圍測試；OSV/NVD/KEV 受限非同步客戶端、快取/取消；SQLite 快照與服務 IPC。
3. #28：安全清單／排序／詳情、三維 UNKNOWN 去重與原因篩選、更新網址移入詳情；測試與繁體中文 CHANGELOG。
4. 全部回歸、獨立 reviewer、修正 blocking、Windows 實際背景服務及 UI/安裝版本證據。通過審查與 CI 才合併，final main CI 的 EXE/MSI 通過既有閘門才發布 prerelease。

## 必測與審查重點

- SEMVER 邊界、重複 aliases、不一致或不支援範圍、撤回公告、同名不同產品、CPE 複雜布林條件。
- KEV 只升級 MATCHED；沒有來源資料不得稱安全；不同更新版本不能代替安裝版本。
- 查詢失敗／取消／過期／查詢上限不隱藏未知；npm 宣告不提升成已安裝漏洞。
- 三维统计各自 unique item IDs；多理由不增加總數；CVEs 別名去重。
- 重啟保留安全證據、取消後保留上次結果、現有 schema/盤點測試不退步。
- CI、原生探針、WebView、手動與 installer 驗收分開；工具阻塞紀錄 NOT VERIFIED。
