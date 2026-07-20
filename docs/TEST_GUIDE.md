# CourseApe 產品發布前全方位測試指南

## 這是什麼？

**CourseApe** 是中原大學 (CYCU) 學生專用的選課輔助 CLI 工具。

- **GitHub Repo**：`C:\Users\user\Documents\學業\courseape`（Rust 專案）
- **定位**：獨立於 openape 的 CLI，專注於畢業門檻分析、開課查詢、衝堂檢查、AI 輔助選課
- **技術棧**：Rust + clap CLI、npm wrapper、Agent Skill（Claude/Codex/OpenCode）
- **授權**：PolyForm Noncommercial 1.0.0（禁止商用）

### 核心功能

| 功能 | 命令 | 說明 |
|------|------|------|
| 登入 | `courseape login` | 使用 CYCU iTouch 帳密，存入 OS keyring |
| 個人資料 | `courseape profile show/edit` | 系所、入學年度、學制 |
| 同步系所 | `courseape sync departments --year 114` | API 取得所有系所代碼 |
| 同步修業辦法 | `courseape sync requirements --year 0` | 下載 PDF（0=自動偵測入學年） |
| 同步成績 | `courseape sync grades` | 下載歷年成績 HTML |
| 同步歷史開課 | `courseape courses history` | 自動逐年期同步，用於通識向度判斷 |
| 同步開課 | `courseape courses offerings --term 1151` | iTouch courseQuery API |
| 篩選課程 | `courseape courses filter --term 1151 [20+選項]` | 系所/必選修/學分/EMI/PBL/時段... |
| 備選清單 | `courseape shortlist add/remove/list/clear` | 選課備選池 |
| 衝堂檢查 | `courseape courses conflicts --term 1151` | 必修自動納入 + 備選，僅警告 |
| 課表 | `courseape courses timetable --term 1151` | 顯示課程名稱的週課表 |
| 課綱 | `courseape courses syllabus <code> --term 1151` | 下載 CMAP PDF |
| 自動規劃 | `courseape courses plan --term 1151` | 比對重修課程與開課，預設只顯示候選；`--apply` 才加入備選 |
| 匯出 | `courseape data export --scope <scope>` | profile/grades/grade-html/offerings/departments |
| 匯入 | `courseape data import --scope grades --file <json>` | Agent 分析結果寫入 DB |
| Skill | `courseape skills install <claude|codex|opencode>` | 安裝 Agent Skill（含 PDF Skill 前置檢查） |

### 安全特性
- 帳密存於 OS keyring，不出現在檔案或日誌
- 預設遮罩學號/姓名；`--no-redact-personal` 才顯示完整資料
- AI 分析由 Agent Skill 執行，CLI 不內建 LLM
- PDF Skill 安裝與執行雙重檢查
- Cookie、JWT、loginToken 不進入日誌

---

## 測試任務

### 1. 產品受眾分析

分析此工具的目標用戶畫像：

- **主要受眾**：中原大學學士班學生（特別是選課季）
- **次要受眾**：研究所學生、雙主修/輔系學生
- **使用場景**：選課前的畢業門檻盤點、開課清單篩選、衝堂檢查、通識向度確認
- **痛點**：手動比對修業辦法 PDF 與歷年成績、逐門查開課時間、通識向度不清楚
- **競品**：openape（i-Learning 自動化）、學校選課系統、手動 Excel

請評估：
1. 目標用戶是否能獨立完成 `courseape login` 到 `courses plan` 的完整流程？
2. 哪些功能最吸引學生？哪些可能被忽略？
3. README 是否能在 30 秒內讓新用戶理解這是什麼、怎麼裝、怎麼用？
4. 與 openape 的定位差異是否清楚？

### 2. 安裝方式測試

測試以下安裝路徑：

```bash
# 方式 A：npm 全域安裝（發布後）
npm install -g @dyliu0306/courseape

# 方式 B：npx 單次執行
npx @dyliu0306/courseape --help

# 方式 C：從原始碼建置
git clone <repo>
cd courseape
cargo build --release
./target/release/courseape --help
```

