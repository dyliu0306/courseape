# CourseApe — CYCU 選課輔助 CLI

> 幫中原大學學生分析畢業門檻、瀏覽開課、檢查衝堂的命令列工具。

---

## 這是什麼？

CourseApe 是一個安裝在電腦上的小工具。你在「命令提示字元」（Windows）或「終端機」（Mac）裡輸入指令，它就會幫你：

- **分析畢業門檻** — 自動下載你系上的修業辦法，告訴你還缺哪些課、哪些學分
- **瀏覽開課清單** — 查詢指定學期的所有課程，依系所、學分、老師、時段篩選
- **檢查衝堂** — 把你想選的課排在一起，自動抓出時間衝突
- **排課表** — 用表格顯示一週的課程安排
- **AI 課程評價搜尋** — 讓 AI Agent 幫你搜網路上的課程心得（需要搭配 AI 工具使用）

### 與選課系統的關係

| | 學校選課系統 | CourseApe |
|---|---|---|
| 功能 | 選課/退選 | 規劃、分析、比較 |
| 介面 | 網頁 | 命令列（打指令） |
| 用途 | 實際選課 | 選課前的準備工作 |

**CourseApe 不會幫你選課**，它只幫你做功課。實際選課還是要到學校選課系統。

---

## 免責聲明

本工具為**非官方開源專案**，與中原大學無關。請勿短時間發送大量請求，以免帳號被封鎖。課程規劃結果僅供參考，畢業資格請以學校官方為準。

---

## 安裝

### 你需要先準備

1. **Node.js**（版本 18 以上）
   - 到 https://nodejs.org/ 下載「LTS」版本並安裝
   - 安裝時一路按「下一步」即可
   - 安裝完成後重新開啟命令提示字元

2. **確認 Node.js 安裝成功**
   - 開啟命令提示字元（Windows 搜尋 `cmd`）
   - 輸入以下指令，應該會顯示版本號：
   ```
   node --version
   ```
   - 如果顯示 `v18.x.x` 或更高的數字就是 OK

### 安裝 CourseApe

在命令提示字元輸入：

```
npm install -g @dyliu0306/courseape
```

> **注意**：`@dyliu0306` 是占位符。實際指令會像 `npm install -g @yourname/courseape`，請依照發布頁面上的說明操作。

### 或者用 npx 免安裝執行

如果只是想試試看，不需要安裝：

```
npx @dyliu0306/courseape --help
```

### 安裝失敗？

| 狀況 | 解決方法 |
|------|----------|
| `npm 不是內部指令` | Node.js 沒裝好，重新安裝 Node.js |
| `EACCES` 權限錯誤 (Mac/Linux) | 指令前加 `sudo` |
| 網路錯誤 | 確認網路連線正常，公司/學校網路可能需要 proxy |

---

## 第一次使用：5 分鐘設定

打開命令提示字元，依序執行以下指令：

### 第 1 步：登入學校帳號

```
courseape login
```

系統會提示你輸入學號和密碼（就是 iTouch 的帳密）。輸入時密碼不會顯示在螢幕上，這是正常的。

登入成功後，帳密會存在電腦的「系統鑰匙圈」裡（Windows 的「認證管理員」、Mac 的「鑰匙圈」），**不會**存在任何檔案中。

### 第 2 步：設定個人資料

```
courseape profile edit
```

系統會問你：
- **入學年度**（例如 `112` 代表 112 學年入學）
- **系所代碼**（例如 `5400B` 是資管系）
- **學制**（學士/碩士/博士）

> 不知道系所代碼？先執行第 3 步，匯出的系所清單裡會有對照表。

### 第 3 步：同步系所清單

```
courseape sync departments --year 114
```

下載 114 學年的所有系所資料。

### 第 4 步：下載修業辦法

```
courseape sync requirements --year 0
```

`--year 0` 代表自動從你的學號推算入學年度。系統會下載你系上的畢業門檻 PDF。

### 第 5 步：下載歷年成績

```
courseape sync grades
```

從 iTouch 下載你的歷史成績。

設定完成！接下來就可以開始使用各項功能了。

---

## 使用情境

### 情境一：「我還差什麼課才能畢業？」

```
courseape status                  # 確認登入狀態
courseape profile show            # 確認個人資料正確
courseape sync requirements --year 0  # 下載最新修業辦法
courseape sync grades             # 下載成績
```

然後將修業辦法和成績交給 AI Agent 分析（需要安裝 AI Agent Skill，詳見下方「AI 分析功能」）。

