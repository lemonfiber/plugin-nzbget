# NZBGet for lemonfiber

**Downloads from Usenet.**
NZBGet is a Usenet download client.
Without it: No Usenet downloads.

## How lemonfiber carries it

This repository holds NZBGet's manifest and the adapter that speaks
lemonfiber's `download.usenet` contract in front of NZBGet: a Rust crate built
on the core's adapter kit, into the image `ghcr.io/lemonfiber/nzbget-adapter`,
published from every commit on `main` and from each tag the release train cuts.
The adapter asks NZBGet's JSON-RPC API for its queue, its rate, its schedule,
its categories and its news servers, and changes them through the same API.
lemonfiber mints the name and password of NZBGet's control account and writes
them into NZBGet's starting `nzbget.conf` before NZBGet first starts, and into
the adapter's `upstream.json`, and hands them to nothing else.

## What is here

| Path | What it is |
| --- | --- |
| `plugin.toml` | The part's manifest |
| `config/nzbget/` | The files NZBGet starts with, written into its configuration directory |
| `recordings/nzbget/` | The responses its proofs are held to |
| `src/` | The adapter |
| `Dockerfile` | The adapter's image |
| `targets.toml` | The lemonfiber its checks run against, and the core the adapter is built on |
| `proofs.json` | What those checks said, committed |

## Checks

The plugin verdict is one job of spec's
[`first-party-plugin`](https://github.com/lemonfiber/spec/blob/main/.github/workflows/first-party-plugin.yml)
workflow: the manifest as the lemonfiber `targets.toml` names reads it, every
proof against its recording, and every pinned digest. The adapter is held to its
format, its lints, its tests, full line coverage and its dependency policy, its
image runs as lemonfiber runs it, and `recordings` takes every recording again
against the pinned NZBGet. `just ci` runs the gates that need nothing but this
tree.

## Licence

The code and data in this repository are under the licence in `LICENSE`.
NZBGet itself is under GPL-2.0-or-later and is not distributed here: this
repository names an image, it does not contain one.