檢查項目：
- [ ] `cargo build` 在 Windows/Linux/macOS 是否都能成功
- [ ] `cargo test` 是否全部通過
- [ ] `cargo clippy -- -D warnings` 是否無警告
- [ ] npm wrapper 是否正確指向 binary
- [ ] `courseape --help` 是否顯示完整指令樹
- [ ] `courseape --version` 是否顯示正確版本
- [ ] 各子命令 `--help` 是否都有清楚的說明

### 3. 打包方式測試

檢查 npm 發布結構：

```text
@dyliu0306/courseape/              # base package (npm/app/)
├── package.json               # optionalDependencies 指向平台包
├── dist/index.js              # wrapper，spawnSync 平台 binary
├── README.md
└── LICENSE

@dyliu0306/courseape-win32-x64/    # platform package
├── package.json               # os/cpu 欄位正確
├── bin/courseape.exe          # Rust binary
├── README.md
└── LICENSE
```

檢查項目：
- [ ] `npm/app/package.json` 的 `@dyliu0306` 已替換為實際 scope
- [ ] `scripts/package-platform.mjs` 產生正確的 platform package metadata
- [ ] platform package 的 `os` 和 `cpu` 欄位正確
- [ ] SHA256 checksum 在 release assets 中
- [ ] `.github/workflows/publish.yml` 觸發條件正確
- [ ] LICENSE 檔案是 PolyForm Noncommercial 1.0.0

### 4. 測試方式與流程

#### 4.1 環境準備

```bash
# 設定測試帳密（CourseApe 會讀取）
$env:CYCU_USERNAME = "學號"
$env:CYCU_PASSWORD = "密碼"

# 或由 CourseApe 寫入自己的 OS 鑰匙圈條目
courseape credentials set
```

#### 4.2 單元測試

```bash
cargo test
```

應通過的測試：
- `parsers::time_slot` — 時段解析、衝堂比對 (9 tests)
- `parsers::department_json` — 系所 JSON 解析
- `parsers::grade_html` — 成績 HTML 解析 (含 <br/> 轉 \n)
- `analysis::filter` — 篩選引擎
- `analysis::conflict` — 衝堂偵測 (含多位時段 2-123)
- `redact::profile` — 學號遮罩
- `auth::keyring` — 憑證序列化

測試數量會隨回歸案例增加；以 `cargo test --locked` 的實際結果為準。

#### 4.3 整合測試（需網路）

依序執行以下命令，每步驗證輸出：

```bash
# 1. 登入
courseape login
courseape status  # 應顯示 "logged in"

# 2. 個人資料
courseape profile edit  # 設定系所、入學年
courseape profile show  # 應顯示去識別化資料

# 3. 同步
courseape sync departments --year 114  # 應回傳 ~46 個系所
courseape sync requirements --year 0   # 應下載 PDF
courseape sync grades                  # 應下載 HTML
courseape courses history              # 應同步 7 學期開課

# 4. 開課查詢
courseape courses offerings --term 1151  # 應回傳 3000+ 筆

# 5. 篩選
courseape courses filter --term 1151 --dept 5400B --type 必修
courseape courses filter --term 1151 --emi
courseape courses filter --term 1151 --teacher "劉" --credit 3

# 6. 備選清單
courseape shortlist add MI034G --term 1151
courseape shortlist list --term 1151
courseape shortlist remove MI034G --term 1151

# 7. 衝堂與課表
courseape courses conflicts --term 1151
courseape courses timetable --term 1151

# 8. 匯出匯入
courseape data export --scope departments
courseape data export --scope offerings
courseape data export --scope grade-html
courseape data import --scope grades --file test_grades.json
courseape data export --scope grades

# 9. 自動規劃
courseape courses plan --term 1151
courseape courses plan --term 1151

# 10. Skill 安裝
courseape skills install opencode
courseape skills show

# 11. 清除
courseape data purge
courseape logout
```

#### 4.4 Agent Skill 測試

安裝 Skill 後，在 Agent 中測試：

