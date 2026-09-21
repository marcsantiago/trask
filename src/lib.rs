use anyhow::{Context, Result, bail};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const TASKS_DIR: &str = "tasks";
pub const TASK_FILE: &str = "TASK.md";
pub const TRASK_FILE: &str = ".trask";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Open,
    Closed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Open => "OPEN",
            Self::Closed => "CLOSED",
        }
    }
}

#[derive(Debug)]
pub struct Task {
    pub title: String,
    pub status: TaskStatus,
    pub priority: u32,
    pub tags: Vec<String>,
    pub description: String,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected end of input")]
    UnexpectedEnd,

    #[error("invalid format: {0}")]
    InvalidFormat(String),

    #[error("invalid priority")]
    InvalidPriority,

    #[error("invalid status: {0}")]
    InvalidStatus(String),
}

impl Task {
    pub fn from_markdown(input: &str) -> Result<Self, ParseError> {
        let mut lines = input.lines();

        let title = lines
            .next()
            .ok_or(ParseError::UnexpectedEnd)?
            .strip_prefix("# ")
            .ok_or_else(|| ParseError::InvalidFormat("expected '# <title>'".into()))?
            .to_string();

        expect_blank(&mut lines)?;

        let status = parse_status(&parse_field(&mut lines, "- STATUS: ")?);

        let priority = parse_field(&mut lines, "- PRIORITY: ")?
            .parse::<u32>()
            .map_err(|_| ParseError::InvalidPriority)?;

        let tags = parse_field(&mut lines, "- TAGS: ")?
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(String::from)
            .collect();

        expect_blank(&mut lines)?;

        let description_header = lines.next().ok_or(ParseError::UnexpectedEnd)?;

        if description_header != "# Description" {
            return Err(ParseError::InvalidFormat("expected '# Description'".into()));
        }

        expect_blank(&mut lines)?;

        let description = lines.collect::<Vec<_>>().join("\n");

        Ok(Self {
            title,
            status,
            priority,
            tags,
            description,
        })
    }

    pub fn to_markdown(&self) -> String {
        let tags = self.tags.join(", ");

        format!(
            "# {}\n\n\
             - STATUS: {}\n\
             - PRIORITY: {}\n\
             - TAGS: {}\n\n\
             # Description\n\n\
             {}\n",
            self.title,
            self.status.as_str(),
            self.priority,
            tags,
            self.description,
        )
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        Ok(Self::from_markdown(&contents)?)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, self.to_markdown())?;
        Ok(())
    }
}

fn parse_status(value: &str) -> TaskStatus {
    match value {
        "OPEN" | "open" => TaskStatus::Open,
        "CLOSED" | "closed" => TaskStatus::Closed,
        _ => TaskStatus::Open,
    }
}

fn parse_field<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    prefix: &str,
) -> Result<String, ParseError> {
    lines
        .next()
        .ok_or(ParseError::UnexpectedEnd)?
        .strip_prefix(prefix)
        .map(str::trim)
        .map(String::from)
        .ok_or_else(|| ParseError::InvalidFormat(format!("expected '{prefix}<value>'")))
}

fn expect_blank<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Result<(), ParseError> {
    match lines.next() {
        Some("") => Ok(()),
        Some(_) => Err(ParseError::InvalidFormat("expected blank line".into())),
        None => Err(ParseError::UnexpectedEnd),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn generate(store: &TaskStore) -> Result<Self> {
        loop {
            let id = Utc::now().format("%Y%m%d-%H%M%S").to_string();
            let task_id = Self::new(id);

            if !store.task_dir(&task_id).exists() {
                return Ok(task_id);
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct TaskEntry {
    pub id: TaskId,
    pub task: Task,
}

pub struct TaskStore {
    root: PathBuf,
}

impl TaskStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().canonicalize()?;
        let marker = root.join(TRASK_FILE);

        if marker.exists() {
            bail!("trask is already initialized in {}", root.display());
        }

        fs::write(&marker, "")?;
        fs::create_dir_all(root.join(TASKS_DIR))?;

        Ok(Self::new(root))
    }

    pub fn discover() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let root = find_root(&current_dir)?;

        Ok(Self::new(root))
    }

    pub fn tasks_dir(&self) -> PathBuf {
        self.root.join(TASKS_DIR)
    }

    pub fn task_dir(&self, id: &TaskId) -> PathBuf {
        self.tasks_dir().join(id.as_str())
    }

    fn task_path(&self, id: &TaskId) -> PathBuf {
        self.task_dir(id).join(TASK_FILE)
    }

    pub fn load(&self, id: &TaskId) -> Result<TaskEntry> {
        let task = Task::load(self.task_path(id))?;

        Ok(TaskEntry {
            id: id.clone(),
            task,
        })
    }

    pub fn save(&self, entry: &TaskEntry) -> Result<()> {
        let task_dir = self.task_dir(&entry.id);

        fs::create_dir_all(&task_dir)?;
        entry.task.save(task_dir.join(TASK_FILE))?;

        Ok(())
    }

    pub fn delete(&self, id: &TaskId) -> Result<()> {
        let p = self.task_dir(id);
        fs::remove_dir_all(p)?;
        Ok(())
    }
}

fn find_root(start: &Path) -> Result<PathBuf> {
    let mut current = start;

    loop {
        if current.join(TRASK_FILE).is_file() {
            return Ok(current.to_path_buf());
        }

        current = current.parent().context("could not find a trask project")?;
    }
}
