# CourseApe Design Specification

> Version: 1.0.0-draft
> Date: 2026-07-19
> Status: Ready for user review

## 1. Product Overview

**CourseApe** is an independent CYCU course-planning CLI. It does not overlap with `openape`, which targets i-Learning (Moodle). CourseApe focuses on graduation requirement analysis, course offering browsing, conflict detection, and AI-assisted review.

Core principles:
- Modular design: easy to debug, modify, and extend.
- Versioned data: every parsed result carries a schema version and source trace.
- Privacy-first: credentials never touch plain files, logs, or CLI arguments; AI workloads are redacted by default.
- Agent-separated: the CLI never calls an LLM directly; AI parsing, PDF analysis, and review search are delegated to an installable Agent Skill.

### 1.1 Scope

In-scope for the MVP:
- `login`, `status`, `logout`, `credentials set`
- `profile show`, `profile edit` (student ID, department, enrollment year, degree)
- `sync departments --year <acadYear>`
- `sync requirements --year <acadYear>`
- `sync grades`
- `courses offerings --term <term>`
- `courses filter --term <term> [filters...]`
- `courses conflicts --term <term>`
- `courses syllabus <course-code> --term <term>`
- `data export --scope profile|grades|requirements|offerings`
- `skills install <claude|codex|opencode>`, `skills show`

Out-of-scope for the MVP:
- Submitting or changing any enrollment state.
- Internal LLM calls; CLI-based web search; browser automation.
- Google Sheets/Docs integration.
- Mobile or web UI.

## 2. Architecture

Four-layer separation:

```text
cli/                clap commands, output formatting, interactive prompts
connectors/         HTTP clients per CYCU endpoint; request/response snapshots
domain/storage/     versioned DTOs, SQLite, cache, raw snapshot archive
agent bridge/       work-package builder, schema validation, redaction, Skill invocation
```

### 2.1 Data Flow

```text
1. login                          -> itouch auth -> CourseApe session cookie -> local session store
2. profile setup                  -> manual edit or auto-detect from s_grade.html / itouch
3. sync departments               -> necessaryCourse/query -> Department cache
4. sync requirements              -> export_PDF -> raw PDF -> work-package for Skill
5. sync grades                    -> s_grade.html -> raw HTML -> work-package for Skill
6. courses offerings              -> elective_system.json -> Offering cache
7. courses filter                 -> same API, filtered -> Offering results
8. courses conflicts              -> Offering cache -> ConflictReport (deterministic)
9. data export                    -> normalized JSON/CSV/table -> Agent-ready work-package
10. skills install/show           -> detect PDF Skill -> install CourseApe Skill
11. CourseApe Skill (in Agent)    -> check PDF Skill -> read work-package -> AI -> validated JSON
```

### 2.2 Module Structure

