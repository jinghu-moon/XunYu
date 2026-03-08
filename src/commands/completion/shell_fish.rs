pub(crate) fn completion_fish() -> &'static str {
    r#"
function __xun_complete
    set -l args (commandline -opc)
    set -e args[1]
    if test (commandline -ct) = ""
        set args $args ""
    end
    if set -q XUN_DISABLE_DYNAMIC_COMPLETE
        return
    end
    set -l out (xun __complete $args)
    set -l sentinel ""
    set -l lines
    for line in $out
        if string match -q "__XUN_COMPLETE__=*" $line
            set sentinel $line
            continue
        end
        set -a lines $line
    end
    if test -z "$sentinel"
        return
    end
    if test (string match -rq "v=([0-9]+)" $sentinel)
        set -l ver (string replace -r '.*v=([0-9]+).*' '$1' $sentinel)
        if test "$ver" != "1"
            return
        end
    else
        return
    end
    for line in $lines
        set -l parts (string split -m1 \t $line)
        if test (count $parts) -gt 1
            printf "%s\t%s\n" $parts[1] $parts[2]
        else
            printf "%s\n" $parts[1]
        end
    end
end

complete -c xun -f -a "(__xun_complete)"
complete -c xyu -f -a "(__xun_complete)"
complete -c xy -f -a "(__xun_complete)"
"#
}
