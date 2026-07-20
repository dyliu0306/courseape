# Agent 指南

本文檔說明 CourseApe 的 Agent Skill 如何運作，以及如何診斷和排除問題。

## 支援的 Agent 平台

| 平台 | 安裝命令 |
|------|----------|
| Claude Code | `courseape skills install claude` |
| OpenCode | `courseape skills install opencode` |
| Codex | `courseape skills install codex` |

## Skill 工作流程

Agent Skill 採用狀態機模式，依 `doctor` → 自動補齊 → `prepare` → 分析 → 匯入 的流程執行。

### 狀態轉換圖

```
收到使用者意圖
  ↓
doctor (診斷現有狀態)
  ↓
判斷缺失項目
  ├─ 未登入 → login → 重新 doctor
  ├─ profile 不完整 → setup → 重新 doctor
  ├─ 缺修業辦法 → prepare graduation
  ├─ 缺成績 → prepare graduation
  └─ 資料齊全 → 進入分析
  ↓
Agent 讀取資料並分析
  ↓
CLI 驗證並保存結果
  ↓
用學生語言回答
```

## 支援的意圖

| 使用者說 | Agent 動作 |
|----------|-----------|
| 「幫我設定 CourseApe」 | setup |
| 「幫我分析畢業門檻」 | context graduation → prepare graduation → 分析 |
| 「我還缺什麼課」 | 同上 |
| 「下學期要選什麼課」 | context planning → prepare planning → 分析 + filter + shortlist |
| 「幫我查課程」 | courses filter |
| 「有沒有衝堂」 | courses conflicts |
| 「這堂課好不好」 | 搜尋網路評價 |

## 診斷命令

### `courseape agent doctor`

回傳 JSON，包含所有關鍵狀態：

```json
{
  "logged_in": true,
  "profile_exists": true,
  "profile_complete": true,
  "profile": { "student_id": "***", "dept_code": "5400B", "enroll_year": 112 },
  "departments_synced": true,
  "requirements_downloaded": true,
  "grades_downloaded": true,
  "grades_analyzed": false,
  "cached_terms": ["1121", "1122", "1131", "1132", "1141", "1142"],
  "current_term": "1142",
  "next_term": "1151"
}
```

### `courseape agent context --task <task>`

回傳特定任務的資料狀態與下一步建議。

## 常見問題

### 「PDF Skill not found」

CourseApe 的 AI 分析功能需要一個能閱讀 PDF 的 Skill。請先安裝。

### 「Not logged in」

執行 `courseape login`，或在 Agent 裡說「幫我登入 CourseApe」。

### 「Profile not set」

執行 `courseape agent setup`，或在 Agent 裡說「幫我設定 CourseApe」。

### 成績分析不準確

1. 重新下載：`courseape sync grades`
2. 重新分析：請 Agent 再次分析成績 HTML
3. 匯入：`courseape data import --scope grades --file <analysis.json>`

### 系所名稱無法辨識

使用 `courseape agent resolve "你系上的名稱"` 查看候選結果。如果沒有匹配，先執行 `courseape sync departments --year 114`。

## JSON 輸出格式

所有 `--json` 命令的 stdout 為穩定 JSON 結構，進度和診斷走 stderr。Agent 應只解析 stdout。

錯誤時，CLI 以非零 exit code 退出，stderr 包含錯誤訊息。
