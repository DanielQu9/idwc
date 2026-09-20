#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  ./scripts/release-check.sh pre-push
  ./scripts/release-check.sh ci [git-ref]

Commands:
  pre-push  Verify a clean release commit locally before git push.
  ci        Wait for every GitHub Actions CI run for git-ref (default: HEAD).
EOF
}

fail() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

repository_root() {
    git rev-parse --show-toplevel 2>/dev/null || fail "not inside a Git repository"
}

log_file="$(mktemp "${TMPDIR:-/tmp}/idwc-release-check.XXXXXX")"
ci_runs_file="$(mktemp "${TMPDIR:-/tmp}/idwc-ci-runs.XXXXXX")"
cleanup() {
    rm -f "$log_file" "$ci_runs_file"
}
trap cleanup EXIT

run_step() {
    local label="$1"
    shift
    printf '... %s\n' "$label"
    if "$@" >"$log_file" 2>&1; then
        printf 'ok  %s\n' "$label"
        return
    else
        local status=$?
        printf 'FAIL %s\n' "$label" >&2
        cat "$log_file" >&2
        exit "$status"
    fi
}

pre_push() {
    require_command cargo
    require_command git

    local root
    root="$(repository_root)"
    cd "$root"

    if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
        git status --short >&2
        fail "pre-push checks require a clean working tree; commit the release first"
    fi

    run_step "git diff validation" git diff --check HEAD
    run_step "rustfmt" cargo fmt --all -- --check
    run_step "tests" cargo test --all-targets --locked
    run_step "clippy" cargo clippy --locked --all-targets --all-features -- -D warnings
    run_step "rustdoc" env RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
    run_step "package file list" cargo package --locked --list
    run_step "publish dry-run (no upload)" cargo publish --locked --dry-run

    printf 'All pre-push checks passed for %s.\n' "$(git rev-parse --short HEAD)"
}

find_ci_runs() {
    local commit="$1"
    local attempt
    for attempt in {1..12}; do
        if gh run list \
            --workflow CI \
            --commit "$commit" \
            --limit 20 \
            --json databaseId,headBranch,url \
            --jq '.[] | [.databaseId, .headBranch, .url] | @tsv' \
            >"$ci_runs_file" 2>"$log_file" && [[ -s "$ci_runs_file" ]]; then
            return
        fi
        sleep 5
    done

    cat "$log_file" >&2
    fail "no GitHub Actions CI runs found for commit $commit"
}

check_ci() {
    require_command gh
    require_command git

    local root ref commit run_id branch url
    root="$(repository_root)"
    cd "$root"
    ref="${1:-HEAD}"
    commit="$(git rev-parse --verify "${ref}^{commit}" 2>/dev/null)" ||
        fail "cannot resolve Git ref: $ref"

    printf 'Looking for CI runs for %s (%s)...\n' "$ref" "${commit:0:12}"
    find_ci_runs "$commit"

    while IFS=$'\t' read -r run_id branch url; do
        [[ -n "$run_id" ]] || continue
        printf '... CI %s (%s)\n' "$branch" "$run_id"
        if gh run watch "$run_id" --exit-status --interval 5 >"$log_file" 2>&1; then
            printf 'ok  CI %s %s\n' "$branch" "$url"
        else
            local watch_status=$?
            printf 'FAIL CI %s %s\n' "$branch" "$url" >&2
            cat "$log_file" >&2
            exit "$watch_status"
        fi
    done <"$ci_runs_file"

    printf 'All CI runs passed for %s.\n' "${commit:0:12}"
}

case "${1:-}" in
    pre-push)
        [[ $# -eq 1 ]] || fail "pre-push does not accept additional arguments"
        pre_push
        ;;
    ci)
        [[ $# -le 2 ]] || fail "ci accepts at most one git-ref"
        check_ci "${2:-HEAD}"
        ;;
    -h | --help | help)
        usage
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac
