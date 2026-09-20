# Trask

A simple, Git-friendly task tracker that stores tasks as Markdown files.

## Installation

### From Source

Trask can be built and installed using Cargo.

Clone the repository:

```bash
git clone <repository-url>
cd trask
```

Install Trask:

```bash
cargo install --path .
```

This installs the `trask` executable into Cargo's binary directory.

Verify the installation:

```bash
trask --help
```

If `trask` is not found after installation, make sure Cargo's binary directory is in your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Build Without Installing

To build Trask without installing it:

```bash
cargo build
```

The debug executable will be available at:

```text
target/debug/trask
```

Run it directly with:

```bash
cargo run -- --help
```

### Release Build

For an optimized release build:

```bash
cargo build --release
```

The release executable will be:

```text
target/release/trask
```

You can run it directly:

```bash
./target/release/trask --help
```

Or install the optimized release binary:

```bash
cargo install --path .
```

### Trask LSP

Trask includes an optional language server for editor integration.

Install the LSP binary with:

```bash
cargo install --path . --bin trask-lsp
```

Verify the installation:

```bash
trask-lsp
```

The language server communicates over standard input/output and is intended to be started by an editor.

### Helix

To use the Trask LSP with Helix, add the following to:

```text
~/.config/helix/languages.toml
```

```toml
[language-server.trask-lsp]
command = "trask-lsp"

[[language]]
name = "markdown"
file-types = [
    "md",
    { glob = "TASK.md" },
]
language-servers = ["trask-lsp"]
```

This keeps `TASK.md` as Markdown while allowing the Trask LSP to provide task-specific completion and diagnostics.

Restart Helix or restart the language server after changing the configuration:

```text
:lsp-restart
```

### Requirements

Trask is a Rust application and requires a current Rust toolchain with Cargo.

Check that Rust and Cargo are installed:

```bash
rustc --version
cargo --version
```

If you need to install Rust, use `rustup`.

## Inspiration

