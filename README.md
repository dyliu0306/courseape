# CourseApe — CYCU 選課規劃工具

> 給中原大學學生與 AI Agent 的選課輔助系統。用自然語言提問，Agent 幫你分析畢業門檻、規劃下學期、檢查衝堂。

## 它能做什麼？

- **畢業門檻分析** — 你還缺哪些課、哪些學分
- **下學期規劃** — 自動匹配重修課程、推薦選課組合
- **衝堂檢查** — 排好的課表有沒有時間衝突
- **課程評價搜尋** — AI 搜尋公開網路上的課程心得

## 安裝

### 1. 安裝 CourseApe Skill（讓 AI Agent 能使用）

在你的 AI Agent 環境中執行：

```
courseape skills install --all
```

或指定平台：

```
courseape skills install claude
courseape skills install opencode
courseape skills install codex
```

### 2. 安裝 CLI（Skill 的後端引擎）

```
npm install -g @dyliu0306/courseape
```

> 需要 [Node.js](https://nodejs.org/) v18+。安裝時一路按「下一步」即可。

## 30 秒 Quick Start

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
| AI 分析 | 預設去識別化（隱藏學號、姓名） |
| 網路 | 只連學校伺服器（itouch.cycu.edu.tw） |

## 限制

- 成績分析需要 AI Agent 讀取 PDF，準確度取決於 PDF 格式與 Agent 能力
- 課程資料來自學校系統，非即時更新
- 通識向度判斷依賴歷史開課資料的 `OP_TYPE` 欄位，部分課程可能缺失
- 本工具為非官方開源專案，與中原大學無關

## 授權

[PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0) — 僅限非商業用途。

## 進階使用

- [CLI 完整參考](docs/cli-reference.md) — 所有低階命令與篩選條件
- [Agent 指南](docs/agent-guide.md) — Agent 支援範圍與故障排除
- [隱私政策](docs/privacy.md) — 帳密、session、成績與 Agent 資料邊界
- [開發指南](CONTRIBUTING.md) — 建置、測試、發布流程
