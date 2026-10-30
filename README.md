# Trask

A simple, Git-friendly task tracker that stores tasks as Markdown files.

## Installation

Trask is a Rust application and requires a Rust toolchain with Cargo. Check that they are installed:

```bash
rustc --version
cargo --version
```

If you need to install Rust, use [rustup](https://rustup.rs/).

<details>
<summary><strong>Install the CLI from source</strong></summary>

Clone the repository:

```bash
git clone <repository-url>
cd trask
```

Install the CLI:

```bash
cargo install --path . --bin trask
```

Verify the installation:

```bash
trask --help
```

Cargo installs binaries into `$HOME/.cargo/bin` by default. Add that directory to your `PATH` if necessary:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

</details>

<details>
<summary><strong>Build without installing</strong></summary>

Build the debug binaries:

```bash
cargo build
```

The executables are available at:

```text
target/debug/trask
target/debug/trask-lsp
```

Run the CLI directly:

```bash
cargo run -- --help
```

</details>

<details>
<summary><strong>Build and install a release binary</strong></summary>

Build optimized binaries:

```bash
cargo build --release
```

The executables are available at:

```text
target/release/trask
target/release/trask-lsp
```

Install the optimized CLI:

```bash
cargo install --path . --bin trask
```

</details>

## Language Server (LSP)

Trask includes an optional language server, `trask-lsp`, for editor integration. It provides:

- `OPEN` / `CLOSED` status completion and validation.
- Completion suggestions based on tags used by existing tasks.
- Diagnostics for invalid task metadata in `TASK.md` files.
- Document formatting for valid task files.
- Go-to-definition for task IDs referenced from source files or other documents.

The LSP communicates over standard input/output and is started by the editor.

Install it from the Trask repository:

```bash
cargo install --path . --bin trask-lsp
```

Verify that it is on your `PATH`:

```bash
which trask-lsp
```

### Editor setup

<details>
<summary><strong>Helix</strong></summary>

Add the LSP definition to `~/.config/helix/languages.toml`:

```toml
[language-server.trask-lsp]
command = "trask-lsp"
```

Add `TASK.md` to the existing Markdown language's `file-types`, and add `trask-lsp` to its `language-servers` list. For example, keeping the other Markdown servers shown here:

```toml
[[language]]
name = "markdown"
file-types = ["md", { glob = "TASK.md" }]
language-servers = ["marksman", "markdown-oxide", "typos", "trask-lsp"]
```

Keep any other Markdown language servers you already use in that list. This configuration keeps `TASK.md` identified as Markdown, so normal Markdown highlighting remains enabled.

The Trask LSP only publishes task diagnostics and offers task-field completion/formatting for files named `TASK.md`. It can also be added to source-language server lists when you want task-ID go-to-definition from code comments. Append `"trask-lsp"` to the existing `language-servers` arrays for the languages where you want that feature (for example Go, Rust, or Python).

After changing the configuration, restart Helix or run:

```text
:lsp-restart
```

Useful commands:

- `:format` formats a valid `TASK.md` using Trask's canonical Markdown format.
- `gd` jumps to a task file when the cursor is on a Trask task ID and `trask-lsp` is attached to that language.

</details>

<details>
<summary><strong>Neovim 0.11+</strong></summary>

Create `~/.config/nvim/lsp/trask.lua`:

```lua
return {
  cmd = { "trask-lsp" },
  filetypes = { "markdown", "go", "rust", "python" },
  root_dir = function(bufnr, on_dir)
    local root = vim.fs.root(bufnr, { ".trask" })
    if root then
      on_dir(root)
    end
  end,
}
```

Enable the LSP by adding this to `~/.config/nvim/init.lua`:

```lua
vim.lsp.enable("trask")
```

The `.trask` marker is used to locate the project root. Trask task diagnostics, completion, and formatting are limited to files named `TASK.md`. The server can also inspect open Markdown, Go, Rust, and Python documents for task IDs to support go-to-definition.

Format a task file with:

```vim
:lua vim.lsp.buf.format()
```

#### LazyVim: use `gd` for Trask task IDs

When using LazyVim, create `~/.config/nvim/lua/plugins/trask-gd.lua`:

```lua
return {
  {
    "neovim/nvim-lspconfig",
    opts = {
      servers = {
        ["*"] = {
          keys = {
            {
              "gd",
              function()
                Snacks.picker.lsp_definitions()
              end,
              desc = "Goto Definition (includes Trask tasks)",
              has = "definition",
            },
          },
        },
      },
    },
  },
}
```

With `trask-lsp` attached to the source buffer, place the cursor on a task ID in a code comment and press `gd` to open the corresponding `TASK.md`. For other symbols, the Snacks picker continues to show normal LSP definition results.

For example, a comment containing `TODO: 20260919-154500` can link to `tasks/20260919-154500/TASK.md`.

Restart Neovim and its LSP after changing the configuration.

</details>

<details>
<summary><strong>JetBrains (LSP4IJ)</strong></summary>

JetBrains can use `trask-lsp` through [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij), a plugin that lets you configure external Language Server Protocol servers without writing a dedicated JetBrains plugin.

#### Install LSP4IJ

1. Open **Settings → Plugins → Marketplace**.
2. Search for **LSP4IJ** by Red Hat and install it.
3. Restart JetBrains.

#### Register `trask-lsp`

1. Open **Settings → Languages & Frameworks → Language Servers**.
2. Add a **User-defined language server**.
3. In the **Server** tab, configure:

   | Setting | Value                            |
   | ------- | -------------------------------- |
   | Name    | `Trask`                          |
   | Command | The absolute path to `trask-lsp` |

   Find the executable path from a terminal with:

   ```bash
   which trask-lsp
   ```

   Use the returned path in JetBrains. Using an absolute path avoids relying on the environment inherited by the IDE.

4. In the **Mappings** tab, add a file-name pattern mapping:

   | Setting           | Value      |
   | ----------------- | ---------- |
   | File name pattern | `TASK.md`  |
   | Language ID       | `markdown` |

   Use a file-name pattern mapping rather than creating a custom file type. This keeps `TASK.md` associated with Markdown so JetBrains's Markdown highlighting remains available.

5. In the **Workspace Folders** tab, configure marker-based discovery with:

   ```json
   {
     "markers": [".trask"]
   }
   ```

   LSP4IJ walks up from the opened file and uses the directory containing `.trask` as the workspace folder. This lets Trask find the project's `tasks/` directory and offer existing tags.

6. Save the server configuration. Open a `TASK.md` file inside an initialized Trask project.

#### Optional: task-ID go-to-definition from source files

The Trask LSP can also resolve task IDs in source files. In the **Mappings** tab, add mappings for the source languages you use, with their corresponding language IDs:

| JetBrains language / file type | Language ID |
| ------------------------------ | ----------- |
| Go                             | `go`        |
| Rust                           | `rust`      |
| Python                         | `python`    |

Only add mappings for languages supported by your JetBrains installation. These mappings allow the LSP to inspect those open documents for task IDs; task-specific diagnostics and field completion remain limited to files named `TASK.md`.

#### Formatting

For a valid `TASK.md`, use JetBrains' **Code → Reformat Code** or **Reformat File** action. LSP4IJ routes the formatting request to Trask, which normalizes the task metadata and description heading.

</details>

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

## Project layout

Tasks live in a `tasks/` directory at the root of your project. Trask uses a `.trask` marker file to identify the project root, so commands can be run from the project root or from a subdirectory.

Each task gets its own directory named with a unique task ID. Every task directory contains a `TASK.md` file and can contain attachments such as screenshots or other files.

Task IDs use the UTC timestamp format `YYYYMMDD-HHMMSS`:

```text
project/
├── .trask
├── tasks/
│   ├── 20260919-154500/
│   │   ├── TASK.md
│   │   └── screenshot.png
│   └── 20260919-160200/
│       └── TASK.md
└── ...
```

## Initialize a project

Run `init` from the root of your project:

```bash
trask init
```

This creates the `.trask` marker and `tasks/` directory. Running `init` on an already initialized project returns an error.

## Create a task

Create a task with:

```bash
trask new "Fix ad request timeout"
```

Aliases:

```bash
trask n "Fix ad request timeout"
trask add "Fix ad request timeout"
trask a "Fix ad request timeout"
```

Trask generates a timestamp-based task ID and copies the new ID to the system clipboard. The task is created with status `OPEN`, no tags, an empty description, and the special unprioritized priority value (`4294967295`).

The generated `TASK.md` looks like:

```markdown
# Fix ad request timeout

- STATUS: OPEN
- PRIORITY: 4294967295
- TAGS:

# Description
```

Edit `TASK.md` directly to add tags, set a priority, and write the description.

## Task format and parsing

A canonical task file looks like:

```markdown
# Fix ad request timeout

- STATUS: OPEN
- PRIORITY: 1
- TAGS: rtb, exchange, timeout

# Description

Investigate why ad requests occasionally exceed the 100ms timeout.
```

The fields are:

| Field           | Meaning                                              | Required |
| --------------- | ---------------------------------------------------- | -------- |
| Title           | The first Markdown heading (`# <title>`)             | Yes      |
| `STATUS`        | `OPEN` or `CLOSED`                                   | Yes      |
| `PRIORITY`      | An unsigned integer                                  | Yes      |
| `TAGS`          | Comma-separated tags                                 | No       |
| `# Description` | Optional section heading before the description body | No       |

### Flexible parsing

Trask accepts flexible spacing around metadata keys and colons, and metadata keys/status values are case-insensitive. `STATUS` and `PRIORITY` are required; tags and description content can be omitted.

The following are all valid:

```markdown
# Fix timeout

- STATUS : opened
- PRIORITY: 1
- TAGS : foo, bar ,

Investigate the timeout.
```

```markdown
# Fix timeout

- PRIORITY: 1
- STATUS: o
```

The description heading is optional. Without it, the text following the metadata is treated as the description. The description body itself can also be empty.

Tag values are split on commas, trimmed individually, and empty values are ignored. For example:

```text
foo,     bar   ,
```

is parsed as two tags: `foo` and `bar`. Whitespace inside a tag is preserved, so `ad exchange` remains one tag.

### Status values

The parser accepts these status spellings, regardless of capitalization:

| Accepted values        | Stored status        | Canonical output |
| ---------------------- | -------------------- | ---------------- |
| `open`, `opened`, `o`  | `TaskStatus::Open`   | `OPEN`           |
| `closed`, `close`, `c` | `TaskStatus::Closed` | `CLOSED`         |

A new task starts as `OPEN`. To close a task, change its `STATUS` field to `CLOSED` (or use an accepted alias and format the file).

### Formatting

The LSP provides document formatting for valid `TASK.md` files. Formatting normalizes the file to Trask's canonical layout:

- Standardizes metadata field names and spacing.
- Converts accepted status aliases to `OPEN` or `CLOSED`.
- Normalizes comma-separated tags to `foo, bar` and removes empty tag entries.
- Adds the `# Description` heading, even when the original file omitted it.
- Preserves the description body as Markdown.

Formatting does not invent missing required values. If the title, status, or priority is invalid or missing, the formatter leaves the document unchanged.

## Priority

Priority uses an unsigned integer. **Lower numbers mean higher priority**, like `P1`, `P2`, and `P3`:

| Priority     | Meaning                                | Sort order                            |
| ------------ | -------------------------------------- | ------------------------------------- |
| `1`          | Highest assigned priority              | First among numbered priorities       |
| `2`          | Next priority                          | After `1`                             |
| `3`          | Next priority                          | After `2`                             |
| `10`         | Lower priority                         | After `3`                             |
| `4294967295` | Unprioritized (reserved special value) | Always at the top in priority sorting |

New tasks use `4294967295` to indicate that they have not yet been assigned a priority. This special value sorts ahead of numbered priorities so new tasks are easy to find. Once you assign a number such as `1`, the task moves into the normal priority order.

Priority sorting is the default. `--reverse` reverses the order of numbered priorities while keeping unprioritized tasks at the top.

## List tasks

List open tasks:

```bash
trask list
trask l
```

By default, closed tasks are excluded and tasks are sorted by priority: unprioritized tasks first, followed by numbered priorities from lowest number to highest number.

Example:

```text
20260919-160200 [OPEN] [4294967295] Add Helix integration
20260919-154500 [OPEN] [1] Fix ad request timeout
20260919-161030 [OPEN] [2] Add task references
```

### Include closed tasks

Use `--closed` or `-c` to include closed tasks in the results. It does not show only closed tasks:

```bash
trask list --closed
trask l -c
```

### Sort tasks

Use `--sort` or `-s`:

| Sort     | Long value | Alias |
| -------- | ---------- | ----- |
| Priority | `priority` | `p`   |
| ID       | `id`       | `i`   |
| Title    | `title`    | `t`   |

Examples:

```bash
trask l -s p
trask l --sort priority
trask l -s i
trask l -s t
```

Priority sorts numbered tasks in ascending numeric order, with unprioritized tasks first. ID sorts by timestamp-based ID. Title sorts alphabetically.

### Reverse the sort

Use `--reverse` or `-r`:

```bash
trask l -r
trask l -s p -r
trask l -s i -r
trask l -s t -r
```

When sorting by priority, `-r` reverses the numbered priorities but leaves unprioritized tasks at the top.

### Filter by tag

Use `--tag` or `-t`:

```bash
trask list --tag bug
trask l -t bug
```

Tag matching is case-insensitive, so `bug`, `Bug`, and `BUG` match the same tag.

You can specify multiple tags. A task must contain **all** requested tags (AND matching):

```bash
trask l -t bug -t rtb
```

### Combine filters and sorting

List options can be combined:

```bash
# Include closed tasks, filter by tag, and sort by priority.
trask l -c -t bug -s p

# Reverse the numbered priority order.
trask l -t rust -s p -r

# Match both tags and sort by title.
trask l -t rust -t tooling -s t
```

## Show a task

Display a task using its ID:

```bash
trask show 20260919-154500
trask s 20260919-154500
```

Example:

```text
ID:       20260919-154500
Title:    Fix ad request timeout
Status:   OPEN
Priority: 1
Tags:     rtb, exchange, timeout

Investigate why ad requests occasionally exceed the 100ms timeout.
```

The description is printed as entered; headings such as `# Background` remain part of the description. The structural `# Description` marker is not printed as an extra heading.

## Delete a task

Delete a task using its ID:

```bash
trask delete 20260919-154500
```

Aliases:

```bash
trask d 20260919-154500
trask remove 20260919-154500
trask r 20260919-154500
```

Delete removes the task's entire directory, including `TASK.md` and any attachments stored alongside it.

## Gist

Show the number of tasks in each status:

```bash
trask gist
trask g
```

Example shape:

```text
Gist:
OPEN: <count>
CLOSED: <count>
```

## Task IDs in source code

Task IDs use the format `YYYYMMDD-HHMMSS`. When `trask-lsp` is attached to a source file, placing the cursor on a task ID and invoking go-to-definition opens the matching task file:

```text
TODO: 20260919-154500
```

The target is:

```text
tasks/20260919-154500/TASK.md
```

The LSP supports this navigation from documents it has open. For Helix, add `trask-lsp` to the relevant source language's `language-servers` list. The Neovim configuration above attaches it to Markdown, Go, Rust, and Python buffers in a Trask project.

## Commands

| Command             | Aliases            | Description                                    |
| ------------------- | ------------------ | ---------------------------------------------- |
| `trask init`        | —                  | Initialize a Trask project                     |
| `trask new <title>` | `n`, `add`, `a`    | Create a task and copy its ID to the clipboard |
| `trask show <id>`   | `s`                | Show a task                                    |
| `trask list`        | `l`                | List tasks with optional filters and sorting   |
| `trask delete <id>` | `d`, `remove`, `r` | Delete a task directory                        |
| `trask gist`        | `g`                | Summarize task counts by status                |

List options:

| Option          | Accepted values                             | Description                                                                            |
| --------------- | ------------------------------------------- | -------------------------------------------------------------------------------------- |
| `-s, --sort`    | `priority` / `p`, `id` / `i`, `title` / `t` | Sort by priority, ID, or title                                                         |
| `-r, --reverse` | —                                           | Reverse the selected sort (unprioritized tasks remain at the top for priority sorting) |
| `-t, --tag`     | A tag value                                 | Filter by tag; repeat for AND matching                                                 |
| `-c, --closed`  | —                                           | Include closed tasks                                                                   |

## Git

Tasks are ordinary files, so they work naturally with Git:

```bash
git add tasks/
git commit -m "Add ad request timeout task"
```

Inspect task history with normal Git tools:

```bash
git log -- tasks/20260919-154500/TASK.md
git blame tasks/20260919-154500/TASK.md
```

Attachments stored alongside a task are also tracked normally by Git.

## Design

Trask intentionally uses a small, predictable Markdown format instead of introducing a database or complex serialization format. Task IDs belong to their directories rather than the `TASK.md` files:

```text
tasks/<task-id>/TASK.md
```

The `.trask` marker allows Trask to discover the project root when commands are run from a subdirectory:

```text
project/
├── .trask
├── tasks/
└── src/
    └── ...
```
