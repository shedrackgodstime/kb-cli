mod commands;

use clap::{CommandFactory, Parser, Subcommand};
use colored::Colorize;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "kb",
    about = "Cross-platform CLI for managing the knowledge-base ecosystem",
    version,
    after_help = "Run `kb <command> --help` for more information on a command."
)]
struct Cli {
    /// Override knowledge-base root discovery
    #[arg(long, global = true)]
    kb_root: Option<PathBuf>,

    /// Output machine-readable JSON
    #[arg(long, global = true)]
    json: bool,

    /// Verbose output
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Quiet output — suppress informational messages
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// One-time setup on a new machine
    Init {
        /// Path to knowledge-base repo (auto-detected if omitted)
        #[arg(long)]
        kb_root: Option<PathBuf>,
    },

    /// Wire a project repository to the knowledge-base
    Link {
        /// Project name or path to project repo
        project: String,
    },

    /// Remove symlinks for a project (keeps memory)
    Unlink {
        /// Project name or path
        project: String,

        /// Do not touch ~/.gitignore
        #[arg(long)]
        keep_gitignore: bool,
    },

    /// Move a project's memory to archive/ (or restore it with --restore)
    Archive {
        /// Project name
        project: String,

        /// Restore an archived project's memory back to projects/
        #[arg(long)]
        restore: bool,
    },

    /// Open a project's KB directory (or the knowledge-base root) in your
    /// editor or file manager
    Open {
        /// Project name (omit to open the knowledge-base root)
        project: Option<String>,
    },

    /// Overview of the knowledge-base on this machine
    Status {
        /// Show all projects, not just active ones
        #[arg(long)]
        all: bool,

        /// Include ref repo details
        #[arg(long)]
        refs: bool,
    },

    /// Health check across the knowledge-base
    Doctor {
        /// Attempt to auto-fix issues where safe
        #[arg(long)]
        fix: bool,
    },

    /// List all projects in the knowledge-base
    Projects {
        /// Only show active projects
        #[arg(long)]
        active_only: bool,

        /// Include ref counts, handoff dates, disk usage
        #[arg(long)]
        verbose: bool,
    },

    /// Clone missing reference repos from ref/README.md
    CloneRefs {
        /// Project name (omit for all active projects)
        project: Option<String>,

        /// Process all active projects
        #[arg(long)]
        all: bool,

        /// Shallow clone (depth=1)
        #[arg(long)]
        shallow: bool,

        /// Full clone (no depth limit)
        #[arg(long)]
        full: bool,

        /// Show what would be cloned without doing it
        #[arg(long)]
        dry_run: bool,

        /// Re-clone even if directory exists (fixes revision mismatches)
        #[arg(long)]
        force: bool,
    },

    /// Sync with remote: git pull --rebase + re-link
    Sync {
        /// Only re-link this specific project (repeatable)
        #[arg(long = "project")]
        projects: Vec<String>,
    },