```text
courseape/
├── src/
│   ├── cli/
│   │   ├── mod.rs              # top-level clap struct, global flags
│   │   ├── auth.rs             # login, status, logout, credentials
│   │   ├── profile.rs          # profile show, profile edit
│   │   ├── sync.rs             # sync departments, requirements, grades
│   │   ├── courses.rs          # offerings, filter, conflicts, syllabus
│   │   ├── data.rs             # data export
│   │   └── skills.rs           # skills install, skills show
│   ├── auth/
│   │   ├── mod.rs              # public auth interface
│   │   ├── keyring.rs          # OS keyring read/write (shared entry)
│   │   ├── itouch_login.rs     # POST login2.jsp, session extraction
│   │   └── session.rs          # session cookie persistence (CourseApe-owned)
│   ├── connectors/
│   │   ├── mod.rs
│   │   ├── itouch.rs           # login2.jsp, s_grade.jsp
│   │   ├── necessary_course.rs # queryNecessary.jsp, export_PDF.jsp
│   │   ├── elective.rs         # elective_system.jsp
│   │   └── cmap.rs             # syllabus PDF download
│   ├── parsers/
│   │   ├── mod.rs
│   │   ├── grade_html.rs       # deterministic HTML extraction
│   │   ├── department_json.rs  # queryNecessary response
│   │   └── time_slot.rs        # period/day code parsing + conflict logic
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── department.rs       # Department { dept_code, name, year }
│   │   ├── completed_course.rs # CompletedCourse { code, name, credits, status, term }
│   │   ├── course_offering.rs  # CourseOffering { code, name, teacher, time_slots, credits, dept, ... }
│   │   ├── requirement_doc.rs  # RequirementDocument { source, raw_path, parsed_items }
│   │   ├── profile.rs          # StudentProfile { student_id, dept_code, enroll_year, degree }
│   │   ├── filter_catalog.rs   # FilterCatalog { filters[] }
│   │   └── conflict.rs         # ConflictReport { pairs[] }
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── db.rs               # SQLite setup + migrations
│   │   ├── repo.rs             # CRUD for domain types
│   │   └── snapshot.rs         # raw HTTP/PDF/HTML snapshot archive
│   ├── analysis/
│   │   ├── mod.rs
│   │   ├── filter.rs           # offering filter engine
│   │   └── conflict.rs         # time-slot conflict detection
│   ├── redact/
│   │   ├── mod.rs              # redaction rules, CLI flag wiring
│   │   └── profile.rs          # name, student_id, email masking
│   ├── output/
│   │   ├── mod.rs              # table/json/csv formatter
│   │   └── formatter.rs
│   └── error.rs
├── skills/
│   └── courseape/
│       └── SKILL.md
├── schemas/
│   ├── work_package.json       # AI input schema
│   ├── grade_analysis.json     # AI grade output schema
│   ├── requirement_analysis.json
│   └── review_output.json
├── fixtures/
│   ├── departments/            # mock queryNecessary responses
│   ├── grades/                 # mock s_grade HTML
│   ├── offerings/              # mock elective_system JSON
│   └── requirements/           # mock PDFs (small, synthetic)
├── npm/
│   ├── app/
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   └── src/index.ts
│   └── package.json.tmpl
├── .github/
│   └── workflows/
│       └── publish.yml
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
└── .gitignore
```

## 3. CLI Commands

### 3.1 Global Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--output <format>` | Output format: `json`, `csv`, `table` | `table` |
| `--redact-personal` | Remove personal identifiers from output | `true` |
| `--no-redact-personal` | Include personal identifiers | (override) |
| `--offline` | Use only cached data; do not contact APIs | `false` |
| `--config <path>` | Override config file path | OS default |
| `--session <path>` | Override session storage path | OS default |
| `--verbose` | Enable debug logging | `false` |
| `--silent` | Suppress all stderr output | `false` |

### 3.2 Commands

#### Auth & Credentials

```bash
courseape login
  # Read student_id + password from OS keyring (openape/moodle-auto-login).
  # If absent, prompt interactively (hidden input).
  # POST to itouch login2.jsp.
  # Save CourseApe session cookie to local session store.
  # Auto-detect profile from s_grade if possible.

courseape status
  # Show session state, login expiry, detected profile (redacted).
  # Exit 0 if logged in, exit 1 if not.

courseape logout
  # Clear CourseApe session cookie.
  # Does NOT clear OS keyring unless --clear-credentials is passed.

courseape credentials set
  # Interactive hidden input for student_id + password.
  # Display warning: "This will update the shared OS keyring entry used by openape."
  # Require explicit confirmation (y/N).
  # Write to keyring entry: service=openape, account=moodle-auto-login.
  # Clear CourseApe session; re-login to validate new credentials.
```

#### Profile

```bash
courseape profile show
  # Display: student_id (redacted by default), dept_code, department name,
  #          enroll_year, degree, last_sync timestamp.

courseape profile edit
  # Interactive editor for: enroll_year, dept_code, degree.
  # If itouch auto-detection is available, show detected values as defaults.
  # Save to local SQLite.
```

