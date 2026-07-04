use crate::error::{AppError, AppResult};
use chrono::Local;
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

const MAX_METADATA_LINES: usize = 160;
const PROTECTED_SKILL_DIRS: &[&str] = &[".system", "yonghu-preferences"];

#[derive(Debug, Clone)]
struct SkillRoots {
    skills: PathBuf,
    disabled: PathBuf,
    trash: PathBuf,
    plugins_cache: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SkillSourceKind {
    User,
    System,
    Plugin,
    Disabled,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SkillStatus {
    Enabled,
    Disabled,
    ReadOnly,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_kind: SkillSourceKind,
    pub source_label: String,
    pub status: SkillStatus,
    pub folder_path: String,
    pub skill_file_path: String,
    pub can_enable: bool,
    pub can_disable: bool,
    pub can_delete: bool,
    pub can_open_folder: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBoard {
    pub refreshed_at: String,
    pub total_count: usize,
    pub skills: Vec<SkillSummary>,
    pub messages: Vec<String>,
}

pub fn get_skill_board() -> AppResult<SkillBoard> {
    build_skill_board(&default_roots()?)
}

pub fn disable_skill(skill_id: &str) -> AppResult<SkillBoard> {
    let roots = default_roots()?;
    disable_skill_with_roots(&roots, skill_id)
}

fn disable_skill_with_roots(roots: &SkillRoots, skill_id: &str) -> AppResult<SkillBoard> {
    let skill = resolve_actionable_skill(&roots, skill_id)?;
    if !skill.can_disable {
        return Err(AppError::Config("该技能不允许禁用".to_string()));
    }

    fs::create_dir_all(&roots.disabled)?;
    let destination = unique_destination(&roots.disabled, folder_name(&skill.folder_path)?);
    fs::rename(Path::new(&skill.folder_path), destination)?;
    build_skill_board(&roots)
}

pub fn enable_skill(skill_id: &str) -> AppResult<SkillBoard> {
    let roots = default_roots()?;
    enable_skill_with_roots(&roots, skill_id)
}

fn enable_skill_with_roots(roots: &SkillRoots, skill_id: &str) -> AppResult<SkillBoard> {
    let skill = resolve_existing_skill(&roots, skill_id)?;
    if !skill.can_enable {
        return Err(AppError::Config("该技能不允许启用".to_string()));
    }

    let folder = fs::canonicalize(&skill.folder_path)?;
    let disabled_root = fs::canonicalize(&roots.disabled)?;
    if !folder.starts_with(&disabled_root) {
        return Err(AppError::Config(
            "仅允许启用已禁用技能目录下的技能".to_string(),
        ));
    }

    fs::create_dir_all(&roots.skills)?;
    let destination = unique_destination(&roots.skills, folder_name(&skill.folder_path)?);
    fs::rename(Path::new(&skill.folder_path), destination)?;
    build_skill_board(&roots)
}

pub fn archive_skill(skill_id: &str) -> AppResult<SkillBoard> {
    let roots = default_roots()?;
    let skill = resolve_actionable_skill(&roots, skill_id)?;
    if !skill.can_delete {
        return Err(AppError::Config("该技能不允许删除".to_string()));
    }

    fs::create_dir_all(&roots.trash)?;
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let folder_name = folder_name(&skill.folder_path)?;
    let destination = unique_destination(&roots.trash, &format!("{timestamp}-{folder_name}"));
    fs::rename(Path::new(&skill.folder_path), destination)?;
    build_skill_board(&roots)
}

pub fn open_skill_folder(skill_id: &str) -> AppResult<String> {
    let roots = default_roots()?;
    let skill = resolve_existing_skill(&roots, skill_id)?;
    let folder = PathBuf::from(&skill.folder_path);
    open_path(&folder)?;
    Ok(skill.folder_path)
}

fn default_roots() -> AppResult<SkillRoots> {
    let home =
        dirs::home_dir().ok_or_else(|| AppError::Config("无法定位用户主目录".to_string()))?;
    let codex = home.join(".codex");
    Ok(SkillRoots {
        skills: codex.join("skills"),
        disabled: codex.join("skills-disabled"),
        trash: codex.join("skills-trash"),
        plugins_cache: codex.join("plugins").join("cache"),
    })
}

fn build_skill_board(roots: &SkillRoots) -> AppResult<SkillBoard> {
    let mut messages = Vec::new();
    let mut skills = Vec::new();
    let mut seen = HashSet::new();

    collect_skill_files(&roots.skills, &mut |skill_file| match summarize_skill(
        roots,
        &skill_file,
        SkillRootClass::Skills,
    ) {
        Ok(skill) => {
            if seen.insert(skill.id.clone()) {
                skills.push(skill);
            }
        }
        Err(err) => messages.push(format!("读取技能失败 {}: {err}", skill_file.display())),
    });

    collect_skill_files(&roots.disabled, &mut |skill_file| match summarize_skill(
        roots,
        &skill_file,
        SkillRootClass::Disabled,
    ) {
        Ok(skill) => {
            if seen.insert(skill.id.clone()) {
                skills.push(skill);
            }
        }
        Err(err) => messages.push(format!(
            "读取已禁用技能失败 {}: {err}",
            skill_file.display()
        )),
    });

    collect_skill_files(&roots.plugins_cache, &mut |skill_file| {
        if !is_plugin_skill_file(&roots.plugins_cache, &skill_file) {
            return;
        }
        match summarize_skill(roots, &skill_file, SkillRootClass::Plugin) {
            Ok(skill) => {
                if seen.insert(skill.id.clone()) {
                    skills.push(skill);
                }
            }
            Err(err) => messages.push(format!("读取插件技能失败 {}: {err}", skill_file.display())),
        }
    });

    skills.sort_by(|left, right| {
        source_rank(&left.source_kind)
            .cmp(&source_rank(&right.source_kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(SkillBoard {
        refreshed_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        total_count: skills.len(),
        skills,
        messages,
    })
}

#[derive(Debug, Clone, Copy)]
enum SkillRootClass {
    Skills,
    Disabled,
    Plugin,
}

fn summarize_skill(
    roots: &SkillRoots,
    skill_file: &Path,
    root_class: SkillRootClass,
) -> AppResult<SkillSummary> {
    let metadata = read_skill_metadata(skill_file)?;
    let folder = skill_file
        .parent()
        .ok_or_else(|| AppError::Config("技能文件缺少父目录".to_string()))?;
    let (id_prefix, source_kind, source_label, status, can_enable, can_disable, can_delete) =
        match root_class {
            SkillRootClass::Skills => {
                if is_under(&roots.skills.join(".system"), folder)
                    || is_protected_user_skill(&roots.skills, folder)
                {
                    (
                        "system",
                        SkillSourceKind::System,
                        "系统技能",
                        SkillStatus::ReadOnly,
                        false,
                        false,
                        false,
                    )
                } else {
                    (
                        "user",
                        SkillSourceKind::User,
                        "用户技能",
                        SkillStatus::Enabled,
                        false,
                        true,
                        true,
                    )
                }
            }
            SkillRootClass::Disabled => (
                "disabled",
                SkillSourceKind::Disabled,
                "已禁用",
                SkillStatus::Disabled,
                true,
                false,
                false,
            ),
            SkillRootClass::Plugin => (
                "plugin",
                SkillSourceKind::Plugin,
                "插件技能",
                SkillStatus::ReadOnly,
                false,
                false,
                false,
            ),
        };
    let base = match root_class {
        SkillRootClass::Skills => &roots.skills,
        SkillRootClass::Disabled => &roots.disabled,
        SkillRootClass::Plugin => &roots.plugins_cache,
    };
    let id = format!("{id_prefix}:{}", relative_id(base, folder));
    let name = metadata
        .name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback_name(folder));

    Ok(SkillSummary {
        id,
        name,
        description: metadata
            .description
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "暂无技能描述。".to_string()),
        source_kind,
        source_label: source_label.to_string(),
        status,
        folder_path: folder.to_string_lossy().to_string(),
        skill_file_path: skill_file.to_string_lossy().to_string(),
        can_enable,
        can_disable,
        can_delete,
        can_open_folder: true,
    })
}

#[derive(Default)]
struct SkillMetadata {
    name: Option<String>,
    description: Option<String>,
}

fn read_skill_metadata(skill_file: &Path) -> AppResult<SkillMetadata> {
    let file = fs::File::open(skill_file)?;
    let reader = BufReader::new(file);
    let mut metadata = SkillMetadata::default();
    let mut in_frontmatter = false;
    let mut saw_opening_marker = false;

    for (index, line) in reader.lines().take(MAX_METADATA_LINES).enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if index == 0 && trimmed == "---" {
            in_frontmatter = true;
            saw_opening_marker = true;
            continue;
        }
        if in_frontmatter && trimmed == "---" {
            break;
        }

        if in_frontmatter {
            if let Some(value) = parse_frontmatter_value(trimmed, "name") {
                metadata.name = Some(value);
            } else if let Some(value) = parse_frontmatter_value(trimmed, "description") {
                metadata.description = Some(value);
            }
            continue;
        }

        if !saw_opening_marker && metadata.name.is_none() && trimmed.starts_with("# ") {
            metadata.name = Some(trimmed.trim_start_matches("# ").trim().to_string());
        }
    }

    Ok(metadata)
}

fn parse_frontmatter_value(line: &str, key: &str) -> Option<String> {
    let (left, right) = line.split_once(':')?;
    if left.trim() != key {
        return None;
    }
    let value = right
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();
    Some(value)
}

fn collect_skill_files(root: &Path, visit: &mut impl FnMut(PathBuf)) {
    if !root.exists() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_skill_files(&path, visit);
        } else if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
            visit(path);
        }
    }
}

fn resolve_existing_skill(roots: &SkillRoots, skill_id: &str) -> AppResult<SkillSummary> {
    build_skill_board(roots)?
        .skills
        .into_iter()
        .find(|skill| skill.id == skill_id)
        .ok_or_else(|| AppError::Config("未找到指定技能，可能已经被移动或删除".to_string()))
}

fn resolve_actionable_skill(roots: &SkillRoots, skill_id: &str) -> AppResult<SkillSummary> {
    let skill = resolve_existing_skill(roots, skill_id)?;
    let folder = fs::canonicalize(&skill.folder_path)?;
    let skills_root = fs::canonicalize(&roots.skills)?;
    if !folder.starts_with(&skills_root) {
        return Err(AppError::Config(
            "仅允许操作用户技能目录下的技能".to_string(),
        ));
    }
    Ok(skill)
}

fn is_plugin_skill_file(plugin_cache_root: &Path, skill_file: &Path) -> bool {
    let Ok(relative) = skill_file.strip_prefix(plugin_cache_root) else {
        return false;
    };
    let parts: Vec<_> = relative.components().collect();
    parts
        .iter()
        .any(|part| part.as_os_str().to_string_lossy() == "skills")
}

fn is_protected_user_skill(skills_root: &Path, folder: &Path) -> bool {
    let Ok(relative) = folder.strip_prefix(skills_root) else {
        return true;
    };
    let Some(first) = relative.components().next() else {
        return true;
    };
    let first = first.as_os_str().to_string_lossy();
    PROTECTED_SKILL_DIRS
        .iter()
        .any(|protected| *protected == first)
}

fn is_under(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
}

fn relative_id(base: &Path, folder: &Path) -> String {
    folder
        .strip_prefix(base)
        .unwrap_or(folder)
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn fallback_name(folder: &Path) -> String {
    folder
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("未命名技能")
        .to_string()
}

fn folder_name(folder: &str) -> AppResult<&str> {
    Path::new(folder)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::Config("无法识别技能目录名称".to_string()))
}

fn unique_destination(base: &Path, name: &str) -> PathBuf {
    let mut destination = base.join(name);
    if !destination.exists() {
        return destination;
    }
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    destination = base.join(format!("{timestamp}-{name}"));
    let mut counter = 2;
    while destination.exists() {
        destination = base.join(format!("{timestamp}-{counter}-{name}"));
        counter += 1;
    }
    destination
}

fn source_rank(source: &SkillSourceKind) -> u8 {
    match source {
        SkillSourceKind::User => 0,
        SkillSourceKind::Disabled => 1,
        SkillSourceKind::System => 2,
        SkillSourceKind::Plugin => 3,
    }
}

fn open_path(path: &Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        Command::new("explorer").arg(path).spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_skill(root: &Path, name: &str, description: &str) -> PathBuf {
        let skill_dir = root.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n\n# Body\n\nsecret body"),
        )
        .unwrap();
        skill_dir
    }

