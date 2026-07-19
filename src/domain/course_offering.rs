use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseOffering {
    // ── Core ───────────────────────────────────────────────────
    pub code: String,           // OP_CODE
    pub name: String,           // CURS_NM_C_S
    pub name_en: String,        // CURS_NM_E_A
    pub teacher: String,        // TEACHER_CNAME
    pub teacher_id: String,     // IDCODE (人事代碼)
    pub credits: u32,           // OP_CREDIT
    pub category: String,       // OP_STDY (必修/選修)

    // ── Department ────────────────────────────────────────────
    pub dept_code: String,      // AUTHORITY_DEPT (權責系所代碼)
    pub dept_name: String,      // AUTHORITY_NAME (權責系所名稱)
    pub class_dept: String,     // DEPT_CODE (班級代碼)
    pub class_dept_name: String,// DEPT_ABVI_C (班級簡稱)
    pub admin_dept: String,     // DEPT_BLN_ADMIN (所屬系所admin)
    pub admin_dept_name: String,// ADMIN_DEPT_NAME

    // ── Time & Location ───────────────────────────────────────
    pub time_slots: Vec<String>, // OP_TIME_1/2/3
    pub classroom: String,      // CLS_NAME_1

    // ── Capacity ──────────────────────────────────────────────
    pub max_capacity: Option<i32>,  // OP_MAN
    pub enrolled: Option<i32>,      // ACT_MAN
    pub remaining: Option<i32>,     // ACT_REMAIN

    // ── Category flags ────────────────────────────────────────
    pub div: String,            // DIV (部別: B=學士, M=碩士, D=博士, H=學士後)
    pub course_type: String,    // OP_TYPE (課程類別)
    pub language: String,       // CURS_LANG (授課語言)
    pub is_emi: bool,           // ALL_ENGLISH 包含 "全英語"
    pub is_english: bool,       // ENG_COURSE 非空
    pub is_distance: bool,      // DISTANCE 包含 "遠距"
    pub is_pbl: bool,           // CROSS_PBL 包含 "PBL"
    pub is_programming: bool,   // CROSS_PROG 包含 "程式設計"
    pub sdgs: String,           // SDGS
    pub spec: String,           // SPEC (課程特色)
    pub cross_name: String,     // CROSS_NAME (跨系/聯盟名稱)
    pub memo: String,           // MEMO1 (備註)

    // ── Status flags ──────────────────────────────────────────
    pub is_stop: bool,          // IS_STOP 包含 "停修" 或 "不可"
    pub auto_set: bool,         // AUTOSET == "V"
    pub semester_half: String,  // TCH_T_COUNT < OP_T_COUNT 時為半學期

    // ── Clock hours ───────────────────────────────────────────
    pub op_clock: f64,          // OP_T_COUNT (開課鐘點數)
    pub tch_clock: f64,         // TCH_T_COUNT (教師授課鐘點數)
    pub op_type: String,        // OP_TYPE (通識向度: 天/人/物/我/宗哲/人哲/公民/歷史/文學/科學/科技/一般/體育/英文/...)
    pub cos_usr: String,        // COS_USR (SDGs/領域 JSON)
}