#### Sync

```bash
courseape sync departments --year <acadYear>
  # GET queryNecessary.jsp with {YEAR, DEGREE_KIND:"學士", PRACTICE_TYPE:1}
  # Cache all {DEPT_CODE, DEPT_NAME} pairs for the year.
  # Output: count of departments synced.

courseape sync requirements --year <acadYear>
  # Determine DEPT_CODE from profile.
  # Download export_PDF for that department/year.
  # Save raw PDF to snapshot archive.
  # Prepare work-package for AI Skill analysis.
  # Output: work-package path, raw PDF path.

courseape sync grades
  # GET s_grade.jsp (requires CourseApe session cookie).
  # Save raw HTML to snapshot archive.
  # Prepare work-package for AI Skill analysis.
  # Output: work-package path, raw HTML path.
```

#### Courses

```bash
courseape courses offerings --term <term>
  # POST elective_system.jsp with default filters (all time slots, all types).
  # Cache full offering list for the term.
  # Output: count of offerings synced.

courseape courses filter --term <term> [--dept <code>] [--type <必修|選修>] \
  [--credit <n>] [--teacher <name>] [--day <d>] [--period <p>] \
  [--general <category>] [--keyword <text>]
  # Query cached offerings, apply local filters.
  # Filter aliases map to API keys in FilterCatalog.
  # Output: filtered offering list.

courseape courses conflicts --term <term>
  # Load user's planned/selected courses for the term (manual list or synced data).
  # Compare all time_slot pairs deterministically.
  # Output: ConflictReport { conflict_count, pairs[] }.

courseape courses syllabus <course-code> --term <term>
  # Download CMAP PDF for the course.
  # Save raw PDF to snapshot archive.
  # Output: PDF path for Agent/Skill reading.
```

#### Data Export

```bash
courseape data export --scope <scope> [--format <json|csv>] [--output-file <path>]
  # scope: profile, grades, requirements, offerings
  # Export normalized local data as work-package or standalone file.
  # Applies --redact-personal rules before writing.
```

#### Skills

```bash
courseape skills install <claude|codex|opencode>
  # 1. Detect target Agent's PDF Skill in its skill directory.
  # 2. If absent: REFUSE with message listing the required PDF Skill and
  #    suggested install command (e.g. "npx skills add ...").
  # 3. If present: install CourseApe SKILL.md + bundled schemas to target.

courseape skills show
  # Print raw SKILL.md content.
```

## 4. Authentication & Credentials

### 4.1 Keyring Strategy

CourseApe reuses the same OS keyring entry as `openape`:

| Field | Value |
|-------|-------|
| Service | `openape` |
| Account | `moodle-auto-login` |
| Payload | JSON: `[student_id, password]` |

This means:
- A student who already uses `openape` does not need to re-enter credentials.
- `courseape credentials set` updates the same entry; both tools are affected.
- CourseApe never reads `openape`'s `.auth/storage-state.json`; it manages its own session independently.

### 4.2 Session Management

| Storage | Location | Content |
|---------|----------|---------|
| CourseApe session | OS app data dir / `session.json` | itouch cookie, login timestamp, expiry estimate |
| Keyring | OS credential store | `student_id`, `password` |
| Profile | SQLite in app data dir | `dept_code`, `enroll_year`, `degree` |
| Cache | SQLite + snapshot dir | API responses, PDFs, HTML, parsed results |
| AI results | SQLite + snapshot dir | Skill analysis output with schema version |

`data purge` clears session, cache, AI results, and snapshots. It does NOT clear keyring.
`logout --clear-credentials` additionally clears keyring (with confirmation prompt).

### 4.3 Auto-Detection

After successful login, CourseApe attempts to detect the student's profile by:
1. Fetching `s_grade.jsp` HTML.
2. Extracting department name, student ID patterns, and enrollment term from the HTML.
3. Cross-referencing with the department cache to find the matching `DEPT_CODE`.

