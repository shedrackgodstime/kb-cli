
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'kb' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'kb'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'kb' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'One-time setup on a new machine')
            [CompletionResult]::new('link', 'link', [CompletionResultType]::ParameterValue, 'Wire a project repository to the knowledge-base')
            [CompletionResult]::new('unlink', 'unlink', [CompletionResultType]::ParameterValue, 'Remove symlinks for a project (keeps memory)')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Move a project''s memory to archive/ (or restore it with --restore)')
            [CompletionResult]::new('open', 'open', [CompletionResultType]::ParameterValue, 'Open a project''s KB directory (or the knowledge-base root) in your editor or file manager')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Overview of the knowledge-base on this machine')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Health check across the knowledge-base')
            [CompletionResult]::new('projects', 'projects', [CompletionResultType]::ParameterValue, 'List all projects in the knowledge-base')
            [CompletionResult]::new('clone-refs', 'clone-refs', [CompletionResultType]::ParameterValue, 'Clone missing reference repos from ref/README.md')
            [CompletionResult]::new('sync', 'sync', [CompletionResultType]::ParameterValue, 'Sync with remote: git pull --rebase + re-link')
            [CompletionResult]::new('global-sync', 'global-sync', [CompletionResultType]::ParameterValue, 'Bidirectional sync of the whole KB: pull if safe, then push everything')
            [CompletionResult]::new('pull', 'pull', [CompletionResultType]::ParameterValue, 'Pull knowledge-base updates (fast-forward only)')
            [CompletionResult]::new('push', 'push', [CompletionResultType]::ParameterValue, 'Push changes for specific projects to remote')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export project memory to a portable tarball')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import project memory from a tarball')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate man page')
            [CompletionResult]::new('work', 'work', [CompletionResultType]::ParameterValue, 'Start working on a project (sync + link + track)')
            [CompletionResult]::new('done', 'done', [CompletionResultType]::ParameterValue, 'Finish working: commit + push tracked projects')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search memory files across the knowledge-base')
            [CompletionResult]::new('log', 'log', [CompletionResultType]::ParameterValue, 'Show recent knowledge-base history from git')
            [CompletionResult]::new('rules', 'rules', [CompletionResultType]::ParameterValue, 'Ensure the personal kb-rules.md map for a project')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe this device to a project''s memory (selective sync)')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Stop receiving a project''s memory on this device')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Inspect and edit machine-local configuration')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'kb;init' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Path to knowledge-base repo (auto-detected if omitted)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;link' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;unlink' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;archive' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--restore', '--restore', [CompletionResultType]::ParameterName, 'Restore an archived project''s memory back to projects/')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;open' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;status' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Show all projects, not just active ones')
            [CompletionResult]::new('--refs', '--refs', [CompletionResultType]::ParameterName, 'Include ref repo details')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;doctor' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--fix', '--fix', [CompletionResultType]::ParameterName, 'Attempt to auto-fix issues where safe')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;projects' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--active-only', '--active-only', [CompletionResultType]::ParameterName, 'Only show active projects')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Include ref counts, handoff dates, disk usage')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;clone-refs' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Process all active projects')
            [CompletionResult]::new('--shallow', '--shallow', [CompletionResultType]::ParameterName, 'Shallow clone (depth=1)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Show what would be cloned without doing it')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Re-clone even if directory exists (fixes revision mismatches)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;sync' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Only re-link this specific project (repeatable)')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;global-sync' {
            [CompletionResult]::new('-m', '-m', [CompletionResultType]::ParameterName, 'Custom commit message')
            [CompletionResult]::new('--message', '--message', [CompletionResultType]::ParameterName, 'Custom commit message')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--no-link', '--no-link', [CompletionResultType]::ParameterName, 'Re-link active projects after syncing (default: on)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview what would happen without changing anything')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;pull' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Only pull and re-link this specific project (repeatable)')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--link', '--link', [CompletionResultType]::ParameterName, 'Re-link active projects after pulling')
            [CompletionResult]::new('--no-link', '--no-link', [CompletionResultType]::ParameterName, 'Only pull, don''t re-link')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;push' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name to push (repeatable)')
            [CompletionResult]::new('-m', '-m', [CompletionResultType]::ParameterName, 'Custom commit message')
            [CompletionResult]::new('--message', '--message', [CompletionResultType]::ParameterName, 'Custom commit message')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;export' {
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output path (default: ~/project-name.tar.gz)')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;import' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Project name (auto-detected from tarball if omitted)')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;completions' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;man' {
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output directory (default: current directory)')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;work' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;done' {
            [CompletionResult]::new('-m', '-m', [CompletionResultType]::ParameterName, 'Custom commit message')
            [CompletionResult]::new('--message', '--message', [CompletionResultType]::ParameterName, 'Custom commit message')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;search' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Only search within these projects (repeatable)')
            [CompletionResult]::new('--glob', '--glob', [CompletionResultType]::ParameterName, 'Only match files that match this glob pattern')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Only show matching file paths, not individual lines')
            [CompletionResult]::new('--case-sensitive', '--case-sensitive', [CompletionResultType]::ParameterName, 'Match case-sensitively (default: insensitive)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;log' {
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Number of commits to show')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Number of commits to show')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Only show commits touching these projects (repeatable)')
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--stat', '--stat', [CompletionResultType]::ParameterName, 'Show per-file change statistics')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;rules' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Refresh every configured project')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Show what would happen without writing anything')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;subscribe' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview what would change without writing anything')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;unsubscribe' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview what would change without writing anything')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;config' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all configuration values')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a single configuration value')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a configuration value')
            [CompletionResult]::new('unset', 'unset', [CompletionResultType]::ParameterValue, 'Reset a configuration key to its default')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'kb;config;list' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;config;get' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;config;set' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;config;unset' {
            [CompletionResult]::new('--kb-root', '--kb-root', [CompletionResultType]::ParameterName, 'Override knowledge-base root discovery')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output machine-readable JSON')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Quiet output — suppress informational messages')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'kb;config;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all configuration values')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a single configuration value')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a configuration value')
            [CompletionResult]::new('unset', 'unset', [CompletionResultType]::ParameterValue, 'Reset a configuration key to its default')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'kb;config;help;list' {
            break
        }
        'kb;config;help;get' {
            break
        }
        'kb;config;help;set' {
            break
        }
        'kb;config;help;unset' {
            break
        }
        'kb;config;help;help' {
            break
        }
        'kb;help' {
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'One-time setup on a new machine')
            [CompletionResult]::new('link', 'link', [CompletionResultType]::ParameterValue, 'Wire a project repository to the knowledge-base')
            [CompletionResult]::new('unlink', 'unlink', [CompletionResultType]::ParameterValue, 'Remove symlinks for a project (keeps memory)')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Move a project''s memory to archive/ (or restore it with --restore)')
            [CompletionResult]::new('open', 'open', [CompletionResultType]::ParameterValue, 'Open a project''s KB directory (or the knowledge-base root) in your editor or file manager')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Overview of the knowledge-base on this machine')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Health check across the knowledge-base')
            [CompletionResult]::new('projects', 'projects', [CompletionResultType]::ParameterValue, 'List all projects in the knowledge-base')
            [CompletionResult]::new('clone-refs', 'clone-refs', [CompletionResultType]::ParameterValue, 'Clone missing reference repos from ref/README.md')
            [CompletionResult]::new('sync', 'sync', [CompletionResultType]::ParameterValue, 'Sync with remote: git pull --rebase + re-link')
            [CompletionResult]::new('global-sync', 'global-sync', [CompletionResultType]::ParameterValue, 'Bidirectional sync of the whole KB: pull if safe, then push everything')
            [CompletionResult]::new('pull', 'pull', [CompletionResultType]::ParameterValue, 'Pull knowledge-base updates (fast-forward only)')
            [CompletionResult]::new('push', 'push', [CompletionResultType]::ParameterValue, 'Push changes for specific projects to remote')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'Export project memory to a portable tarball')
            [CompletionResult]::new('import', 'import', [CompletionResultType]::ParameterValue, 'Import project memory from a tarball')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate man page')
            [CompletionResult]::new('work', 'work', [CompletionResultType]::ParameterValue, 'Start working on a project (sync + link + track)')
            [CompletionResult]::new('done', 'done', [CompletionResultType]::ParameterValue, 'Finish working: commit + push tracked projects')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Search memory files across the knowledge-base')
            [CompletionResult]::new('log', 'log', [CompletionResultType]::ParameterValue, 'Show recent knowledge-base history from git')
            [CompletionResult]::new('rules', 'rules', [CompletionResultType]::ParameterValue, 'Ensure the personal kb-rules.md map for a project')
            [CompletionResult]::new('subscribe', 'subscribe', [CompletionResultType]::ParameterValue, 'Subscribe this device to a project''s memory (selective sync)')
            [CompletionResult]::new('unsubscribe', 'unsubscribe', [CompletionResultType]::ParameterValue, 'Stop receiving a project''s memory on this device')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Inspect and edit machine-local configuration')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'kb;help;init' {
            break
        }
        'kb;help;link' {
            break
        }
        'kb;help;unlink' {
            break
        }
        'kb;help;archive' {
            break
        }
        'kb;help;open' {
            break
        }
        'kb;help;status' {
            break
        }
        'kb;help;doctor' {
            break
        }
        'kb;help;projects' {
            break
        }
        'kb;help;clone-refs' {
            break
        }
        'kb;help;sync' {
            break
        }
        'kb;help;global-sync' {
            break
        }
        'kb;help;pull' {
            break
        }
        'kb;help;push' {
            break
        }
        'kb;help;export' {
            break
        }
        'kb;help;import' {
            break
        }
        'kb;help;completions' {
            break
        }
        'kb;help;man' {
            break
        }
        'kb;help;work' {
            break
        }
        'kb;help;done' {
            break
        }
        'kb;help;search' {
            break
        }
        'kb;help;log' {
            break
        }
        'kb;help;rules' {
            break
        }
        'kb;help;subscribe' {
            break
        }
        'kb;help;unsubscribe' {
            break
        }
        'kb;help;config' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all configuration values')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get a single configuration value')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set a configuration value')
            [CompletionResult]::new('unset', 'unset', [CompletionResultType]::ParameterValue, 'Reset a configuration key to its default')
            break
        }
        'kb;help;config;list' {
            break
        }
        'kb;help;config;get' {
            break
        }
        'kb;help;config;set' {
            break
        }
        'kb;help;config;unset' {
            break
        }
        'kb;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
