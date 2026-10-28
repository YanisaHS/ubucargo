#!/bin/sh
# Stand in for APT while keeping each invocation observable.
set -eu
fixture=$UBUCARGO_TEST_FIXTURE

reject_call() {
    printf '%s\n' "$*" >> "$fixture/unexpected-calls"
    printf 'unexpected fixture command: %s\n' "$*" >&2
    exit 99
}

case "${0##*/}" in
    apt-get)
        sources=
        lists=
        status=
        architecture=
        while [ "${1:-}" = -o ]; do
            case "$2" in
                Dir::Etc::sourcelist=*) sources=${2#*=} ;;
                Dir::State::lists=*) lists=${2#*=} ;;
                Dir::State::status=*) status=${2#*=} ;;
                APT::Architecture=*) architecture=${2#*=} ;;
            esac
            shift 2
        done
        [ "$sources" = "$fixture/cache/ubucargo/apt/sources.sources" ] || reject_call "APT source path: $sources"
        [ "$lists" = "$fixture/cache/ubucargo/apt/lists/" ] || reject_call "APT list path: $lists"
        [ "$status" = "$fixture/cache/ubucargo/apt/status" ] || reject_call "APT status path: $status"
        [ "$architecture" = amd64 ] || reject_call "APT architecture: $architecture"
        case "${1:-}" in
            update)
                [ "$#" = 1 ] || reject_call "apt-get $*"
                printf 'apt-get update\n' >> "$fixture/calls"
                if [ -f "$fixture/update-error" ]; then
                    cat "$fixture/update-error" >&2
                    exit 100
                fi
                ;;
            indextargets)
                [ "$#" = 3 ] && [ "$2" = --format ] || reject_call "apt-get $*"
                [ "$3" = '$(FILENAME)|$(SITE)|$(REPO_URI)|$(RELEASE)|$(COMPONENT)|$(ARCHITECTURE)|$(IDENTIFIER)' ] || reject_call "APT index format: $3"
                printf 'apt-get indextargets\n' >> "$fixture/calls"
                cat "$fixture/indextargets"
                ;;
            *) reject_call "apt-get $*" ;;
        esac
        ;;
    *) reject_call "$0 $*" ;;
esac
