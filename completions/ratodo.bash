# ratodo — bash completion. Hand-written: see docs/cli.md#shells.
#
# Install:  cp ratodo.bash ~/.local/share/bash-completion/completions/ratodo
#           (or source it from ~/.bashrc)

_ratodo() {
    local cur prev commands global
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    commands="add done list status sync theme help"
    global="--file --theme --help --version"

    case "$prev" in
        --file|-f)
            COMPREPLY=( $(compgen -f -- "$cur") )
            return
            ;;
        --theme)
            COMPREPLY=( $(compgen -W "catppuccin-mocha catppuccin-latte gruvbox-dark nord dracula terminal" -- "$cur") )
            return
            ;;
        --prio)
            COMPREPLY=( $(compgen -W "high med low" -- "$cur") )
            return
            ;;
    esac

    # Which subcommand are we inside, if any.
    local i sub=""
    for (( i=1; i < COMP_CWORD; i++ )); do
        case "${COMP_WORDS[i]}" in
            add|done|list|status|sync|theme) sub="${COMP_WORDS[i]}"; break ;;
        esac
    done

    case "$sub" in
        list)   COMPREPLY=( $(compgen -W "--tag --prio --porcelain $global" -- "$cur") ) ;;
        status) COMPREPLY=( $(compgen -W "--json $global" -- "$cur") ) ;;
        theme)  COMPREPLY=( $(compgen -W "list dump $global" -- "$cur") ) ;;
        # `add` and `done` take free text, and guessing at it would get in the way.
        add|done) COMPREPLY=( $(compgen -W "$global" -- "$cur") ) ;;
        *)      COMPREPLY=( $(compgen -W "$commands $global" -- "$cur") ) ;;
    esac
}

complete -F _ratodo ratodo
