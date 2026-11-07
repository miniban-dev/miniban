# Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

# Shared helpers for the smoke tasks, which test the release binaries from outside the process. Sourcing
# this file creates a temporary directory outside the repository and installs traps that stop the server
# and remove the directory however the task ends.
#
# Binaries run from that directory with an empty environment, so they can't rely on frontend files in the
# repository or on tools such as Deno and Node.js on the PATH.
#
# Key functions: start_server, get, expect_equal, expect_html, run_mb.

work_dir=$(mktemp -d)
readonly work_dir
server_pid=""
base_url=""
response_status=""
response_type=""
response_body=""

# Prints the failure to stderr and ends the task, which runs the exit trap.
die() {
  echo "smoke: FAIL: $*" >&2
  exit 1
}

# Stops the server this task started, escalating to SIGKILL if it ignores SIGTERM.
stop_server() {
  [[ -n $server_pid ]] || return 0
  kill -TERM "$server_pid" 2>/dev/null || true
  local attempt
  for attempt in {1..50}; do
    kill -0 "$server_pid" 2>/dev/null || break
    sleep 0.1
  done
  kill -KILL "$server_pid" 2>/dev/null || true
  wait "$server_pid" 2>/dev/null || true
  server_pid=""
}

# Stops the server, shows its log if the task failed, and removes the temporary directory.
cleanup() {
  local status=$?
  stop_server
  if [[ $status -ne 0 && -s $work_dir/server.log ]]; then
    echo "smoke: server log:" >&2
    cat "$work_dir/server.log" >&2
  fi
  rm -rf "$work_dir"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Prints the absolute path of a release executable, or fails if it hasn't been built.
release_binary() {
  local target_dir=${CARGO_TARGET_DIR:-$MISE_PROJECT_ROOT/target}
  [[ -x $target_dir/release/$1 ]] || die "no $1 in $target_dir/release: run mise run build:release"
  echo "$(cd "$target_dir/release" && pwd)/$1"
}

# Sends one request, so its outcome distinguishes a closed port (curl exit code 7) from any other.
probe() {
  curl --silent --noproxy '*' --output /dev/null --max-time 1 "$base_url/api/health"
}

# Starts the release miniban on a port that must be free, and waits until it answers.
start_server() {
  local port=$1 binary status=0 attempt
  binary=$(release_binary miniban) || exit 1
  base_url=http://127.0.0.1:$port

  probe || status=$?
  [[ $status -eq 7 ]] || die "port $port is already in use"

  (cd "$work_dir" && exec env -i MINIBAN_HOST=127.0.0.1 "MINIBAN_PORT=$port" RUST_LOG=info "$binary") \
    </dev/null >"$work_dir/server.log" 2>&1 &
  server_pid=$!

  for attempt in {1..100}; do
    kill -0 "$server_pid" 2>/dev/null || die "miniban exited during startup"
    if probe; then
      return 0
    fi
    sleep 0.1
  done
  die "miniban did not answer on port $port"
}

# Sends a GET request and stores the outcome in response_status, response_type and response_body. Failing
# if the server has stopped also catches another process having taken the port during startup.
get() {
  local written
  written=$(curl --silent --show-error --noproxy '*' --max-time 5 --output "$work_dir/body" \
    --write-out '%{http_code} %{content_type}' "$base_url$1") || die "GET $1: no response"
  kill -0 "$server_pid" 2>/dev/null || die "GET $1: miniban stopped"
  response_status=${written%% *}
  response_type=${written#* }
  response_body=$(<"$work_dir/body")
}

# Fails unless a value from the response to `label` matches what the contract specifies.
expect_equal() {
  local label=$1 expected=$2 actual=$3
  [[ $actual == "$expected" ]] || die "$label: expected '$expected', got '$actual'"
}

# Checks that a GET request returns the HTML document with status 200.
expect_html() {
  get "$1"
  expect_equal "GET $1: status" 200 "$response_status"
  expect_equal "GET $1: content type" 'text/html; charset=utf-8' "$response_type"
  [[ $response_body == '<!DOCTYPE html>'* ]] || die "GET $1: body is not an HTML document"
  echo "smoke: ok: GET $1 returns the HTML document"
}

# Runs the release mb with an argument, failing unless it succeeds and prints something.
run_mb() {
  local binary output
  binary=$(release_binary mb) || exit 1
  output=$(cd "$work_dir" && env -i "$binary" "$@" 2>&1) || die "mb $*: failed: $output"
  [[ -n $output ]] || die "mb $*: printed nothing"
  echo "smoke: ok: mb $*"
}