### 情境二：「下學期有什麼課可以選？」

```
courseape courses offerings --term 1151    # 列出 1151 學期所有開課
```

加上篩選條件：

```
courseape courses filter --term 1151 --dept 5400B        # 只看資管系的課
courseape courses filter --term 1151 --credit 3          # 只看 3 學分的課
courseape courses filter --term 1151 --teacher 王        # 只看姓王老師的課
courseape courses filter --term 1151 --type 必修         # 只看必修
courseape courses filter --term 1151 --day 2 --period A  # 只看週二第 1-2 節
```

條件可以自由組合：

```
courseape courses filter --term 1151 --dept 5400B --type 必修 --credit 3
```

### 情境三：「這幾堂課會不會衝堂？」

先建立「備選清單」（shortlist），把有興趣的課加進去：

```
courseape shortlist add MI5001 --term 1151
courseape shortlist add MI5002 --term 1151
courseape shortlist add GE2001 --term 1151
```

然後檢查衝堂：

```
courseape courses conflicts --term 1151
```

查看完整課表：

```
courseape courses timetable --term 1151
```

管理備選清單：

```
courseape shortlist list --term 1151          # 查看清單
courseape shortlist remove MI5001 --term 1151 # 移除某堂課
courseape shortlist clear --term 1151         # 清空清單
```

### 情境四：「幫我自動規劃重修」

```
courseape courses plan --term 1151
```

系統會自動掃描你不及格的課程，在開課清單中找到對應的課程，加到備選清單。加 `--dry-run` 只預覽、不實際加入：

```
courseape courses plan --term 1151 --dry-run
```

### 情境五：「下載課綱 PDF」

```
courseape courses syllabus MI5001 --term 1151
```

會下載該課程的大綱 PDF 到本機。

### 情境六：「匯出資料給 AI 分析」

```
courseape data export --scope profile       # 匯出個人資料
courseape data export --scope departments   # 匯出系所清單
courseape data export --scope grades        # 匯出已分析的成績
courseape data export --scope grade-html    # 匯出原始成績 HTML（給 AI 分析用）
courseape data export --scope offerings     # 匯出歷史開課清單（含通識向度）
```

---

## 完整指令一覽

### 登入與帳號

| 指令 | 說明 |
|------|------|
| `courseape login` | 登入 iTouch，session 會存起來 |
| `courseape status` | 查看目前登入狀態 |
| `courseape logout` | 登出（保留鑰匙圈中的帳密） |
| `courseape logout --clear-credentials` | 登出並清除帳密（**會影響 openape**） |
| `courseape credentials set` | 更新學號密碼（與 openape 共用） |

### 個人資料

| 指令 | 說明 |
|------|------|
| `courseape profile show` | 查看個人資料（預設去識別化） |
| `courseape profile show --no-redact-personal` | 查看完整資料（含學號等） |
| `courseape profile edit` | 修改入學年度、系所、學制 |

### 資料同步

| 指令 | 說明 |
|------|------|
| `courseape sync departments --year 114` | 同步該學年系所清單 |
| `courseape sync requirements --year 112` | 下載 112 學年入學的修業辦法 |
| `courseape sync requirements --year 0` | 自動推算入學年度並下載修業辦法 |
| `courseape sync grades` | 下載歷年成績 HTML |

### 課程查詢

| 指令 | 說明 |
|------|------|
| `courseape courses offerings --term 1151` | 列出該學期所有開課 |
| `courseape courses filter --term 1151 [條件]` | 篩選課程（見下方篩選條件表） |
| `courseape courses conflicts --term 1151` | 檢查備選清單的衝堂 |
| `courseape courses timetable --term 1151` | 顯示一週課表 |
| `courseape courses syllabus <代碼> --term 1151` | 下載課綱 PDF |
| `courseape courses plan --term 1151` | 自動規劃重修課程 |
| `courseape courses history` | 同步所有歷史學期的開課資料 |

#### 篩選條件一覽

所有條件可以自由組合，全部都是「且」的關係。

