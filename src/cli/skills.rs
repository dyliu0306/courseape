use crate::{Cli, SkillsCommands};

const SKILL_NAME: &str = "courseape";
const BUNDLED_SKILL: &str = include_str!("../../skills/courseape/SKILL.md");

struct Platform {
    name: &'static str,
    path: std::path::PathBuf,
}

fn builtin_platforms() -> Vec<(&'static str, Platform)> {
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

fn detect_extra_platforms() -> Vec<Platform> {
    let home = dirs::home_dir().unwrap_or_default();
    let builtin_paths: Vec<_> = builtin_platforms()
        .into_iter()
        .map(|(_, p)| p.path)
        .collect();
    let candidates = [
        home.join(".agents").join("skills"),
        home.join(".config").join("opencode").join("skills"),
    ];
    let mut extra = Vec::new();
    for path in candidates {
        if builtin_paths.contains(&path) {
            continue;
        }
        if path.parent().is_some_and(|p| p.exists()) {
            let name: &'static str = match path.to_str() {
                Some(s) if s.contains(".agents") => "agents",
                _ => "extra",
            };
            extra.push(Platform { name, path });
        }
    }
    extra
}

pub async fn run(cmd: &SkillsCommands, _cli: &Cli) -> anyhow::Result<()> {
    match cmd {
        SkillsCommands::Install { platform, all } => {
            let mut targets: Vec<Platform> = Vec::new();

            if *all {
                for (_, plat) in builtin_platforms() {
                    if plat.path.parent().is_some_and(|p| p.exists()) {
                        targets.push(plat);
                    }
                }
                targets.extend(detect_extra_platforms());
                if targets.is_empty() {
                    eprintln!("No supported agents detected. Supported: claude, codex, opencode");
                    return Ok(());
                }
            } else if let Some(ref p) = platform {
                let key = p.to_lowercase();
                let found = builtin_platforms()
                    .into_iter()
                    .find(|(k, _)| *k == key.as_str())
                    .map(|(_, plat)| plat);
                match found {
                    Some(plat) => targets.push(plat),
                    None => anyhow::bail!(
                        "Unknown platform: {}. Supported: claude, codex, opencode",
                        p
                    ),
                }
            } else {
                anyhow::bail!(
                    "Specify a platform or use --all. Example: courseape skills install claude"
                );
            }

            let schemas: Vec<(&str, &[u8])> = vec![
                (
                    "grade_analysis.json",
                    include_bytes!("../../schemas/grade_analysis.json").as_slice(),
                ),
                (
                    "requirement_analysis.json",
                    include_bytes!("../../schemas/requirement_analysis.json").as_slice(),
                ),
                (
                    "review_output.json",
                    include_bytes!("../../schemas/review_output.json").as_slice(),
                ),
                (
                    "work_package.json",
                    include_bytes!("../../schemas/work_package.json").as_slice(),
                ),
            ];

            for plat in &targets {
                let dest_dir = plat.path.join(SKILL_NAME);
                tokio::fs::create_dir_all(&dest_dir).await?;
                tokio::fs::write(dest_dir.join("SKILL.md"), BUNDLED_SKILL).await?;
                let schemas_dir = dest_dir.join("schemas");
                tokio::fs::create_dir_all(&schemas_dir).await?;
                for (name, content) in &schemas {
                    tokio::fs::write(schemas_dir.join(name), *content).await?;
                }
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