1. 說「幫我分析成績」→ Agent 應執行 Workflow 0
2. 說「我還缺什麼課」→ Agent 應執行 Workflow 1
3. 說「下學期要選什麼課」→ Agent 應執行 Workflow 2
4. 說「這堂課好不好」→ Agent 應搜尋網路評價

#### 4.5 錯誤處理測試

| 情境 | 預期行為 |
|------|----------|
| 未登入執行 `sync grades` | 顯示 "Not logged in" 錯誤 |
| 未設定 profile 執行 `sync requirements` | 顯示 "Profile not set" 錯誤 |
| 未同步 offerings 執行 `courses filter` | 顯示 "No cached offerings" |
| 無效學期代碼 | 顯示 API 錯誤 |
| PDF Skill 未安裝時執行 `skills install` | 拒絕安裝並顯示安裝指引 |
| `data import` 傳入無效 JSON | 顯示解析錯誤 |
| 重複執行 `shortlist add` 同一課程 | 不重複加入，顯示 "[已加入]" |

### 5. 使用體感評分

請對以下項目評分（1-5 分）：

#### 5.1 首次體驗

| 項目 | 評分標準 |
|------|----------|
| README 可讀性 | 30 秒內能否理解功能、安裝、使用方式 |
| 首次登入流暢度 | 從 `courseape login` 到成功需幾步、幾個錯誤 |
| 首次同步體驗 | `sync departments/requirements/grades` 是否一次成功 |
| 錯誤訊息品質 | 錯誤訊息是否明確告訴用戶下一步該做什麼 |

#### 5.2 日常使用

| 項目 | 評分標準 |
|------|----------|
| 篩選直覺性 | `courses filter` 的參數名稱是否一看就懂 |
| 課表可讀性 | `courses timetable` 輸出是否一目了然 |
| 衝堂報告 | 衝堂警告是否包含足夠資訊（課程名、時段） |
| 備選清單操作 | add/remove/list 是否直覺 |
| 命令記憶負擔 | 常用命令是否容易記住 |

#### 5.3 進階功能

| 項目 | 評分標準 |
|------|----------|
| 歷史開課同步 | `courses history` 是否一次成功、速度是否可接受 |
| 自動規劃 | `courses plan` 匹配邏輯是否合理 |
| Agent 整合 | Skill 安裝是否順暢、Agent 是否正確執行工作流 |
| 資料隱私 | 去識別化是否有效、敏感資料是否洩漏 |

#### 5.4 整體評分

| 維度 | 評分 (1-5) | 備註 |
|------|-----------|------|
| 安裝便利性 | ? | |
| 首次使用門檻 | ? | |
| 功能完整性 | ? | |
| 錯誤處理品質 | ? | |
| 文件品質 | ? | |
| 效能感受 | ? | |
| 隱私安全感 | ? | |
| 整體推薦度 | ? | |

---

## 已知限制

1. **elective API 需要 browser loginToken**：開課資料改用 iTouch courseQuery API（已實作）
2. **修業辦法 PDF 解析**：由 AI Agent 執行，準確度依賴 PDF Skill 品質
3. **成績 HTML 編碼**：iTouch 回傳的 HTML 編碼不一致，需 UTF-8/Big5 雙重嘗試
4. **通識向度判斷**：依賴歷史開課資料的 `OP_TYPE` 欄位，約 5% 課程無此欄位
5. **衝堂偵測**：已改為展開式比對，`2-123` 展開為 period 1,2,3 逐個比對。`2-A` 與 `2-1` 不視為衝突（A=第9節中午, 1=第1節，時段不重疊）

## 測試報告格式

請按以下格式輸出測試報告：

```markdown
# CourseApe 測試報告

## 測試環境
- OS: 
- Rust 版本:
- Node.js 版本:
- 測試時間:

## 測試結果摘要
| 類別 | 通過/總數 | 狀態 |
|------|----------|------|

## 詳細結果
（每項測試的實際輸出與問題）

## Bug 列表
| # | 嚴重度 | 描述 | 重現步驟 |
|---|--------|------|----------|

## 改進建議
（按優先級排列）

## 體感評分
（依上述評分表填寫）
```