| 條件 | 說明 | 範例 |
|------|------|------|
| `--dept <代碼>` | 系所（AUTHORITY_DEPT 代碼） | `--dept 5400B` |
| `--class_dept <代碼>` | 班級 | `--class_dept 5431B` |
| `--keyword <文字>` | 課程名稱（中英文皆可） | `--keyword 資管` |
| `--code <代碼>` | 課程代碼前綴 | `--code MI` |
| `--teacher <姓名>` | 教師姓名 | `--teacher 劉` |
| `--teacher_id <代碼>` | 教師人事代碼 | `--teacher_id 12508` |
| `--type <必修\|選修>` | 必修或選修 | `--type 必修` |
| `--credit <數字>` | 學分數 | `--credit 3` |
| `--div <B\|M\|D\|H>` | 部別（學士/碩士/博士/學士後） | `--div B` |
| `--language <文字>` | 授課語言 | `--language 英語` |
| `--day <1-7>` | 星期幾（1=週一） | `--day 2` |
| `--period <代碼>` | 節次（1-8 或 A-G） | `--period A` |
| `--classroom <文字>` | 教室 | `--classroom 管理` |
| `--general <類別>` | 通識向度 | `--general 基礎天` |
| `--emi` | 只看全英語授課 (EMI) | `--emi` |
| `--english` | 只看英語授課 | `--english` |
| `--distance` | 只看遠距教學 | `--distance` |
| `--pbl` | 只看 PBL 課程 | `--pbl` |
| `--programming` | 只看程式設計課程 | `--programming` |
| `--available` | 只看有餘額的課 | `--available` |
| `--semester <全學期\|半學期>` | 期程 | `--semester 半學期` |
| `--cross` | 只看跨系/聯盟課程 | `--cross` |
| `--sdgs <文字>` | SDGs 目標 | `--sdgs SDGS` |

### 備選清單

| 指令 | 說明 |
|------|------|
| `courseape shortlist add <代碼> --term 1151` | 加入備選清單 |
| `courseape shortlist remove <代碼> --term 1151` | 從清單移除 |
| `courseape shortlist list --term 1151` | 查看清單 |
| `courseape shortlist clear --term 1151` | 清空清單 |

### 資料匯出與匯入

| 指令 | 說明 |
|------|------|
| `courseape data export --scope profile` | 匯出個人資料 |
| `courseape data export --scope departments` | 匯出系所清單 |
| `courseape data export --scope grades` | 匯出已分析的成績 |
| `courseape data export --scope grade-html` | 匯出原始成績 HTML |
| `courseape data export --scope offerings` | 匯出歷史開課（含通識向度） |
| `courseape data import --scope grades --file <檔案>` | 匯入 AI 分析的成績 JSON |
| `courseape data purge` | 清除所有快取與 session（保留鑰匙圈帳密） |

### Skills（AI Agent 整合）

| 指令 | 說明 |
|------|------|
| `courseape skills install claude` | 為 Claude Code 安裝 Skill |
| `courseape skills install opencode` | 為 OpenCode 安裝 Skill |
| `courseape skills install codex` | 為 Codex 安裝 Skill |
| `courseape skills install --all` | 自動偵測並安裝到所有已安裝的 Agent |
| `courseape skills show` | 查看 SKILL.md 內容 |

### 全域選項

這些選項可以加在任何指令後面。

| 選項 | 說明 | 預設值 |
|------|------|--------|
| `--output table\|json\|csv` | 輸出格式 | `table`（表格） |
| `--redact-personal` | 隱藏個資（學號等） | 開啟 |
| `--no-redact-personal` | 顯示完整個資 | - |
| `--offline` | 離線模式，只用本地快取 | 關閉 |
| `--verbose` | 顯示除錯資訊 | 關閉 |
| `--silent` | 靜默模式，只顯示錯誤 | 關閉 |

#### 輸出格式範例

```
courseape courses filter --term 1151 --dept 5400B                    # 預設表格
courseape courses filter --term 1151 --dept 5400B --output json      # JSON 格式
courseape courses filter --term 1151 --dept 5400B --output csv       # CSV 格式（可匯入 Excel）
```

---

## AI 分析功能

CourseApe 本身**不會直接呼叫 AI**。它的做法是：

1. 把你的成績、修業辦法等資料準備好
2. 透過「Agent Skill」讓 AI 工具（Claude Code、OpenCode、Codex 等）讀取這些資料
3. AI 分析完後，把結果匯入 CourseApe

### 前置條件