    /// Bidirectional sync of the whole KB: pull if safe, then push everything
    GlobalSync {
        /// Custom commit message
        #[arg(long, short)]
        message: Option<String>,

        /// Re-link active projects after syncing (default: on)
        #[arg(long)]
        no_link: bool,

        /// Preview what would happen without changing anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Pull knowledge-base updates (fast-forward only)
    Pull {
        /// Only pull and re-link this specific project (repeatable)
        #[arg(long = "project")]
        projects: Vec<String>,

        /// Re-link active projects after pulling
        #[arg(long)]
        link: bool,

        /// Only pull, don't re-link
        #[arg(long)]
        no_link: bool,
    },

    /// Push changes for specific projects to remote
    Push {
        /// Project name to push (repeatable)
        #[arg(long = "project")]
        projects: Vec<String>,

        /// Custom commit message
        #[arg(long, short)]
        message: Option<String>,
    },

    /// Export project memory to a portable tarball
    Export {
        /// Project name to export
        project: String,

        /// Output path (default: ~/project-name.tar.gz)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Import project memory from a tarball
    Import {
        /// Path to tarball
        tarball: PathBuf,

        /// Project name (auto-detected from tarball if omitted)
        #[arg(long)]
        name: Option<String>,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Generate man page
    Man {
        /// Output directory (default: current directory)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Start working on a project (sync + link + track)
    Work {
        /// Project name
        project: String,
    },

    /// Finish working: commit + push tracked projects
    Done {
        /// Custom commit message
        #[arg(long, short)]
        message: Option<String>,
    },

    /// Search memory files across the knowledge-base
    Search {
        /// Pattern to search for (case-insensitive substring match)
        query: String,

        /// Only search within these projects (repeatable)
        #[arg(long = "project")]
        projects: Vec<String>,

        /// Only show matching file paths, not individual lines
        #[arg(long)]
        files_only: bool,

        /// Only match files that match this glob pattern
        #[arg(long)]
        glob: Option<String>,

        /// Match case-sensitively (default: insensitive)
        #[arg(long)]
        case_sensitive: bool,
    },

    /// Show recent knowledge-base history from git
    Log {
        /// Number of commits to show
        #[arg(short = 'n', long, default_value_t = 15)]
        limit: usize,

        /// Show per-file change statistics
        #[arg(long)]
        stat: bool,

        /// Only show commits touching these projects (repeatable)
        #[arg(long = "project")]
        projects: Vec<String>,
    },

    /// Ensure the personal kb-rules.md map for a project
    Rules {
        /// Project name or path (default: current repository)
        project: Option<String>,

        /// Refresh every configured project
        #[arg(long)]
        all: bool,

        /// Show what would happen without writing anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Subscribe this device to a project's memory (selective sync)
    Subscribe {
        /// Project name
        project: String,

        /// Preview what would change without writing anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Stop receiving a project's memory on this device
    Unsubscribe {
        /// Project name
        project: String,

        /// Preview what would change without writing anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Inspect and edit machine-local configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// List all configuration values
    List,

    /// Get a single configuration value
    Get {
        /// Dotted config key, e.g. kb_root or projects.irosh.clone_depth
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Dotted config key
        key: String,

        /// Value (JSON array, or plain value; accepts ~ for paths)
        value: String,
    },

    /// Reset a configuration key to its default
    Unset {
        /// Dotted config key
        key: String,
    },
}

#[derive(clap::ValueEnum, Clone)]
#[allow(clippy::enum_variant_names)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell", alias = "power-shell")]
    PowerShell,
    Elvish,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { kb_root } => commands::init::run(kb_root.as_deref(), cli.json, cli.quiet),
        Commands::Link { project } => {
            commands::link::run(cli.kb_root.as_deref(), &project, cli.json, cli.quiet)
        }
        Commands::Unlink {
            project,
            keep_gitignore,
        } => commands::unlink::run(
            cli.kb_root.as_deref(),
            &project,
            keep_gitignore,
            cli.json,
            cli.quiet,
        ),
        Commands::Archive { project, restore } => commands::archive::run(
            cli.kb_root.as_deref(),
            &project,
            restore,
            cli.json,
            cli.quiet,
        ),
        Commands::Status { all, refs } => {
            commands::status::run(cli.kb_root.as_deref(), all, refs, cli.json, cli.quiet)
        }
        Commands::Doctor { fix } => {
            commands::doctor::run(cli.kb_root.as_deref(), fix, cli.json, cli.quiet)
        }
        Commands::Projects {
            active_only,
            verbose,
        } => commands::projects::run(
            cli.kb_root.as_deref(),
            active_only,
            verbose,
            cli.json,
            cli.quiet,
        ),
        Commands::CloneRefs {
            project,
            all,
            shallow,
            full,
            dry_run,
            force,
        } => commands::clone_refs::run(
            cli.kb_root.as_deref(),
            project.as_deref(),
            all,
            shallow,
            full,
            dry_run,
            force,
            cli.json,
            cli.quiet,
        ),
        Commands::Sync { projects } => {
            commands::sync::run(cli.kb_root.as_deref(), &projects, cli.json, cli.quiet)
        }
        Commands::GlobalSync {
            message,
            no_link,
            dry_run,
        } => commands::global_sync::run(
            cli.kb_root.as_deref(),
            message.as_deref(),
            !no_link,
            dry_run,
            cli.json,
            cli.quiet,
        ),
        Commands::Pull {
            projects,
            link,
            no_link,
        } => commands::pull::run(
            cli.kb_root.as_deref(),
            &projects,
            link,
            no_link,
            cli.json,
            cli.quiet,
        ),
        Commands::Push { projects, message } => commands::push::run(
            cli.kb_root.as_deref(),
            &projects,
            message.as_deref(),
            cli.json,
            cli.quiet,
        ),
        Commands::Export { project, output } => commands::export::run(
            cli.kb_root.as_deref(),
            &project,
            output.as_deref(),
            cli.json,
            cli.quiet,
        ),
        Commands::Import { tarball, name } => commands::import::run(
            cli.kb_root.as_deref(),
            &tarball,
            name.as_deref(),
            cli.json,
            cli.quiet,
        ),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let shell_name = match shell {
                Shell::Bash => "bash",
                Shell::Zsh => "zsh",
                Shell::Fish => "fish",
                Shell::PowerShell => "powershell",
                Shell::Elvish => "elvish",
            };

            let ext = match shell {
                Shell::Bash => "bash",
                Shell::Zsh => "zsh",
                Shell::Fish => "fish",
                Shell::PowerShell => "ps1",
                Shell::Elvish => "elvish",
            };

            let filename = format!("kb.{}", ext);

            if cli.json {
                let mut buf = Vec::new();
                clap_complete::generate(
                    match shell {
                        Shell::Bash => clap_complete::Shell::Bash,
                        Shell::Zsh => clap_complete::Shell::Zsh,
                        Shell::Fish => clap_complete::Shell::Fish,
                        Shell::PowerShell => clap_complete::Shell::PowerShell,
                        Shell::Elvish => clap_complete::Shell::Elvish,
                    },
                    &mut cmd,
                    "kb",
                    &mut buf,
                );
                let output = serde_json::json!({
                    "ok": true,
                    "data": {
                        "shell": shell_name,
                        "file": filename,
                        "completions": String::from_utf8_lossy(&buf),
                    }
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                let output_dir = std::env::current_dir()?;
                let path = output_dir.join(&filename);
                let mut file = std::fs::File::create(&path)?;
                clap_complete::generate(
                    match shell {
                        Shell::Bash => clap_complete::Shell::Bash,
                        Shell::Zsh => clap_complete::Shell::Zsh,
                        Shell::Fish => clap_complete::Shell::Fish,
                        Shell::PowerShell => clap_complete::Shell::PowerShell,
                        Shell::Elvish => clap_complete::Shell::Elvish,
                    },
                    &mut cmd,
                    "kb",
                    &mut file,
                );
                println!();
                println!("  {} {}", "Generated".green().bold(), shell_name.bold());
                println!("  {}", path.display());
                println!();
                match shell {
                    Shell::Bash => {
                        println!("  To install:");
                        println!(
                            "    cp {} ~/.local/share/bash-completion/completions/",
                            path.display()
                        );
                    }
                    Shell::Zsh => {
                        println!("  To install:");
                        println!("    cp {} ~/.zsh/completions/_kb", path.display());
                    }
                    Shell::Fish => {
                        println!("  To install:");
                        println!(
                            "    cp {} ~/.config/fish/completions/kb.fish",
                            path.display()
                        );
                    }
                    Shell::PowerShell => {
                        println!("  To install:");
                        println!(
                            "    cp {} $env:USERPROFILE\\Documents\\WindowsPowerShell\\Modules\\",
                            path.display()
                        );
                    }
                    Shell::Elvish => {
                        println!("  To install:");
                        println!("    cp {} ~/.config/elvish/lib/", path.display());
                    }
                }
                println!();
            }

            Ok(())
        }
        Commands::Man { output } => {
            let cmd = Cli::command();
            let output_dir = output.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

            std::fs::create_dir_all(&output_dir)?;

            let man = clap_mangen::Man::new(cmd);
            let mut buffer = Vec::new();
            man.render(&mut buffer)?;

            let path = output_dir.join("kb.1");
            let mut file = std::fs::File::create(&path)?;
            file.write_all(&buffer)?;

            if cli.json {
                let output = serde_json::json!({
                    "ok": true,
                    "data": {
                        "file": path,
                    }
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!();
                println!("  {} {}", "Generated".green().bold(), "man page".bold());
                println!("  {}", path.display());
                println!();
                println!("  To view: man ./kb.1");
                println!(
                    "  To install: cp {} /usr/local/share/man/man1/",
                    path.display()
                );
                println!();
            }

            Ok(())
        }
        Commands::Work { project } => {
            commands::work::run(cli.kb_root.as_deref(), &project, cli.json, cli.quiet)
        }
        Commands::Search {
            query,
            projects,
            files_only,
            glob,
            case_sensitive,
        } => {
            let found = commands::search::run(
                cli.kb_root.as_deref(),
                &query,
                &projects,
                glob.as_deref(),
                files_only,
                case_sensitive,
                cli.json,
                cli.quiet,
            )?;
            if !found {
                std::process::exit(1);
            }
            Ok(())
        }
        Commands::Done { message } => commands::done::run(
            cli.kb_root.as_deref(),
            message.as_deref(),
            cli.json,
            cli.quiet,
        ),
        Commands::Log {
            limit,
            stat,
            projects,
        } => commands::log::run(
            cli.kb_root.as_deref(),
            limit,
            stat,
            &projects,
            cli.json,
            cli.quiet,
        ),
        Commands::Subscribe { project, dry_run } => commands::subscriptions::subscribe(
            cli.kb_root.as_deref(),
            &project,
            dry_run,
            cli.json,
            cli.quiet,
        ),
        Commands::Unsubscribe { project, dry_run } => commands::subscriptions::unsubscribe(
            cli.kb_root.as_deref(),
            &project,
            dry_run,
            cli.json,
            cli.quiet,
        ),
        Commands::Open { project } => commands::open::run(
            cli.kb_root.as_deref(),
            project.as_deref(),
            cli.json,
            cli.quiet,
        ),
        Commands::Rules {
            project,
            all,
            dry_run,
        } => commands::rules::run(
            cli.kb_root.as_deref(),
            project.as_deref(),
            all,
            dry_run,
            cli.json,
            cli.quiet,
        ),
        Commands::Config { command } => match command {
            ConfigCommand::List => commands::config::run_list(cli.json, cli.quiet),
            ConfigCommand::Get { key } => commands::config::run_get(cli.json, &key, cli.quiet),
            ConfigCommand::Set { key, value } => {
                commands::config::run_set(cli.json, &key, &value, cli.quiet)
            }
            ConfigCommand::Unset { key } => commands::config::run_unset(cli.json, &key, cli.quiet),
        },
    }
}