    fn test_roots(base: &Path) -> SkillRoots {
        SkillRoots {
            skills: base.join("skills"),
            disabled: base.join("skills-disabled"),
            trash: base.join("skills-trash"),
            plugins_cache: base.join("plugins").join("cache"),
        }
    }

    #[test]
    fn reads_frontmatter_without_body() {
        let temp = tempdir().unwrap();
        let roots = test_roots(temp.path());
        write_skill(&roots.skills, "demo", "Demo description");

        let board = build_skill_board(&roots).unwrap();
        assert_eq!(board.total_count, 1);
        assert_eq!(board.skills[0].name, "demo");
        assert_eq!(board.skills[0].description, "Demo description");
        assert!(!board.skills[0].description.contains("secret body"));
    }

    #[test]
    fn protects_system_plugin_and_preferences_skills() {
        let temp = tempdir().unwrap();
        let roots = test_roots(temp.path());
        write_skill(
            &roots.skills.join(".system"),
            "system-demo",
            "System description",
        );
        write_skill(
            &roots.skills,
            "yonghu-preferences",
            "Preference description",
        );
        write_skill(
            &roots
                .plugins_cache
                .join("openai-curated")
                .join("demo")
                .join("skills"),
            "plugin-demo",
            "Plugin description",
        );

        let board = build_skill_board(&roots).unwrap();
        assert_eq!(board.total_count, 3);
        assert!(board
            .skills
            .iter()
            .all(|skill| !skill.can_disable && !skill.can_delete));
    }