If detection fails or yields ambiguous results (e.g., double major, transfer, delayed graduation), CourseApe prompts the user to manually confirm or edit via `profile edit`.

## 5. API Connectors

### 5.1 Endpoint Map

| Connector | Endpoint | Method | Auth | Purpose |
|-----------|----------|--------|------|---------|
| `necessary_course` | `/active_project/cycu2000h_03/necessaryCourse/mvc/queryNecessary.jsp` | GET/POST | None | Department list + DEPT_CODE per year |
| `necessary_course` | `/active_project/cycu2000h_03/necessaryCourse/mvc/export_PDF.jsp` | GET | None | Graduation requirement PDF per dept |
| `itouch` | `/active_system/login/login2.jsp?a=b` | POST | Form: UserNm, UserPasswd | Login; returns session cookie |
| `itouch` | `/active_system/quary/s_grade.jsp` | GET | Cookie | Historical grade HTML |
| `elective` | `/myself.cycu.edu.tw/myself_api_127/elective/mvc/elective_system.jsp` | POST | loginToken + JWT | Course offering JSON |
| `cmap` | `cmap.cycu.edu.tw:8443/Syllabus/syllabus/outPutCoursePreView.action` | GET | None | Course syllabus PDF |

### 5.2 Connector Contract

Each connector module:
- Defines the base URL, required headers, and expected response content type.
- Returns a `ConnectorResult { status, headers, body_bytes, elapsed_ms }`.
- Saves a raw snapshot (status, hash, truncated body preview) to the snapshot archive.
- Never logs `UserPasswd`, `Cookie`, `loginToken`, or JWT.
- Never writes secrets to snapshot filenames.

### 5.3 Filter Catalog

The `elective_system.jsp` endpoint accepts a complex JSON payload with many filter keys. CourseApe maintains a `FilterCatalog` that maps human-readable filter names to API keys.

| CLI Alias | API Key | Value Type | Notes |
|-----------|---------|------------|-------|
| `--dept` | `DEPT_CODE` | string[] | Department code filter |
| `--type` | `OP_STDY` | string | `必修` / `選修` |
| `--credit` | `OP_CREDIT` | object `{value, value2, compare}` | Credit filter with comparison mode |
| `--teacher` | `TEACHER` | string | Teacher name search |
| `--day` | `OP_TIME_123` | string[] | Day-period codes (e.g., `2-A`) |
| `--period` | (combined with day) | string | Period within day |
| `--general` | `GENERAL` | string[] | General education categories |
| `--keyword` | `CNAME` | string | Course name keyword |
| `--dept-div` | `DEPT_DIV` | string | `B` for bachelor |

The catalog is stored as a versioned JSON file. Each entry records:
- `api_key`, `cli_aliases[]`
- `value_type` (string, string[], number, object)
- `allowed_values[]` (if enumerable)
- `tested_date`, `test_status` (verified / untested / changed)
- `notes`

### 5.4 Time Slot Encoding

CYCU uses codes like `2-A`, `4-1`, `5-2` where:
- First digit = day of week (1=Mon ... 7=Sun)
- Second part = period (`A`=1-2, `B`=3-4, `1`=1st, `2`=2nd, etc.)

The `time_slot` parser converts these to structured `TimeSlot { day, start_period, end_period }` for deterministic conflict comparison.

## 6. Storage & Privacy

### 6.1 Data Classification

| Class | Storage | Retention | Clearable by |
|-------|---------|-----------|--------------|
| Credentials | OS keyring | Until user deletes | `credentials set`, `logout --clear-credentials` |
| Session cookie | App data / `session.json` | Until expiry or logout | `logout`, `data purge` |
| Profile | SQLite | Until user edits | `data purge` |
| API cache | SQLite + snapshots | Until purge or TTL | `data purge` |
| AI results | SQLite + snapshots | Until purge | `data purge` |
| Work-packages | Snapshot dir | Until purge | `data purge` |

