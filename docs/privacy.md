# 隱私政策

## 帳密儲存

CourseApe 使用獨立的 OS 鑰匙圈條目（`courseape/cycu-itouch`）。

- **Windows**：認證管理員（Credential Manager）
- **macOS**：鑰匙圈（Keychain）
- **Linux**：Secret Service（GNOME Keyring / KWallet）

帳密**不會**出現在：
- 任何檔案（設定檔、日誌、快取）
- 環境變數（登入後）
- Git repository
- npm 套件

### 環境變數（僅限非互動模式）

`CYCU_USERNAME` 和 `CYCU_PASSWORD` 只在以下情況使用：
- CI/CD 自動化測試
- 首次登入時，從環境變數讀取並存入鑰匙圈

登入成功後，環境變數不再被讀取。

## Session

- Session cookie 儲存在獨立 OS 鑰匙圈條目 `courseape/cycu-itouch-session`
- 舊版 `session.json` 會在讀取時遷移到鑰匙圈並刪除
- 不會上傳到任何伺服器
- 執行 `courseape logout` 可清除

## 成績與課程資料

- SQLite 儲存 profile、系所、開課資料、分析成績與 requirements metadata
- 原始成績 HTML、修業辦法 PDF、API snapshots 儲存在 snapshots 目錄
- 路徑：`~/.local/share/courseape/courseape.db`（Linux）或 `%APPDATA%\courseape\courseape.db`（Windows）
- 不會上傳到任何伺服器

## AI 分析

- 預設啟用去識別化
- 去識別化會隱藏學號、姓名等個人資料
- 使用 `--no-redact-personal` 可關閉，但不建議

## 網路連線

CourseApe 只連線到：
- `itouch.cycu.edu.tw` — 登入、成績、開課查詢
- `cmap.cycu.edu.tw` — 課綱 PDF 下載
- `registry.npmjs.org` — npm 套件安裝／更新，以及每 4 小時最多一次的最新版檢查

## 清除所有資料

```bash
courseape data purge
```

這會刪除：登入 session、所有快取資料、成績、開課清單、系所清單、本機資料庫。

不會刪除鑰匙圈中的帳密。要連帳密也清除：

```bash
courseape logout --clear-credentials
```

## 資料邊界

| 資料 | 儲存位置 | 會上傳嗎 | 會分享給 Agent 嗎 |
|------|----------|----------|-------------------|
| 帳密 | 系統鑰匙圈 | 否 | 否 |
| Session | 系統鑰匙圈 | 否 | 否 |
| 成績 | 本機 SQLite + HTML snapshot | 否 | 是（去識別化後） |
| 修業辦法 PDF | 本機檔案 | 否 | 是 |
| 開課清單 | 本機 SQLite + API snapshot | 否 | 是 |
