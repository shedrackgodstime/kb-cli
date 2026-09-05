# kb

Cross-platform CLI for managing a private knowledge-base ecosystem.

`kb` wires your project repos to a central knowledge-base via symlinks, so every project automatically has research, specs, handoffs, and agent rules — and you can sync it all across machines.

## Install

**Unix / macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/shedrackgodstime/kb-cli/master/install.sh | bash
```

**Windows (PowerShell):**

```powershell
iwr -useb https://raw.githubusercontent.com/shedrackgodstime/kb-cli/master/install.ps1 | iex
```

**From source (requires Rust):**

```bash
git clone git@github.com:shedrackgodstime/kb-cli.git
cd kb-cli
cargo install --path .
```

## Quick Start

```bash
# 1. Setup on a new machine
kb init

# 2. Wire a project
cd ~/Projects/my-app
kb link .

# 3. Check health
kb doctor

# 4. Start working
kb work my-app

# 5. Finish up
kb done
```

## Commands

| Command | Description |
|---|---|
| `kb init` | Setup on a new machine |
| `kb link <project>` | Wire a project to the knowledge-base |
| `kb unlink <project>` | Remove symlinks (keeps memory) |
| `kb status` | Overview of wired projects |
| `kb doctor` | Health check with warnings |
| `kb projects` | List all projects |
| `kb work <project>` | Start working (sync + link + track) |
| `kb done` | Finish working (commit + push) |
| `kb sync` | Sync with remote (pull --rebase) |
| `kb pull` | Pull updates (fast-forward only) |
| `kb push --project X` | Push changes for specific projects |
| `kb clone-refs` | Clone reference repos |
| `kb export` | Export project memory to tarball |
| `kb import` | Import project memory from tarball |
| `kb completions` | Generate shell completions |
| `kb man` | Generate man page |

All commands support `--json` for machine-readable output.

## Multi-Machine Workflow

```bash
# Machine A: start work
kb work dioxus-auth
# ... make changes ...
kb done

# Machine B: sync
kb work dioxus-auth
# ... make changes ...
kb done
```

`kb work` does `git pull --rebase` automatically. `kb done` only commits files for the projects you're working on.

## Configuration

Config at `~/.kb/config.toml`:

```toml
kb_root = "/home/user/knowledge-base"
active_projects = ["dioxus-auth", "kb"]
```

## Global Flags

- `--json` — machine-readable JSON output
- `--verbose` / `--quiet` — verbosity
- `--kb-root` — override KB root discovery

## Windows Notes

Windows requires **Developer Mode** or **Administrator privileges** to create symlinks.

To enable Developer Mode:
1. Open Settings → System → For developers
2. Toggle "Developer Mode" on

Without this, `kb link` will fail with a privilege error.

## Shell Completions

```bash
kb completions bash > ~/.local/share/bash-completion/completions/kb
kb completions zsh > ~/.zsh/completions/_kb
kb completions fish > ~/.config/fish/completions/kb.fish
```

## License

MIT
