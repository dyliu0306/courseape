# CLI 完整參考

本文檔為 CourseApe 所有低階 CLI 命令的完整參考。一般使用者應優先使用 Agent 自然語言介面。

## 全域選項

這些選項可以加在任何指令後面。

| 選項 | 說明 | 預設值 |
|------|------|--------|
| `--output table\|json\|csv` | 輸出格式 | `table` |
| `--redact-personal` | 隱藏個資 | 開啟 |
| `--no-redact-personal` | 顯示完整個資 | - |
| `--offline` | 離線模式，只用本地快取 | 關閉 |
| `--verbose` | 顯示除錯資訊 | 關閉 |
| `--silent` | 靜默模式 | 關閉 |

## Agent 高階命令

### `courseape doctor`

回報系統狀態（JSON）。Agent 用此命令診斷問題。

```bash
courseape agent doctor
```

回傳：登入狀態、個人資料、快取狀態、資料新鮮度。

### `courseape setup`

一鍵初始化：登入檢查、系所同步、個人資料自動推導。

```bash
courseape agent setup
```

自動執行：檢查 session → 同步系所清單 → 從學號推導入學年度 → 儲存 profile。

### `courseape prepare graduation`

準備畢業分析所需的所有資料。

```bash
courseape agent prepare graduation
```

自動執行：下載修業辦法 PDF → 下載成績 HTML → 同步歷史開課資料。

### `courseape prepare planning`

準備下學期規劃所需的所有資料。

```bash
courseape agent prepare planning              # 自動判斷下學期
courseape agent prepare planning --term 1151  # 指定學期
```

### `courseape resolve`

將自然語言系所名稱解析為代碼。

```bash
courseape agent resolve "資管系"
courseape agent resolve "資訊管理學系"
courseape agent resolve "5400B"
```

回傳 JSON，包含候選結果與信心分數。

### `courseape context`

回傳 Agent 執行特定任務所需的資料狀態與下一步。

```bash
courseape agent context --task graduation
courseape agent context --task planning
```

### `courseape refresh`

重新下載過期或缺失的資料。

```bash
courseape agent refresh
```

## 登入與帳號

| 指令 | 說明 |
|------|------|
| `courseape login` | 登入 iTouch |
| `courseape status` | 查看登入狀態 |
| `courseape logout` | 登出（保留鑰匙圈帳密） |
| `courseape logout --clear-credentials` | 登出並清除帳密 |
| `courseape credentials set` | 更新學號密碼 |

## 個人資料

| 指令 | 說明 |
|------|------|
| `courseape profile show` | 查看個人資料 |
| `courseape profile edit` | 修改入學年度、系所、學制 |

## 資料同步

| 指令 | 說明 |
|------|------|
| `courseape sync departments --year 114` | 同步系所清單 |
| `courseape sync requirements --year 112` | 下載修業辦法 PDF |
| `courseape sync requirements --year 0` | 自動推算入學年度 |
| `courseape sync grades` | 下載歷年成績 HTML |

## 課程查詢

### 列出開課

```bash
courseape courses offerings --term 1151
```

### 篩選課程

所有條件可自由組合（且的關係）。

```bash
courseape courses filter --term 1151 --dept 5400B --type 必修 --credit 3
```

| 條件 | 說明 | 範例 |
|------|------|------|
| `--dept <代碼>` | 系所代碼 | `--dept 5400B` |
| `--class_dept <代碼>` | 班級 | `--class_dept 5431B` |
| `--keyword <文字>` | 課程名稱 | `--keyword 資管` |
| `--code <代碼>` | 課程代碼前綴 | `--code MI` |
| `--teacher <姓名>` | 教師姓名 | `--teacher 劉` |
| `--teacher_id <代碼>` | 教師人事代碼 | `--teacher_id 12508` |
| `--type <必修\|選修>` | 必修或選修 | `--type 必修` |
| `--credit <數字>` | 學分數 | `--credit 3` |
| `--div <B\|M\|D\|H>` | 部別 | `--div B` |
| `--language <文字>` | 授課語言 | `--language 英語` |
| `--day <1-7>` | 星期幾（1=週一） | `--day 2` |
| `--period <代碼>` | 節次 | `--period A` |
| `--classroom <文字>` | 教室 | `--classroom 管理` |
| `--general <類別>` | 通識向度 | `--general 基礎天` |
| `--emi` | 只看 EMI | `--emi` |
| `--english` | 只看英語授課 | `--english` |
| `--distance` | 只看遠距 | `--distance` |
| `--pbl` | 只看 PBL | `--pbl` |
| `--programming` | 只看程式設計 | `--programming` |
| `--available` | 只看有餘額 | `--available` |
| `--semester <全學期\|半學期>` | 期程 | `--semester 半學期` |
| `--cross` | 只看跨系課程 | `--cross` |
| `--sdgs <文字>` | SDGs 目標 | `--sdgs SDGS` |

### 其他課程命令

| 指令 | 說明 |
|------|------|
| `courseape courses conflicts --term 1151` | 檢查衝堂 |
| `courseape courses timetable --term 1151` | 顯示課表 |
| `courseape courses syllabus <代碼> --term 1151` | 下載課綱 PDF |
| `courseape courses plan --term 1151` | 自動規劃重修 |
| `courseape courses history` | 同步歷史開課資料 |

## 備選清單

| 指令 | 說明 |
|------|------|
| `courseape shortlist add <代碼> --term 1151` | 加入備選 |
| `courseape shortlist remove <代碼> --term 1151` | 移除 |
| `courseape shortlist list --term 1151` | 查看清單 |
| `courseape shortlist clear --term 1151` | 清空 |

## 資料匯出與匯入

| 指令 | 說明 |
|------|------|
| `courseape data export --scope profile` | 匯出個人資料 |
| `courseape data export --scope departments` | 匯出系所清單 |
| `courseape data export --scope grades` | 匯出已分析成績 |
| `courseape data export --scope grade-html` | 匯出原始成績 HTML |
| `courseape data export --scope offerings` | 匯出歷史開課 |
| `courseape data import --scope grades --file <檔案>` | 匯入 AI 分析成績 |
| `courseape data purge` | 清除所有快取 |

## Skills

| 指令 | 說明 |
|------|------|
| `courseape skills install --all` | 自動偵測並安裝 |
| `courseape skills install claude` | 安裝到 Claude Code |
| `courseape skills install opencode` | 安裝到 OpenCode |
| `courseape skills install codex` | 安裝到 Codex |
| `courseape skills show` | 查看 SKILL.md |

## 學期代碼

4 位數字：前 3 碼民國學年度，第 4 碼學期（1 或 2）。

| 代碼 | 意義 |
|------|------|
| `1141` | 114 學年第 1 學期（2025.9 ~ 2026.1） |
| `1142` | 114 學年第 2 學期（2026.2 ~ 2026.6） |
| `1151` | 115 學年第 1 學期（2026.9 ~ 2027.1） |
