# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_kb_global_optspecs
    string join \n kb-root= json v/verbose q/quiet h/help V/version
end

function __fish_kb_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_kb_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_kb_using_subcommand
    set -l cmd (__fish_kb_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c kb -n "__fish_kb_needs_command" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_needs_command" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_needs_command" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_needs_command" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_needs_command" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_needs_command" -s V -l version -d 'Print version'
complete -c kb -n "__fish_kb_needs_command" -f -a "init" -d 'One-time setup on a new machine'
complete -c kb -n "__fish_kb_needs_command" -f -a "link" -d 'Wire a project repository to the knowledge-base'
complete -c kb -n "__fish_kb_needs_command" -f -a "unlink" -d 'Remove symlinks for a project (keeps memory)'
complete -c kb -n "__fish_kb_needs_command" -f -a "archive" -d 'Move a project\'s memory to archive/ (or restore it with --restore)'
complete -c kb -n "__fish_kb_needs_command" -f -a "open" -d 'Open a project\'s KB directory (or the knowledge-base root) in your editor or file manager'
complete -c kb -n "__fish_kb_needs_command" -f -a "status" -d 'Overview of the knowledge-base on this machine'
complete -c kb -n "__fish_kb_needs_command" -f -a "doctor" -d 'Health check across the knowledge-base'
complete -c kb -n "__fish_kb_needs_command" -f -a "projects" -d 'List all projects in the knowledge-base'
complete -c kb -n "__fish_kb_needs_command" -f -a "clone-refs" -d 'Clone missing reference repos from ref/README.md'
complete -c kb -n "__fish_kb_needs_command" -f -a "sync" -d 'Sync with remote: git pull --rebase + re-link'
complete -c kb -n "__fish_kb_needs_command" -f -a "global-sync" -d 'Bidirectional sync of the whole KB: pull if safe, then push everything'
complete -c kb -n "__fish_kb_needs_command" -f -a "pull" -d 'Pull knowledge-base updates (fast-forward only)'
complete -c kb -n "__fish_kb_needs_command" -f -a "push" -d 'Push changes for specific projects to remote'
complete -c kb -n "__fish_kb_needs_command" -f -a "export" -d 'Export project memory to a portable tarball'
complete -c kb -n "__fish_kb_needs_command" -f -a "import" -d 'Import project memory from a tarball'
complete -c kb -n "__fish_kb_needs_command" -f -a "completions" -d 'Generate shell completions'
complete -c kb -n "__fish_kb_needs_command" -f -a "man" -d 'Generate man page'
complete -c kb -n "__fish_kb_needs_command" -f -a "work" -d 'Start working on a project (sync + link + track)'
complete -c kb -n "__fish_kb_needs_command" -f -a "done" -d 'Finish working: commit + push tracked projects'
complete -c kb -n "__fish_kb_needs_command" -f -a "search" -d 'Search memory files across the knowledge-base'
complete -c kb -n "__fish_kb_needs_command" -f -a "log" -d 'Show recent knowledge-base history from git'
complete -c kb -n "__fish_kb_needs_command" -f -a "rules" -d 'Ensure the personal kb-rules.md map for a project'
complete -c kb -n "__fish_kb_needs_command" -f -a "subscribe" -d 'Subscribe this device to a project\'s memory (selective sync)'
complete -c kb -n "__fish_kb_needs_command" -f -a "unsubscribe" -d 'Stop receiving a project\'s memory on this device'
complete -c kb -n "__fish_kb_needs_command" -f -a "config" -d 'Inspect and edit machine-local configuration'
complete -c kb -n "__fish_kb_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c kb -n "__fish_kb_using_subcommand init" -l kb-root -d 'Path to knowledge-base repo (auto-detected if omitted)' -r -F
complete -c kb -n "__fish_kb_using_subcommand init" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand init" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand init" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand init" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand link" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand link" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand link" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand link" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand link" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand unlink" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand unlink" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand unlink" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand unlink" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand unlink" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand archive" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand archive" -l restore -d 'Restore an archived project\'s memory back to projects/'
complete -c kb -n "__fish_kb_using_subcommand archive" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand archive" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand archive" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand archive" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand open" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand open" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand open" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand open" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand open" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand status" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand status" -l all -d 'Show all projects, not just active ones'
complete -c kb -n "__fish_kb_using_subcommand status" -l refs -d 'Include ref repo details'
complete -c kb -n "__fish_kb_using_subcommand status" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand status" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand status" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand status" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand doctor" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand doctor" -l fix -d 'Attempt to auto-fix issues where safe'
complete -c kb -n "__fish_kb_using_subcommand doctor" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand doctor" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand doctor" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand projects" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand projects" -l active-only -d 'Only show active projects'
complete -c kb -n "__fish_kb_using_subcommand projects" -l verbose -d 'Include ref counts, handoff dates, disk usage'
complete -c kb -n "__fish_kb_using_subcommand projects" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand projects" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand projects" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -l all -d 'Process all active projects'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -l shallow -d 'Shallow clone (depth=1)'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -l dry-run -d 'Show what would be cloned without doing it'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -l force -d 'Re-clone even if directory exists (fixes revision mismatches)'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand clone-refs" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand sync" -l project -d 'Only re-link this specific project (repeatable)' -r
complete -c kb -n "__fish_kb_using_subcommand sync" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand sync" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand sync" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand sync" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand sync" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand global-sync" -s m -l message -d 'Custom commit message' -r
complete -c kb -n "__fish_kb_using_subcommand global-sync" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand global-sync" -l no-link -d 'Re-link active projects after syncing (default: on)'
complete -c kb -n "__fish_kb_using_subcommand global-sync" -l dry-run -d 'Preview what would happen without changing anything'
complete -c kb -n "__fish_kb_using_subcommand global-sync" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand global-sync" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand global-sync" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand global-sync" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand pull" -l project -d 'Only pull and re-link this specific project (repeatable)' -r
complete -c kb -n "__fish_kb_using_subcommand pull" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand pull" -l link -d 'Re-link active projects after pulling'
complete -c kb -n "__fish_kb_using_subcommand pull" -l no-link -d 'Only pull, don\'t re-link'
complete -c kb -n "__fish_kb_using_subcommand pull" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand pull" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand pull" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand pull" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand push" -l project -d 'Project name to push (repeatable)' -r
complete -c kb -n "__fish_kb_using_subcommand push" -s m -l message -d 'Custom commit message' -r
complete -c kb -n "__fish_kb_using_subcommand push" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand push" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand push" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand push" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand push" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand export" -l output -d 'Output path (default: ~/project-name.tar.gz)' -r -F
complete -c kb -n "__fish_kb_using_subcommand export" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand export" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand export" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand export" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand export" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand import" -l name -d 'Project name (auto-detected from tarball if omitted)' -r
complete -c kb -n "__fish_kb_using_subcommand import" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand import" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand import" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand import" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand import" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand completions" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand completions" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand completions" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand completions" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand completions" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand man" -l output -d 'Output directory (default: current directory)' -r -F
complete -c kb -n "__fish_kb_using_subcommand man" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand man" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand man" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand man" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand man" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand work" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand work" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand work" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand work" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand work" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand done" -s m -l message -d 'Custom commit message' -r
complete -c kb -n "__fish_kb_using_subcommand done" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand done" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand done" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand done" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand done" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand search" -l project -d 'Only search within these projects (repeatable)' -r
complete -c kb -n "__fish_kb_using_subcommand search" -l glob -d 'Only match files that match this glob pattern' -r
complete -c kb -n "__fish_kb_using_subcommand search" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand search" -l files-only -d 'Only show matching file paths, not individual lines'
complete -c kb -n "__fish_kb_using_subcommand search" -l case-sensitive -d 'Match case-sensitively (default: insensitive)'
complete -c kb -n "__fish_kb_using_subcommand search" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand search" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand search" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand search" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand log" -s n -l limit -d 'Number of commits to show' -r
complete -c kb -n "__fish_kb_using_subcommand log" -l project -d 'Only show commits touching these projects (repeatable)' -r
complete -c kb -n "__fish_kb_using_subcommand log" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand log" -l stat -d 'Show per-file change statistics'
complete -c kb -n "__fish_kb_using_subcommand log" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand log" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand log" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand log" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand rules" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand rules" -l all -d 'Refresh every configured project'
complete -c kb -n "__fish_kb_using_subcommand rules" -l dry-run -d 'Show what would happen without writing anything'
complete -c kb -n "__fish_kb_using_subcommand rules" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand rules" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand rules" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand rules" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand subscribe" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand subscribe" -l dry-run -d 'Preview what would change without writing anything'
complete -c kb -n "__fish_kb_using_subcommand subscribe" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand subscribe" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand subscribe" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand subscribe" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand unsubscribe" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand unsubscribe" -l dry-run -d 'Preview what would change without writing anything'
complete -c kb -n "__fish_kb_using_subcommand unsubscribe" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand unsubscribe" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand unsubscribe" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand unsubscribe" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -f -a "list" -d 'List all configuration values'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -f -a "get" -d 'Get a single configuration value'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -f -a "set" -d 'Set a configuration value'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -f -a "unset" -d 'Reset a configuration key to its default'
complete -c kb -n "__fish_kb_using_subcommand config; and not __fish_seen_subcommand_from list get set unset help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from list" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from list" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from list" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from list" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from get" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from get" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from get" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from get" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from get" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from set" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from set" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from set" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from set" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from unset" -l kb-root -d 'Override knowledge-base root discovery' -r -F
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from unset" -l json -d 'Output machine-readable JSON'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from unset" -s v -l verbose -d 'Verbose output'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from unset" -s q -l quiet -d 'Quiet output — suppress informational messages'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from unset" -s h -l help -d 'Print help'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "list" -d 'List all configuration values'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "get" -d 'Get a single configuration value'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "set" -d 'Set a configuration value'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "unset" -d 'Reset a configuration key to its default'
complete -c kb -n "__fish_kb_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "init" -d 'One-time setup on a new machine'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "link" -d 'Wire a project repository to the knowledge-base'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "unlink" -d 'Remove symlinks for a project (keeps memory)'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "archive" -d 'Move a project\'s memory to archive/ (or restore it with --restore)'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "open" -d 'Open a project\'s KB directory (or the knowledge-base root) in your editor or file manager'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "status" -d 'Overview of the knowledge-base on this machine'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "doctor" -d 'Health check across the knowledge-base'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "projects" -d 'List all projects in the knowledge-base'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "clone-refs" -d 'Clone missing reference repos from ref/README.md'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "sync" -d 'Sync with remote: git pull --rebase + re-link'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "global-sync" -d 'Bidirectional sync of the whole KB: pull if safe, then push everything'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "pull" -d 'Pull knowledge-base updates (fast-forward only)'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "push" -d 'Push changes for specific projects to remote'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "export" -d 'Export project memory to a portable tarball'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "import" -d 'Import project memory from a tarball'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "completions" -d 'Generate shell completions'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "man" -d 'Generate man page'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "work" -d 'Start working on a project (sync + link + track)'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "done" -d 'Finish working: commit + push tracked projects'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "search" -d 'Search memory files across the knowledge-base'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "log" -d 'Show recent knowledge-base history from git'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "rules" -d 'Ensure the personal kb-rules.md map for a project'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "subscribe" -d 'Subscribe this device to a project\'s memory (selective sync)'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "unsubscribe" -d 'Stop receiving a project\'s memory on this device'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "config" -d 'Inspect and edit machine-local configuration'
complete -c kb -n "__fish_kb_using_subcommand help; and not __fish_seen_subcommand_from init link unlink archive open status doctor projects clone-refs sync global-sync pull push export import completions man work done search log rules subscribe unsubscribe config help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c kb -n "__fish_kb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "list" -d 'List all configuration values'
complete -c kb -n "__fish_kb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "get" -d 'Get a single configuration value'
complete -c kb -n "__fish_kb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "set" -d 'Set a configuration value'
complete -c kb -n "__fish_kb_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "unset" -d 'Reset a configuration key to its default'
