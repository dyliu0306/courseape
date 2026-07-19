# 開發指南

## 環境需求

- Rust 1.70+ (建議使用 [rustup](https://rustup.rs/))
- Node.js v18+（npm wrapper 用）
- Windows: VS Build Tools

## 建置

```bash
git clone https://github.com/dyliu0306/courseape && cd courseape

# 開發建置
cargo build

# 執行
cargo run -- --help

# 測試
cargo test

# Lint
cargo clippy -- -D warnings

# Release 建置
cargo build --release
```

## 專案結構

```
courseape/
├── src/
│   ├── main.rs              # CLI 命令樹
│   ├── cli/                 # 命令處理器
│   │   ├── agent.rs         # doctor/setup/prepare/resolve/context/refresh
│   │   ├── auth.rs          # login/status/logout
│   │   ├── courses.rs       # offerings/filter/conflicts/timetable/plan/history
│   │   ├── data.rs          # export/import/purge
│   │   ├── profile.rs       # profile show/edit
│   │   ├── shortlist.rs     # shortlist add/remove/list/clear
│   │   ├── skills.rs        # skills install/show
│   │   └── sync.rs          # departments/requirements/grades
│   ├── connectors/          # HTTP 連接器
│   │   ├── itouch.rs        # iTouch 登入 + 成績
│   │   ├── elective.rs      # courseQuery API
│   │   ├── necessary_course.rs  # 系所清單 + 修業辦法
│   │   └── cmap.rs          # 課綱 PDF
│   ├── domain/              # 領域模型
│   │   ├── course_offering.rs
│   │   ├── department.rs
│   │   ├── profile.rs
│   │   └── resolver.rs      # 系所名稱解析 + term 自動判斷
│   ├── analysis/            # 分析引擎
│   │   ├── conflict.rs      # 衝堂偵測
│   │   └── filter.rs        # 課程篩選
│   ├── storage/             # 資料儲存
│   │   ├── db.rs            # SQLite schema + migration
│   │   ├── repo.rs          # Repository pattern
│   │   └── snapshot.rs      # 檔案快照
│   ├── parsers/             # 解析器
│   │   ├── grade_html.rs    # 成績 HTML 解析
│   │   ├── department_json.rs
│   │   └── time_slot.rs
│   ├── auth/                # 認證
│   │   ├── keyring.rs       # OS 鑰匙圈
│   │   └── session.rs       # Session 管理
│   ├── output/              # 輸出格式化
│   ├── redact/              # 去識別化
│   └── error.rs             # 錯誤類型
├── skills/courseape/        # Agent Skill
├── schemas/                 # JSON Schema
├── npm/app/                 # npm wrapper
├── docs/                    # 文件
└── .github/workflows/       # CI/CD
```

## 測試

```bash
cargo test                   # 執行所有測試
cargo test test_name         # 執行特定測試
cargo test -- --nocapture    # 顯示 eprintln 輸出
```

## 發布流程

1. 更新 `Cargo.toml` 版本號
2. Commit & push
3. 建立 GitHub Release（tag: `v0.x.x`）
4. CI 自動：建置 6 平台 binary → 發布 npm → 上傳 GitHub Release

## 架構決策

- **courseQuery API over elective API**：elective API 需要 browser-based loginToken；courseQuery 只需 iTouch session cookie + Origin/Referer headers
- **AI-first 成績解析**：成績 HTML 由 Agent 分析，不使用 deterministic parser
- **OP_TYPE 判斷通識向度**：不靠課名猜測，用歷史開課資料的 OP_TYPE 欄位
- **Offerings PK = (code, term, class_dept)**：避免同課程代碼不同班級的資料遺失
- **keyring v4**：與 openape 相容
