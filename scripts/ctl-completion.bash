#!/bin/bash
# CTL v2.0 Bash Completion
# Install: source ctl-completion.bash

_ctl_complete() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Main commands
    local commands="add query complete metric today report graph"
    
    # Kind types
    local kinds="T A B F M S R P D C E"
    
    # Priorities
    local priorities="1 2 3 4 5"
    
    case "${COMP_CWORD}" in
        1)
            # Complete command names
            COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
            return 0
            ;;
        *)
            case "${COMP_WORDS[1]}" in
                add)
                    # Suggest JSON structure
                    if [[ ${cur} == * ]]; then
                        COMPREPLY=('{"k":"T","id":"","t":""}')
                    fi
                    ;;
                query)
                    case "${prev}" in
                        --kind|-k)
                            COMPREPLY=( $(compgen -W "${kinds}" -- ${cur}) )
                            ;;
                        --priority|-p)
                            COMPREPLY=( $(compgen -W "${priorities}" -- ${cur}) )
                            ;;
                        *)
                            local query_opts="--kind --priority --depends-on --flags"
                            COMPREPLY=( $(compgen -W "${query_opts}" -- ${cur}) )
                            ;;
                    esac
                    ;;
                complete)
                    # Complete task IDs from active tasks
                    if command -v ctl >/dev/null 2>&1; then
                        local task_ids=$(ctl query --kind T 2>/dev/null | jq -r '.id' 2>/dev/null)
                        COMPREPLY=( $(compgen -W "${task_ids}" -- ${cur}) )
                    fi
                    ;;
                metric)
                    if [[ ${COMP_CWORD} -eq 2 ]]; then
                        COMPREPLY=( $(compgen -W "update" -- ${cur}) )
                    elif [[ ${COMP_CWORD} -eq 3 ]]; then
                        # Complete metric IDs
                        if command -v ctl >/dev/null 2>&1; then
                            local metric_ids=$(ctl query --kind M 2>/dev/null | jq -r '.id' 2>/dev/null)
                            COMPREPLY=( $(compgen -W "${metric_ids}" -- ${cur}) )
                        fi
                    elif [[ ${prev} == "--current" || ${prev} == "-c" ]]; then
                        # Suggest numeric value
                        COMPREPLY=()
                    else
                        COMPREPLY=( $(compgen -W "--current -c" -- ${cur}) )
                    fi
                    ;;
                report)
                    case "${prev}" in
                        --format|-f)
                            COMPREPLY=( $(compgen -W "text markdown" -- ${cur}) )
                            ;;
                        *)
                            COMPREPLY=( $(compgen -W "--format -f" -- ${cur}) )
                            ;;
                    esac
                    ;;
                graph)
                    case "${prev}" in
                        --format|-f)
                            COMPREPLY=( $(compgen -W "mermaid" -- ${cur}) )
                            ;;
                        *)
                            COMPREPLY=( $(compgen -W "--format -f" -- ${cur}) )
                            ;;
                    esac
                    ;;
            esac
            ;;
    esac
}

# Register completion
complete -F _ctl_complete ctl
complete -F _ctl_complete ./ctl.py
complete -F _ctl_complete python3\ ctl.py