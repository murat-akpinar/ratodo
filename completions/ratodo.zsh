#compdef ratodo
# ratodo — zsh completion. Hand-written: see docs/cli.md#shells.
#
# Install:  cp ratodo.zsh ~/.zfunc/_ratodo   (with ~/.zfunc on $fpath)

_ratodo() {
    local -a commands themes
    commands=(
        'add:capture a task and exit'
        'done:mark the one matching task done'
        'list:print the agenda'
        'status:print the counts, for a bar'
        'sync:regenerate todo.ics'
        'theme:look at the colours'
    )
    themes=(catppuccin-mocha catppuccin-latte gruvbox-dark nord dracula terminal)

    _arguments -C \
        '(-f --file)'{-f,--file}'[use a different list]:path:_files' \
        "--theme[run once with a built-in theme]:name:($themes)" \
        '-a[capture a task and exit]' \
        '-l[print the agenda]' \
        '--help[show help]' \
        '--version[show the version]' \
        '1: :->command' \
        '*:: :->args'

    case "$state" in
        command) _describe 'command' commands ;;
        args)
            case "$words[1]" in
                list)
                    _arguments \
                        '*--tag[only tasks with this tag]:tag:' \
                        '--prio[only this priority]:level:(high med low)' \
                        '(-t --today)'{-t,--today}'[only overdue and due today]' \
                        '--porcelain[tab-separated output for scripts]'
                    ;;
                status) _arguments '--json[waybar format]' ;;
                theme)  _values 'what' 'list[list the built-in themes]' 'dump[print the active theme]' ;;
            esac
            ;;
    esac
}

_ratodo "$@"
