use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use trask::{Task, TaskEntry, TaskId, TaskStatus, TaskStore};

#[derive(Debug, Parser)]
#[command(name = "trask")]
#[command(about = "A simple Markdown task tracker")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,

    #[command(alias = "n", alias = "add", alias = "a")]
    New {
        title: String,
    },

    #[command(alias = "s")]
    Show {
        id: String,
    },

    #[command(alias = "l")]
    List {
        #[arg(short = 's', long, value_enum, default_value_t = Sort::Priority)]
        sort: Sort,

        #[arg(short, long)]
        reverse: bool,

        #[arg(short, long)]
        tag: Vec<String>,

        #[arg(short, long)]
        closed: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Sort {
    #[value(name = "priority", alias = "p")]
    Priority,

    #[value(name = "id", alias = "i")]
    Id,

    #[value(name = "title", alias = "t")]
    Title,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => init()?,
        Command::New { title } => new_task(&title)?,
        Command::Show { id } => show_task(&id)?,
        Command::List {
            sort,
            reverse,
            tag,
            closed,
        } => list_tasks(sort, reverse, &tag, closed)?,
    }

    Ok(())
}

fn init() -> Result<()> {
    TaskStore::init(".")?;

    println!("initialized trask");

    Ok(())
}

fn new_task(title: &str) -> Result<()> {
    let store = TaskStore::discover()?;
    let id = TaskId::generate(&store)?;

    let task = Task {
        title: title.to_string(),
        status: TaskStatus::Open,
        priority: u32::MAX,
        tags: Vec::new(),
        description: String::new(),
    };

    store.save(&TaskEntry { id, task })?;

    Ok(())
}

fn show_task(id: &str) -> Result<()> {
    let store = TaskStore::discover()?;
    let entry = store.load(&TaskId::new(id))?;
    let status = match entry.task.status {
        TaskStatus::Open => "OPEN",
        TaskStatus::Closed => "CLOSED",
    };

    println!("ID:       {}", entry.id.as_str());
    println!("Title:    {}", entry.task.title);
    println!("Status:   {}", status);
    println!("Priority: {}", entry.task.priority);
    println!("Tags:     {}", entry.task.tags.join(", "));
    println!();
    println!("# Description");
    println!();
    println!("{}", entry.task.description);

    Ok(())
}

fn list_tasks(sort: Sort, reverse: bool, tags: &[String], closed: bool) -> Result<()> {
    let store = TaskStore::discover()?;
    let mut tasks = Vec::new();

    for entry in std::fs::read_dir(store.tasks_dir())? {
        let entry = entry?;

        if !entry.path().is_dir() {
            continue;
        }

        let Some(id) = entry.file_name().to_str().map(String::from) else {
            continue;
        };

        let task_id = TaskId::new(id);

        if let Ok(task) = store.load(&task_id)
            && (closed || task.task.status == TaskStatus::Open)
            && matches_tags(&task.task, tags)
        {
            tasks.push(task);
        }
    }

    tasks.sort_by(|a, b| {
        let ordering = match sort {
            Sort::Priority => b.task.priority.cmp(&a.task.priority),
            Sort::Id => a.id.as_str().cmp(b.id.as_str()),
            Sort::Title => a.task.title.cmp(&b.task.title),
        };

        if reverse {
            ordering.reverse()
        } else {
            ordering
        }
    });

    for entry in tasks {
        let status = match entry.task.status {
            TaskStatus::Open => "OPEN",
            TaskStatus::Closed => "CLOSED",
        };

        println!(
            "{} [{}] [{}] {}",
            entry.id.as_str(),
            status,
            entry.task.priority,
            entry.task.title
        );
    }
    Ok(())
}

fn matches_tags(task: &Task, tags: &[String]) -> bool {
    tags.is_empty()
        || tags
            .iter()
            .all(|tag| task.tags.iter().any(|task_tag| task_tag == tag))
}