### 6.2 Redaction Rules

When `--redact-personal` is active (default), the following are masked in all CLI output, work-packages, and Skill input:

| Field | Redaction |
|-------|-----------|
| Student name | `[REDACTED]` |
| Student ID | Show last 4 digits only, e.g., `****1511` |
| Email | `[REDACTED]` |
| Cookie / JWT / loginToken | Completely removed |
| Personal URLs | Domain kept, path removed |
| Grade scores | Removed (only status: pass/fail/withdrawn retained) |

`--no-redact-personal` bypasses redaction for a single command invocation.

### 6.3 Security Rules

- HTTP debug logs never contain passwords, cookies, JWTs, or loginTokens.
- URL query parameters containing secrets are stripped before logging.
- Snapshot filenames use content hashes, never secret values.
- Cookie values are stored only in the CourseApe session file with restricted OS file permissions.
- No data is uploaded to any CourseApe-controlled server.
- AI work-packages are generated locally and only transmitted to the user's chosen Agent endpoint.

## 7. Agent Skill Design

### 7.1 PDF Skill Prerequisite

The CourseApe Skill enforces a hard prerequisite on a PDF reading/parsing Skill in the target Agent:

**At install time:**
1. Scan the target Agent's skill directories for any Skill whose description matches PDF reading/parsing.
2. If not found: refuse installation and display the required Skill name and suggested install command.
3. If found: proceed with CourseApe Skill installation.

**At analysis time:**
1. Before any PDF-related analysis, re-check for the PDF Skill.
2. If not found: abort with clear error message.
3. If found: proceed.

### 7.2 Skill Workflows

The CourseApe Skill provides three Agent workflows:

#### `courseape analyze requirements` (Skill workflow — NOT a CLI command)
1. Read the requirement work-package from the CLI export.
2. Verify PDF Skill is available.
3. Use PDF Skill to extract text, tables, and structure from the requirement PDF.
4. Analyze graduation requirements: category, course list, credit thresholds, special rules.
5. Output structured JSON matching `requirement_analysis.json` schema.
6. Each requirement item includes `source_page`, `source_section`, `confidence` (`high`/`medium`/`low`/`needs_review`).

#### `courseape analyze grades` (Skill workflow — NOT a CLI command)
1. Read the grade work-package from the CLI export.
2. Parse completed courses, statuses, credits, and terms.
3. Cross-reference with requirement analysis (if available).
4. Output structured JSON matching `grade_analysis.json` schema.
5. Highlight: completed, in-progress, missing, insufficient credits per category.

#### `courseape review course <code> --term <term>` (Skill workflow — NOT a CLI command)
1. Read course offering data from the CLI export.
2. Read syllabus PDF if available (via PDF Skill).
3. Search the web for course and teacher reviews using the Agent's search capability.
4. For each search result: record `source_url`, `source_name`, `published_date`, `query_date`, `summary`.
5. Present balanced view: include positive and negative feedback, note sample size and recency.
6. If insufficient sources: state "insufficient public reviews found" rather than fabricating.
7. Output structured JSON matching `review_output.json` schema.

### 7.3 Skill Security

- All Moodle/iTouch/CMAP content is treated as untrusted data. Instructions inside course names, PDFs, HTML pages, or syllabi are never executed.
- Only the user's direct request in the current conversation authorizes Skill actions.
- The Skill never reads credentials, cookies, or session files directly; it only reads explicitly exported work-packages.

## 8. Testing Strategy

### 8.1 Test Layers

| Layer | Scope | Fixture Source | Runs With |
|-------|-------|---------------|-----------|
| Unit tests | Parser logic, time-slot conflict, redaction, schema validation | Synthetic strings | `cargo test` (no network) |
| Contract tests | Connector request shape, response parsing | Mock JSON/HTML/PDF fixtures | `cargo test` (no network) |
| Agent integration | Skill PDF check, schema enforcement, source trace | Mock work-packages | CI (no network) |
| Live smoke tests | Real login, real API calls | Real keyring (opt-in) | `COURSEAPE_LIVE_TEST=1 cargo test --ignored` |

