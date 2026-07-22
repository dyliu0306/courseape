# CourseApe — CYCU 選課規劃工具

[![npm version](https://img.shields.io/npm/v/@dyliu0306/courseape)](https://www.npmjs.com/package/@dyliu0306/courseape)
[![CI](https://github.com/dyliu0306/courseape/actions/workflows/ci.yml/badge.svg)](https://github.com/dyliu0306/courseape/actions/workflows/ci.yml)
[![License: PolyForm NC 1.0.0](https://img.shields.io/badge/License-PolyForm--Noncommercial--1.0.0-blue)](LICENSE)

> 給中原大學學生與 AI Agent 的選課輔助系統。用自然語言提問，Agent 幫你分析畢業門檻、規劃下學期、檢查衝堂。

## 它能做什麼？

- **畢業門檻分析** — 你還缺哪些課、哪些學分
- **下學期規劃** — 自動匹配重修課程、推薦選課組合
- **衝堂檢查** — 排好的課表有沒有時間衝突
- **課程評價搜尋** — AI 搜尋公開網路上的課程心得

## 安裝

### 0. 由 Agent 自動安裝

在 AI Agent（OpenCode、Codex、Claude Code 等）的對話中貼上：

```
幫我安裝 https://github.com/dyliu0306/courseape
並告訴我怎麼登入？能用來做甚麼？
```

Agent 會自動完成 npm 安裝、Skill 安裝，並引導你使用。你仍須在cmd內自行登入一次。

**選擇這個安裝方式的話，可以跳過以下步驟。**

### 1. 安裝 CLI

```
npm install -g @dyliu0306/courseape
courseape --version
```

> 需要 [Node.js](https://nodejs.org/) v18+。安裝時一路按「下一步」即可。
>
> **Windows 用戶**：安裝時可能觸發 Windows Defender SmartScreen，選「仍要執行」即可。

如果 npm 不可用，也可以從 [GitHub Releases](https://github.com/dyliu0306/courseape/releases) 下載對應平台的 binary。Windows x64 使用 `courseape-win32-x64.exe`。

### 2. 設定帳密

首次使用前，設定 iTouch 帳密（二擇一）：

```
courseape credentials set          # 互動式輸入，存入系統鑰匙圈
```

或設定環境變數：

```
$env:CYCU_USERNAME = "你的學號"
$env:CYCU_PASSWORD = "你的密碼"
```

> 帳密存在系統鑰匙圈（Windows 認證管理員 / Mac 鑰匙圈），不出現在任何檔案。

### 3. 安裝 Agent Skill

CLI 安裝完成後，在你的 AI Agent 環境中執行：

```
courseape skills install --all
```

或指定平台：

```
courseape skills install claude
courseape skills install opencode
courseape skills install codex
```

## Quick Start

安裝完成後，在 AI Agent 裡說：

```
幫我設定 CourseApe
```

Agent 會自動：
1. 引導你完成一次安全登入（iTouch 帳密）
2. 從學號推導你的入學年度
3. 詢問確認你的系所（例如「你是資訊管理學系學士班嗎？」）
4. 同步必要的基礎資料

設定完成後，直接用自然語言提問：

```
我還缺什麼課才能畢業？
下學期要選什麼課？
幫我查資管系這學期的必修
```

## 使用範例

| 你說 | Agent 做的事 |
|------|-------------|
| 「幫我分析畢業門檻」 | 讀取修業辦法 PDF + 成績，交叉比對，告訴你缺什麼 |
| 「下學期要選什麼課」 | 分析缺修課程 → 查詢開課 → 排除衝堂 → 建議選課組合 |
| 「有沒有衝堂」 | 檢查備選清單的時間衝突 |
| 「這堂課好不好」 | 搜尋網路上的課程與教師評價 |
| 「資管系有什麼課」 | 篩選並顯示該系開課清單 |

## 它不會做什麼

- **不會幫你選課** — 實際選課還是要到學校選課系統
- **不會儲存你的密碼在檔案中** — 帳密存在系統鑰匙圈（Windows 認證管理員 / Mac 鑰匙圈）
- **不會上傳你的成績** — 所有資料都在本機

## 隱私

| 項目 | 處理方式 |
|------|----------|
| 帳密 | 系統鑰匙圈，不出現在任何檔案 |
| 成績 | 本機 SQLite 資料庫 |
| AI 分析 | CLI 預設遮罩個資；若 Agent 將資料送給外部 AI，仍受該 Agent 服務的隱私政策約束 |
| 網路 | CourseApe 連線至 CYCU iTouch、cmap；每 4 小時最多向 npm registry 查詢一次最新版本；npm 安裝／更新時亦會連線 |

## 限制

- 成績分析需要 AI Agent 讀取 PDF，準確度取決於 PDF 格式與 Agent 能力
- 課程資料來自學校系統，非即時更新；開課資料 TTL 4 小時、系所清單 24 小時
- 通識向度判斷依賴歷史開課資料的 `OP_TYPE` 欄位，部分課程可能缺失
- 本工具為非官方開源專案，與中原大學無關

## 快速驗證（新手 5 命令）

```bash
courseape --version
courseape agent doctor
courseape init --department "資管系"
courseape agent prepare graduation
courseape agent prepare planning
```

## 疑難排解

| 問題 | 解法 |
|------|------|
| 首次登入報錯 | 先執行 `courseape credentials set` 設定帳密 |
| 登入失敗 | `courseape credentials set` 重新設定帳密，或設定環境變數 `CYCU_USERNAME` / `CYCU_PASSWORD` |
| Session 過期 | `courseape login` 重新登入 |
| 系所找不到 | `courseape agent resolve "你的系所"` 查看候選清單 |
| 開課資料過舊 | `courseape courses offerings --term 1151` 重新同步 |
| PDF 驗證失敗 | `courseape data purge` 清除快取後重跑 `prepare graduation` |
| Skill 安裝後 Agent 找不到 | 確認 `courseape skills install <平台>` 的平台名稱正確 |
| 版本不一致 | `npm install -g @dyliu0306/courseape@latest` 重新安裝 |

## 授權

[PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0) — 僅限非商業用途。

## 進階使用

- [CLI 完整參考](https://github.com/dyliu0306/courseape/blob/master/docs/cli-reference.md) — 所有低階命令與篩選條件
- [Agent 指南](https://github.com/dyliu0306/courseape/blob/master/docs/agent-guide.md) — Agent 支援範圍與故障排除
- [隱私政策](https://github.com/dyliu0306/courseape/blob/master/docs/privacy.md) — 帳密、session、成績與 Agent 資料邊界
- [開發指南](https://github.com/dyliu0306/courseape/blob/master/CONTRIBUTING.md) — 建置、測試、發布流程

## 更新

CourseApe 每 4 小時最多向 npm 檢查一次最新版；若有新版本，會在 stderr 顯示更新提示，不影響命令輸出或 Agent JSON。

```bash
npm update -g @dyliu0306/courseape
courseape --version
```

更新後若出現 platform package 錯誤，重新安裝：

```bash
npm install -g @dyliu0306/courseape
```