This project was inspired by [Tsoding's Tatr](https://github.com/tsoding/tatr), an improvised task-tracking system designed to be more powerful than source-code TODOs without requiring a full issue tracker.

The task directory layout and `TASK.md` concept are directly inspired by Tatr:

```text
tasks/
└── <task-id>/
    └── TASK.md
```

This project is an independent Rust implementation with its own design and implementation.

Thanks to **Alexey Kutepov (Tsoding)** for the original idea and inspiration.

## Layout

Tasks live in a `tasks/` directory at the root of your project.

Trask uses a `.trask` marker file to identify the project root. Commands can therefore be run from the project root or from a subdirectory within the project.

Each task gets its own directory named with its unique task ID. Every task directory contains a mandatory `TASK.md` file.

Task IDs are generated using the UTC timestamp at creation time:

```text
YYYYMMDD-HHMMSS
```

For example:

```text
project/
├── .trask
├── tasks/
│   ├── 20260919-154500/
│   │   └── TASK.md
│   └── 20260919-160200/
│       └── TASK.md
└── ...
```

Task directories can also contain attachments such as screenshots or other files.

## Initialize

Run `init` from the root of your project:

```bash
trask init
```

This creates:

```text
.trask
tasks/
```

The `.trask` file identifies the project root. Once initialized, Trask commands can also be run from subdirectories.

Running `init` on an already initialized project will return an error.

## Create a Task

Create a task with:

```bash
trask new "Fix ad request timeout"
```

The `new` command also has the following aliases:

```bash
trask n "Fix ad request timeout"
trask add "Fix ad request timeout"
trask a "Fix ad request timeout"
```

Trask automatically generates a unique timestamp-based task ID.

For example:

```text
tasks/
└── 20260919-154500/
    └── TASK.md
```

The generated `TASK.md` looks like:

```markdown
# Fix ad request timeout

- STATUS: OPEN
- PRIORITY: 4294967295
- TAGS:

# Description

```

The default status is `OPEN`.

The default priority is `u32::MAX` (`4294967295`), which causes newly created tasks to appear first when using the default priority sort.

Edit the `TASK.md` file directly to add or modify the task description.

## Task Format

Each task contains a `TASK.md` file with the following format:

```markdown
# Task title

- STATUS: OPEN
- PRIORITY: 50
- TAGS: rtb, exchange, timeout

# Description

Task description goes here.

The description can contain normal Markdown.
```

### Status

`STATUS` describes whether the task is open or closed.

```text
OPEN
CLOSED
```

Trask represents these statuses internally using the `TaskStatus` enum.

A newly created task always starts with:

```text
OPEN
```

To close a task, edit its `TASK.md`:

```markdown
- STATUS: CLOSED
```

When using `trask list`, closed tasks are excluded by default.

Use `--closed` or `-c` to include closed tasks.

### Priority

`PRIORITY` is an unsigned integer.

```text
- PRIORITY: 50
```

The default priority for newly created tasks is `4294967295` (`u32::MAX`).

When listing tasks, priority is sorted in descending order by default, so higher-priority tasks appear first.

### Tags

`TAGS` is a comma-separated list:

```text
- TAGS: rust, helix, tooling
```

Whitespace around tags is ignored.

An empty tag list is valid:

```text
- TAGS:
```

### Description

The `# Description` heading marks the beginning of the task description:

```markdown
# Description

Task description goes here.
```

Everything after the `# Description` heading is treated as the task description and may contain normal Markdown.

## List Tasks

List open tasks:

```bash
trask list
```

You can also use the shorthand:

```bash
trask l
```

By default, closed tasks are excluded and tasks are sorted by priority in descending order.

Example:

```text
20260919-160200 [OPEN] [4294967295] Add Helix integration
20260919-154500 [OPEN] [100] Fix ad request timeout
```

A closed task such as:

```text
20260919-161030 [CLOSED] [10] Initial project setup
```

is not shown by default.

### Include Closed Tasks

Use `--closed` or `-c` to include closed tasks:

```bash
trask list --closed
```

or:

```bash
trask l -c
```

Example:

```text
20260919-160200 [OPEN] [4294967295] Add Helix integration
20260919-154500 [OPEN] [100] Fix ad request timeout
20260919-161030 [CLOSED] [10] Initial project setup
```

The `--closed` option does not mean "show only closed tasks." It means **include closed tasks in the results**.

### Sorting

Use `--sort` or `-s` to choose the sort order:

```bash
trask list --sort priority
trask list -s priority
```

The available sort values are:

| Sort     | Long form  | Short value |
| -------- | ---------- | ----------- |
| Priority | `priority` | `p`         |
| ID       | `id`       | `i`         |
| Title    | `title`    | `t`         |

#### Sort by Priority

Priority is the default sort:

```bash
trask list --sort priority
trask list --sort p
trask l -s p
```

Higher priorities appear first.

#### Sort by ID

Sort tasks by their timestamp-based ID:

```bash
trask list --sort id
trask list --sort i
trask l -s i
```

Example:

```text
20260919-154500 [OPEN] [100] Fix ad request timeout
20260919-160200 [OPEN] [4294967295] Add Helix integration
20260919-161030 [CLOSED] [10] Initial project setup
```

#### Sort by Title

Sort tasks alphabetically by title:

```bash
trask list --sort title
trask list --sort t
trask l -s t
```

### Reverse the Sort

Use `--reverse` or `-r` to reverse whichever sort is selected:

```bash
trask list --reverse
trask list -r
```

For example:

```bash
trask l -s p -r
```

reverses the default priority ordering.

You can also combine reverse with the other sort options:

```bash
trask l -s i -r
trask l -s t -r
```

### Filter by Tag

Use `--tag` or `-t` to show only tasks containing a specific tag:

```bash
trask list --tag rust
```

or:

```bash
trask l -t rust
```

For example, given:

```markdown
# Fix ad request timeout

- STATUS: OPEN
- PRIORITY: 100
- TAGS: rtb, exchange

# Description

Investigate why ad requests occasionally exceed the 100ms timeout.
```

```markdown
# Add Rust parser

- STATUS: OPEN
- PRIORITY: 50
- TAGS: rust, tooling

# Description

Improve the Markdown parser.
```

```markdown
# Update dashboard

- STATUS: OPEN
- PRIORITY: 25
- TAGS: frontend, dashboard

# Description

Update the dashboard UI.
```

then:

```bash
trask l -t rust
```

returns only:

```text
20260919-160200 [OPEN] [50] Add Rust parser
```

### Filter by Multiple Tags

The `--tag` / `-t` option can be specified multiple times:

```bash
trask l -t rust -t tooling
```

When multiple tags are specified, a task must contain **all** of the requested tags.

For example:

```bash
trask l -t rust -t rtb
```

only returns tasks containing both `rust` and `rtb`.

### Combine Closed Tasks, Filtering, and Sorting

All list options can be combined.

Include closed tasks and sort by priority:

```bash
trask l -c -s p
```

Include closed tasks and reverse the priority sort:

```bash
trask l -c -s p -r
```

Show only Rust tasks and sort by title:

```bash
trask l -t rust -s t
```

Show Rust tasks, including closed tasks, sorted by ID:

```bash
trask l -c -t rust -s i
```

Show Rust and tooling tasks, including closed tasks, sorted by reverse priority:

```bash
trask l -c -t rust -t tooling -s p -r
```

## Show a Task

Display a task using its ID:

```bash
trask show 20260919-154500
```

The `show` command also has the shorthand alias:

```bash
trask s 20260919-154500
```

Example:

```text
ID:       20260919-154500
Title:    Fix ad request timeout
Status:   OPEN
Priority: 100
Tags:     rtb, exchange, timeout

# Description

Investigate why ad requests occasionally exceed the 100ms timeout.
```

## Git

Tasks are ordinary files, so they work naturally with Git.

```bash
git add tasks/
git commit -m "Add ad request timeout task"
```

Task history can be inspected using normal Git tools:

```bash
git log -- tasks/20260919-154500/TASK.md
```

and:

```bash
git blame tasks/20260919-154500/TASK.md
```

Attachments stored alongside a task are also tracked normally by Git.

## Commands

```text
trask init

trask new <title>
trask n <title>
trask add <title>
trask a <title>

trask show <id>
trask s <id>

trask list [OPTIONS]
trask l [OPTIONS]
```

### `init`

Initialize a Trask project:

```bash
trask init
```

Creates the `.trask` marker and `tasks/` directory.

### `new`

Create a new task:

```bash
trask new <title>
```

Aliases:

```bash
trask n <title>
trask add <title>
trask a <title>
```

A timestamp-based task ID is automatically generated.

### `show`

Display a task:

```bash
trask show <id>
trask s <id>
```

### `list` / `l`

List tasks:

```bash
trask list
trask l
```

Options:

```text
-s, --sort <priority|p|id|i|title|t>
-r, --reverse
-t, --tag <tag>
-c, --closed
```

Sort aliases:

```text
priority / p
id / i
title / t
```

Examples:

```bash
trask l
trask l -c

trask l -s p
trask l -s i
trask l -s t

trask l -r
trask l -c -r

trask l -t rust
trask l -t rust -t tooling

trask l -t rust -s t
trask l -t rust -s p -r

trask l -c -t rust -s i
trask l -c -t rust -t tooling -s p -r
```

## Design

The tool intentionally uses a small, predictable Markdown format instead of introducing a database or complex serialization format.

The structured fields are:

```text
title
status
priority
tags
description
```

The task status is represented internally by a `TaskStatus` enum with two values:

```text
Open
Closed
```

These are serialized to Markdown as:

```text
OPEN
CLOSED
```

Everything after the `# Description` heading is treated as the task description and may contain normal Markdown.

The task ID belongs to the task directory rather than the `TASK.md` file:

```text
tasks/<task-id>/TASK.md
```

Task IDs use the UTC timestamp format:

```text
YYYYMMDD-HHMMSS
```

For example:

```text
tasks/20260919-154500/TASK.md
```

This also allows additional files to be stored alongside the task:

```text
tasks/
└── 20260919-154500/
    ├── TASK.md
    ├── screenshot.png
    └── notes.txt
```

The `.trask` marker allows Trask to discover the project root when commands are run from a subdirectory:

```text
project/
├── .trask
├── tasks/
└── src/
    └── ...
```