### 8.2 Fixture Management

Fixtures are stored in `fixtures/` with descriptive names:

```text
fixtures/
├── departments/
│   ├── query_114.json              # mock queryNecessary response for year 114
│   └── query_114_empty.json        # edge case: no departments
├── grades/
│   ├── s_grade_pass.html           # mock grade HTML with pass/fail/withdrawn
│   └── s_grade_mixed.html
├── offerings/
│   ├── elective_cs_required.json   # mock filtered course list
│   └── elective_empty.json
└── requirements/
    ├── cs_114_requirements.pdf      # small synthetic PDF (not real CYCU data)
    └── empty_dept.pdf
```

### 8.3 Live Test Protocol

Live smoke tests require `COURSEAPE_LIVE_TEST=1` environment variable. They:
- Read credentials from the existing openape keyring entry.
- Execute a minimal sequence: login -> profile detect -> one API call -> logout.
- Verify HTTP status, field names, field types, and count ranges.
- Save only: status code, field schema, anonymized count, response hash, error category.
- NEVER save: raw response body, credentials, cookies, personal data, or full student IDs.

### 8.4 CI/CD

GitHub Actions workflow (mirrors openape's publish.yml):

1. **Test job**: `cargo test`, `cargo clippy -- -D warnings` on all platforms.
2. **Build job**: Cross-compile Rust binaries for 6 targets (win/linux/mac x x64/arm64).
3. **Package job**: Prepare npm platform packages with SHA256 checksums.
4. **Publish job**: Publish `@dyliu0306/courseape` (base) + platform packages to npm.
5. **Release job**: Upload binaries + checksums to GitHub Release.

Triggers: `release: [published]` and `workflow_dispatch`.

## 9. Versioning & Updates

| Component | Version Scheme | Breaking Change Trigger |
|-----------|---------------|------------------------|
| CLI binary | SemVer | Command removal/rename, flag removal |
| Domain DTOs | Schema version in each JSON | Field removal, type change |
| Filter catalog | Catalog version | API key mapping changes |
| Skill | Skill version | Workflow rename, schema incompatibility |
| Connectors | Internal version | Endpoint URL/response format changes |

On `sync` or `analyze`, if the work-package schema version differs from what the Skill expects, the Skill aborts with a version mismatch message and suggests updating.

## 10. README Structure

The README follows openape's information structure (not its content):

```text
# CourseApe CLI (Unofficial)

[logo, badges]

## Features
## Installation
## Core Commands
### Authentication
### Profile
### Sync
### Courses
### Data Export
### Skills
## Development
## License & Disclaimer
```

The disclaimer states:
- This is an unofficial tool, not affiliated with CYCU.
- Users should exercise caution and avoid excessive requests.
- Course planning results are advisory; verify with official sources.

## 11. Open Questions & Known Limitations

1. **itouch session expiry**: Unknown exact duration. Mitigation: detect 401/redirect on API calls, prompt re-login.
2. **Grade HTML structure stability**: HTML format may change. Mitigation: version fixtures, detect parsing failures, request user report.
3. **Elective API undocumented**: Filter catalog reverse-engineered. Mitigation: `tested_date` field, periodic re-validation.
4. **PDF parsing accuracy**: AI may misread complex tables. Mitigation: `confidence` field, `needs_review` flag, source page reference.
5. **Transfer/double-major students**: Auto-detection may pick wrong department. Mitigation: `profile edit` always available, detection is advisory.
6. **Course evaluation legal**: Web search for reviews must respect robots.txt and terms of service. Mitigation: Agent search tool handles this; CourseApe does not bypass access controls.