- 安裝任一個支援的 AI Agent（[Claude Code](https://docs.anthropic.com/claude-code)、[OpenCode](https://opencode.ai)、[Codex](https://openai.com/codex)）
- 安裝 PDF 閱讀 Skill（AI 需要它來讀取修業辦法 PDF）

### 安裝 Skill

```
courseape skills install --all    # 自動偵測已安裝的 Agent 並安裝
```

或指定平台：

```
courseape skills install claude   # 只裝到 Claude Code
courseape skills install opencode # 只裝到 OpenCode
```

### 使用方式

在 AI Agent 裡直接用自然語言提問：

- 「幫我分析畢業門檻」
- 「我還缺什麼課」
- 「下學期要選什麼課」
- 「有沒有衝堂」
- 「這堂課好不好」

AI Agent 會自動執行 CourseApe 指令、讀取資料、分析後告訴你結果。

---

## 隱私與安全

| 項目 | 處理方式 |
|------|----------|
| 學號密碼 | 存在系統鑰匙圈（Windows 認證管理員 / Mac 鑰匙圈），不出現在任何檔案 |
| Session | 存在本機，不會上傳 |
| 成績 | 存在本機 SQLite 資料庫 |
| AI 分析 | 預設去識別化（隱藏學號、姓名） |
| 網路請求 | 只連學校伺服器（itouch.cycu.edu.tw） |

### 去識別化

預設所有輸出都會隱藏個人資料。用 `--no-redact-personal` 可顯示完整資料：

```
courseape profile show                        # 學號顯示為 ****1234
courseape profile show --no-redact-personal   # 完整學號
```

### 清除所有資料

```
courseape data purge
```

這會刪除：
- 登入 session
- 所有快取資料（成績、開課清單、系所清單）
- 所有下載的檔案（修業辦法 PDF、課綱等）
- 本機資料庫

**不會**刪除鑰匙圈中的帳密。要連帳密也清除：

```
courseape logout --clear-credentials
```

---

## 常見問題

### 登入相關

**Q: 登入失敗，顯示 `Login failed`**

確認學號密碼正確。可以先到 iTouch 網頁版（https://itouch.cycu.edu.tw）測試能否登入。

**Q: 顯示 `Session expired`**

Session 過期了，重新登入：
```
courseape login
```

### 資料相關

**Q: 顯示 `No cached offerings`**

你需要先同步開課資料：
```
courseape courses offerings --term 1151
```

**Q: 顯示 `Profile not set`**

第一次使用需要設定個人資料：
```
courseape profile edit
```

**Q: 成績分析結果不準確**

成績是從 iTouch HTML 解析的，如果格式異常可能出錯。可以：
1. 重新下載：`courseape sync grades`
2. 用 AI Agent 重新分析

### 安裝相關

**Q: `npm install -g` 權限不足 (Mac/Linux)**

```
sudo npm install -g @dyliu0306/courseape
```

**Q: Windows 上執行被防毒軟體擋住**

release 版本有經過 `strip` 處理，部分防毒可能誤判。可以將 courseape.exe 加入防毒白名單。

### AI 相關

**Q: `PDF Skill not found`**

CourseApe 的 AI 分析功能需要一個能閱讀 PDF 的 Skill。請先安裝：
```
npx skills add <pdf-skill>
```

**Q: AI 分析的成績怎麼匯入？**

AI 分析完會輸出 JSON 檔案，用以下指令匯入：
```
courseape data import --scope grades --file /path/to/grade_analysis.json
```

---

## 學期代碼說明

CourseApe 的 `--term` 參數使用 4 位數字：

| 代碼 | 意義 |
|------|------|
| `1141` | 114 學年度第 1 學期（2025 年 9 月 ~ 2026 年 1 月） |
| `1142` | 114 學年度第 2 學期（2026 年 2 月 ~ 2026 年 6 月） |
| `1151` | 115 學年度第 1 學期（2026 年 9 月 ~ 2027 年 1 月） |

前 3 碼是民國學年度，第 4 碼是學期（1 或 2）。

---

## 系所代碼範例

| 代碼 | 系所 |
|------|------|
| `5400B` | 資訊管理學系（學士班） |
| `5431B` | 資管一甲 |

> 完整系所清單：`courseape data export --scope departments`

---

## 授權

本專案採用 [PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0) — 僅限非商業用途。

---

## 開發者資訊

```bash
git clone https://github.com/<owner>/courseape && cd courseape

# 建置
cargo build

# 執行
cargo run -- --help

# 測試（28 個單元測試）
cargo test

# Lint
cargo clippy -- -D warnings

# Release 建置
cargo build --release
```

- 語言：Rust (核心) + TypeScript (npm wrapper)
- 最低 Node.js：v18
- 支援平台：Windows x64/arm64、Linux x64/arm64、macOS x64/arm64
- Session 儲存：本機 SQLite + 系統鑰匙圈
- CI/CD：GitHub Actions（push tag `v*` 觸發自動發布到 npm + GitHub Release）
