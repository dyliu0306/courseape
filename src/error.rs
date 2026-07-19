#[derive(Debug, thiserror::Error)]
pub enum CourseapeError {
    #[error("Not logged in. Run `courseape login` first.")]
    NotLoggedIn,

    #[allow(dead_code)]
    #[error("Session expired. Run `courseape login` to re-authenticate.")]
    SessionExpired,

    #[allow(dead_code)]
    #[error("PDF Skill not found in target Agent. Install a PDF reading/parsing skill first.")]
    PdfSkillMissing,

    #[error("Profile not set. Run `courseape profile edit` first.")]
    ProfileNotSet,
}
