#compdef ctl
# CTL v2.0 ZSH Completion
# Install: place in ~/.zsh/completions/ or fpath

_ctl() {
    local -a commands
    commands=(
        'add:Add a new CTL item'
        'query:Query CTL items'
        'complete:Mark task as completed'
        'metric:Update metric'
        'today:Show high-priority tasks'
        'report:Generate report'
        'graph:Visualize dependencies'
    )
    
    local -a kinds
    kinds=(T A B F M S R P D C E)
    
    local -a priorities
    priorities=(1 2 3 4 5)
    
    _arguments -C \
        '1: :->command' \
        '*:: :->args'
    
    case $state in
        command)
            _describe -t commands 'ctl command' commands
            ;;
        args)
            case $words[1] in
                add)
                    _message 'JSON object or key=value pairs'
                    _values 'example' '{"k":"T","id":"task_id","t":"Task title","p":4}'
                    ;;
                query)
                    _arguments \
                        '(-k --kind)'{-k,--kind}'[Filter by kind]:kind:(T A B F M S R P D C E)' \
                        '(-p --priority)'{-p,--priority}'[Filter by priority]:priority:(1 2 3 4 5)' \
                        '(-d --depends-on)'{-d,--depends-on}'[Filter by dependency]:dependency:' \
                        '(-f --flags)'{-f,--flags}'[Filter by flags]:flags:'
                    ;;
                complete)
                    # Complete task IDs
                    local -a task_ids
                    if command -v ctl >/dev/null 2>&1; then
                        task_ids=(${(f)"$(ctl query --kind T 2>/dev/null | jq -r '.id' 2>/dev/null)"})
                        _describe -t task_ids 'task ID' task_ids
                    else
                        _message 'task ID'
                    fi
                    ;;
                metric)
                    case $CURRENT in
                        2)
                            _values 'action' 'update'
                            ;;
                        3)
                            # Complete metric IDs
                            local -a metric_ids
                            if command -v ctl >/dev/null 2>&1; then
                                metric_ids=(${(f)"$(ctl query --kind M 2>/dev/null | jq -r '.id' 2>/dev/null)"})
                                _describe -t metric_ids 'metric ID' metric_ids
                            else
                                _message 'metric ID'
                            fi
                            ;;
                        *)
                            _arguments \
                                '(-c --current)'{-c,--current}'[Current value]:value:'
                            ;;
                    esac
                    ;;
                report)
                    _arguments \
                        '(-f --format)'{-f,--format}'[Output format]:format:(text markdown)'
                    ;;
                graph)
                    _arguments \
                        '(-f --format)'{-f,--format}'[Output format]:format:(mermaid)'
                    ;;
            esac
            ;;
    esac
}

# Helper function for JSON completion
_ctl_json_keys() {
    local -a json_keys
    json_keys=(
        'k:kind (T/A/B/F/M/S/R/P/D/C/E)'
        'id:unique identifier'
        't:title (max 40 chars)'
        'p:priority (1-5)'
        'e:effort (ISO8601 duration)'
        'd:dependencies array'
        'r:result code'
        'm:metric object'
        'f:flags array'
    )
    _describe -t json_keys 'JSON key' json_keys
}

_ctl "$@"