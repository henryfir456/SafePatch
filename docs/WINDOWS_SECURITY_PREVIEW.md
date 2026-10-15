# SafePatch v0.3.0 Preview 安装與驗證

本文件描述 v0.3.0-preview 目標；只有 GitHub 對應 prerelease 出現下載項目才表示已發布。現階段實際驗證以 repository `docs/SECURITY_HEALTH_VERIFICATION.md` 為準，未執行的裝置驗收為 NOT VERIFIED。

EXE/MSI 是目前使用者 x64 安裝包，需既有 Microsoft WebView2，不需要 Rust/Node 開發環境。預覽版本尚未簽章（UNSIGNED），不應停用 Windows 安全政策。掃描不提升權限、不安裝／更新受測軟體；外部安全查詢需在 UI 明確勾選。

下載後先比對 `SHA256SUMS.txt`、`manifest.json` 中的包與 production SHA256、`source_commit` 和對應 final main SHA。EXE 檔名為 `SafePatch-Preview-0.3.0-preview-x64-setup.exe`；MSI 為 `SafePatch-Preview-0.3.0-preview-x64.msi`。MSI 數值版本 0.3.0；二進位/UI 完整版本為 0.3.0-preview。標題與底部顯示實際嵌入版本、短 SHA，「關於」顯示完整 SHA 和 Preview 通道。

先執行唯讀盤點，再在安全健檢頁明確允許查詢。NVD 僅接收受控 WinGet 產品 CPE／版本，OSV（若勾選 npm）僅接收所選快照的 registry 套件名稱／版本，CISA 公共 KEV 不需本機資料。專案路徑、內容、Publisher 及完整本機軟體清單不傳送。

MATCHED 才代表有充分證據支持該已安裝版本受影響。POTENTIAL／UNKNOWN 需查核；來源不足保持 PARTIAL，不代表全機安全。npm lockfile 只是宣告，不證明安裝。CVSS 描述嚴重度；KEV 只提高已確認適用漏洞的優先級。WinGet 有新版不表示新版已修復 CVE；NVD 上界也不是修復版本證明。

對於已安裝舊版或已有私人掃描資料的裝置，先確認版本及備份，避免以自動測試覆蓋使用者安裝或清除本機資料。移除程式不得刪除掃描歷史。Win10、人工可見檢查與無法執行的驗收維持 NOT VERIFIED。