    #[test]
    fn archives_user_skill_to_trash() {
        let temp = tempdir().unwrap();
        let roots = test_roots(temp.path());
        let skill_dir = write_skill(&roots.skills, "demo", "Demo description");
        let skill = build_skill_board(&roots).unwrap().skills.remove(0);

        let destination =
            unique_destination(&roots.trash, folder_name(&skill.folder_path).unwrap());
        fs::create_dir_all(&roots.trash).unwrap();
        fs::rename(&skill_dir, &destination).unwrap();

        assert!(destination.join("SKILL.md").exists());
        assert!(!skill_dir.exists());
    }

    #[test]
    fn enables_disabled_skill_to_user_skills() {
        let temp = tempdir().unwrap();
        let roots = test_roots(temp.path());
        let disabled_dir = write_skill(&roots.disabled, "old-demo", "Disabled description");
        let skill = build_skill_board(&roots).unwrap().skills.remove(0);

        assert!(skill.can_enable);
        fs::create_dir_all(&roots.skills).unwrap();
        let destination =
            unique_destination(&roots.skills, folder_name(&skill.folder_path).unwrap());
        fs::rename(&disabled_dir, &destination).unwrap();

        assert!(destination.join("SKILL.md").exists());
        assert!(!disabled_dir.exists());
    }

    #[test]
    fn disables_then_enables_user_skill_roundtrip() {
        let temp = tempdir().unwrap();
        let roots = test_roots(temp.path());
        write_skill(&roots.skills, "demo", "Demo description");

        let disabled_board = disable_skill_with_roots(&roots, "user:demo").unwrap();
        assert!(roots.disabled.join("demo").join("SKILL.md").exists());
        assert!(!roots.skills.join("demo").exists());
        let disabled_skill = disabled_board
            .skills
            .iter()
            .find(|skill| skill.id == "disabled:demo")
            .unwrap();
        assert!(disabled_skill.can_enable);
        assert!(!disabled_skill.can_disable);

        let enabled_board = enable_skill_with_roots(&roots, "disabled:demo").unwrap();
        assert!(roots.skills.join("demo").join("SKILL.md").exists());
        assert!(!roots.disabled.join("demo").exists());
        let enabled_skill = enabled_board
            .skills
            .iter()
            .find(|skill| skill.id == "user:demo")
            .unwrap();
        assert!(enabled_skill.can_disable);
        assert!(!enabled_skill.can_enable);
    }
}
