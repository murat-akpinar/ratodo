# ratodo — fish completion. Hand-written: see docs/cli.md#shells.
#
# Install:  cp ratodo.fish ~/.config/fish/completions/ratodo.fish

set -l commands add done list status sync theme

complete -c ratodo -f
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -a add -d 'capture a task and exit'
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -s a -d 'capture a task and exit'
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -s l -d 'print the agenda'
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -a done -d 'mark the one matching task done'
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -a list -d 'print the agenda'
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -a status -d 'print the counts, for a bar'
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -a sync -d 'regenerate todo.ics'
complete -c ratodo -n "not __fish_seen_subcommand_from $commands" -a theme -d 'look at the colours'

# Global, so no subcommand condition.
complete -c ratodo -s f -l file -r -F -d 'use a different list'
complete -c ratodo -l theme -x -d 'run once with a built-in theme' \
    -a 'catppuccin-mocha catppuccin-latte gruvbox-dark nord dracula terminal'
complete -c ratodo -l help -d 'show help'
complete -c ratodo -l version -d 'show the version'

complete -c ratodo -n '__fish_seen_subcommand_from list' -l tag -x -d 'only tasks with this tag'
complete -c ratodo -n '__fish_seen_subcommand_from list' -l prio -x -a 'high med low' -d 'only this priority'
complete -c ratodo -n '__fish_seen_subcommand_from list' -s t -l today -d 'only overdue and due today'
complete -c ratodo -n '__fish_seen_subcommand_from list' -l porcelain -d 'tab-separated output for scripts'
complete -c ratodo -n '__fish_seen_subcommand_from status' -l json -d 'waybar format'
complete -c ratodo -n '__fish_seen_subcommand_from theme' -a 'list dump'
