[![Project Status: Concept – Minimal or no implementation has been done yet, or the repository is only intended to be a limited example, demo, or proof-of-concept.](https://www.repostatus.org/badges/latest/concept.svg)](https://www.repostatus.org/#concept)
[![CI Status](https://github.com/jwodder/axum-hammer/actions/workflows/test.yml/badge.svg)](https://github.com/jwodder/axum-hammer/actions/workflows/test.yml) <!-- [![codecov.io](https://codecov.io/gh/jwodder/axum-hammer/branch/main/graph/badge.svg)](https://codecov.io/gh/jwodder/axum-hammer) -->
[![Minimum Supported Rust Version](https://img.shields.io/badge/MSRV-1.74-orange)](https://www.rust-lang.org)
[![MIT License](https://img.shields.io/github/license/jwodder/axum-hammer.svg)](https://opensource.org/licenses/MIT)

This is an experimental attempt at creating an MVCE
[axum](https://github.com/tokio-rs/axum) server for investigating performance
issues with concurrent requests to https://webdav.dandiarchive.org (See
dandi/dandidav#54).

The current focus of the experiment is on investigating increased
request+response times as the number of concurrent requests to an axum server
increases, even when the number of concurrent requests is below the number of
CPUs on the server machine.

Repository Contents
===================

- `nail/` — The `axum-nail` package, defining an axum server with a variety of
  endpoints

- `hammer/` — The `axum-hammer` package, defining a client program for making
  repeated requests to the server with different numbers of workers tasks
  issuing concurrent requests.  The command emits a JSON file listing the
  request+response times of all requests, organized by number of workers.

- `batch.sh` — A shell script for running `axum-hammer` against all endpoints
  of `axum-nail` and saving the JSON files under `stats/`

- `plot.py` — A Python script that takes a JSON file emitted by `axum-hammer`
  and produces a PNG containing a boxplot of the request+response times for
  each number of workers
