# AGENTS.md — plugin-nzbget

> **Start at the roadmap and board on [lemonfiber.app](https://lemonfiber.app),
> rendered from the report of where every unreleased version stands. Then the
> rules** every repository shares:
> [working in the repositories](https://github.com/lemonfiber/spec/blob/main/50-governance/working-in-the-repositories.md)
> and [the rules for agents](https://github.com/lemonfiber/spec/blob/main/50-governance/ai-contributors.md).
> This file holds only what is true of this repository.

## What this repo is

NZBGet as a first-party lemonfiber plugin: `plugin.toml`, its starting file
`config/nzbget/nzbget.conf`, and the adapter that speaks the
`download.usenet@1` contract in front of NZBGet, the second Usenet client, which
stands in for SABnzbd where `F14-R31` proves `download.usenet` by a substitute.
The adapter is the Rust crate at the root, built on `lemonfiber-adapter-kit` from
the core at the commit `Cargo.toml` pins, into the image the `Dockerfile` builds.
`release.yml` publishes that image from every commit on `main` and from the
train's tag.

The adapter asks NZBGet's JSON-RPC API at `/jsonrpc`, signed in with the control
account lemonfiber minted. NZBGet takes up a changed setting only on a reload,
which also starts it fetching at its own rate, so the adapter writes the rate it
is held to with the settings and stops a stopped NZBGet again afterwards.

## The rules you cannot break

- **The plugin id is the service's id** (`ARCH-R266`). The container, the
  configuration directory, the settings and the journal keep their names.
- **Its proofs' recordings sit under `recordings/nzbget/`**, and its starting
  file under `config/nzbget/` (`ARCH-R271`), which names the account lemonfiber
  mints as `{{secret:upstream-username}}` and `{{secret:upstream-password}}`
  (`ARCH-R316`).
- **Every service declares `reaches` and `asks_for`** (`ARCH-R270`).
- **The reader is spec's `first-party-plugin` workflow's,** and lemonfiber
  records the recordings itself. There is no Python here.
- **No `unwrap`, `expect`, `panic` or indexing** outside tests. The lints in
  `Cargo.toml` are the core's.
- **Every value the core sends is held to its shape** before it is written into
  NZBGet's settings.
- **The adapter never writes its credential.** lemonfiber mints the control
  account NZBGet answers to and writes it into the adapter's `upstream.json` as
  `username` and `password`, as the adapter's `upstream` maps them (`ARCH-R315`,
  `ARCH-R317`); the adapter reads it for every call and never logs it.
- **NZBGet never starts with its well-known account.** NZBGet's own template
  holds `nzbget` and `tegbzn6789`; the starting file holds the minted account
  instead, which the proof `nzbget.guarded` holds the pinned image to.
- **The core's crates move with the train.** The `rev` in `Cargo.toml` is the
  core commit `targets.toml` names, the same for every `lemonfiber-*` crate.
- **Moving NZBGet's digest** means taking `nzbget.conf` from the new image's
  template again, recording every proof's response again against the new image
  in the same change, moving `RELEASE` and `DIGEST` in `src/adapter.rs` with it,
  and committing the `proofs.json` the checks write.

## Checks

```
just ci
```

The gates that need nothing but this tree. The plugin verdict and the
conformance recordings run in CI; `just recordings` takes the recordings again
locally, with Docker.
