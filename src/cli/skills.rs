use crate::{Cli, SkillsCommands};

const SKILL_NAME: &str = "courseape";
const BUNDLED_SKILL: &str = include_str!("../../skills/courseape/SKILL.md");

struct Platform {
    name: &'static str,
    path: std::path::PathBuf,
}

fn platforms() -> Vec<(&'static str, Platform)> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        (
            "claude",
            Platform {
                name: "Claude Code",
                path: home.join(".claude").join("skills"),
            },
        ),
        (
            "codex",
            Platform {
                name: "Codex CLI",
                path: home.join(".codex").join("skills"),
            },
        ),
        (
            "opencode",
            Platform {
                name: "OpenCode",
                path: home.join(".opencode").join("skills"),
            },
        ),
    ]
}

fn check_pdf_skill(platform_path: &std::path::Path) -> bool {
    if !platform_path.exists() {
        return false;
    }
    // Scan for any skill directory containing SKILL.md whose description mentions PDF
    if let Ok(entries) = std::fs::read_dir(platform_path) {
        for entry in entries.flatten() {
            let skill_md = entry.path().join("SKILL.md");
            if skill_md.exists() {
                if let Ok(content) = std::fs::read_to_string(&skill_md) {
                    let lower = content.to_lowercase();
                    if lower.contains("pdf") && (lower.contains("read") || lower.contains("parse") || lower.contains("extract")) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub async fn run(cmd: &SkillsCommands, _cli: &Cli) -> anyhow::Result<()> {
    match cmd {
        SkillsCommands::Install { platform, all } => {
            let mut targets: Vec<Platform> = Vec::new();

            if *all {
                for (_, plat) in platforms() {
                    if plat.path.parent().is_some_and(|p| p.exists()) {
                        targets.push(plat);
                    }
                }
                if targets.is_empty() {
                    eprintln!("No supported agents detected. Supported: claude, codex, opencode");
                    return Ok(());
                }
            } else if let Some(ref p) = platform {
                let key = p.to_lowercase();
                let found = platforms()
                    .into_iter()
                    .find(|(k, _)| *k == key.as_str())
                    .map(|(_, plat)| plat);
                match found {
                    Some(plat) => targets.push(plat),
                    None => anyhow::bail!("Unknown platform: {}. Supported: claude, codex, opencode", p),
                }
            } else {
                anyhow::bail!("Specify a platform or use --all. Example: courseape skills install claude");
            }

            for plat in &targets {
                // Check PDF Skill prerequisite
                if !check_pdf_skill(&plat.path) {
                    eprintln!("ERROR: PDF Skill not found in {}.", plat.name);
                    eprintln!("CourseApe requires a PDF reading/parsing Skill to be installed first.");
                    eprintln!();
                    eprintln!("Install a PDF Skill first, then retry:");
                    eprintln!("  npx skills add <pdf-skill-package>");
                    eprintln!("  # or install manually to: {}", plat.path.display());
                    anyhow::bail!("PDF Skill prerequisite not met");
                }

                let dest_dir = plat.path.join(SKILL_NAME);
                tokio::fs::create_dir_all(&dest_dir).await?;
                tokio::fs::write(dest_dir.join("SKILL.md"), BUNDLED_SKILL).await?;
                eprintln!("  {} installed to {}", SKILL_NAME, plat.name);
            }

            Ok(())
        }
        SkillsCommands::Show => {
            println!("{}", BUNDLED_SKILL);
            Ok(())
        }
    }
}
