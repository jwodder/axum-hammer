#!/bin/bash
set -ex

samples=25
#workers='5 10 15 20 21 22 23 24 25 26 27 28 29 30 40 50 100'
workers="$(jot -s " " 50)"

cd "$(dirname "$0")"

# The packages need to be built in separate commands due to
# <https://github.com/rust-lang/cargo/issues/4463> or similar:
cargo build -r -p axum-hammer
cargo build -r -p axum-nail

cargo run -q -r -p axum-nail &
trap "kill $!" EXIT

while ! nc -w1 -z 127.0.0.1 8080
do sleep 1
done

now="$(date -u +%Y.%m.%d-%H.%M.%SZ)"
mkdir -p stats/"$now"

cargo run -q -r -p axum-hammer -- run \
    -o "stats/$now/hello.json" \
    "http://127.0.0.1:8080/hello" \
    "$samples" \
    $workers

cargo run -q -r -p axum-hammer -- run \
    -o "stats/$now/sleep-10-25.json" \
    "http://127.0.0.1:8080/sleep?min=10&max=25" \
    "$samples" \
    $workers

for endpoint in subpages subpages-arc subpages-service subpages-service-arc
do
    cargo run -q -r -p axum-hammer -- subpages \
        -o "stats/$now/$endpoint.json" \
        --samples "$samples" \
        "http://127.0.0.1:8080/$endpoint" \
        $workers
done
