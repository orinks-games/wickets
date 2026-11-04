# Releasing on stable, beta and alpha

Wicket's has three update channels. Players pick one in Settings, Update
channel.

- **Alpha:** new games and features at their roughest, for players who want
  them first.
- **Beta:** work being refined before everyone gets it.
- **Stable:** everyone. A new copy starts here.

Everything merges to `main`. What is not ready for every player names its
stage, and a copy built for a steadier channel leaves it out:

| Copy    | Shows                  |
|---------|------------------------|
| stable  | stable work            |
| beta    | beta and stable work   |
| alpha   | everything             |

A developer build (`cargo run`, the agent server) shows everything.

## Staging a game or a feature

A whole game: add its id to `GAMES` in `crates/parlor-core/src/channel.rs`.
The launcher and event games leave it out of steadier copies. Its saved records stay put.

Part of a game: guard it with `parlor_core::channel::shows(Stage::Beta)`.

Moving work along is one edit: change `Stage::Alpha` to `Stage::Beta`, or
remove the line once it is stable, then release on that channel.

## Tags

The workspace version in `Cargo.toml` is the next stable release. The tag
says which channel a build is for:

| Tag              | Channel | Release on GitHub |
|------------------|---------|-------------------|
| `v1.3.0`         | stable  | normal            |
| `v1.3.0-beta.1`  | beta    | prerelease        |
| `v1.4.0-alpha.1` | alpha   | prerelease        |

A beta or alpha tag must start with the version in `Cargo.toml`; the build
stamps the tag's version in, so About says "1.3.0-beta.1" and Cargo.toml
needs no edit. Numbers count up from 1 for each version. A tag that is
none of these fails the build.

Since 1.3.0-beta.1 sorts before 1.3.0, bump `Cargo.toml` to the next
version right after a stable release, before the next beta or alpha tag.

Release notes go in `docs/release-notes/<tag>.md`, as for stable:
`docs/release-notes/v1.4.0-alpha.1.md`.

## Who gets what

Each release reaches its own channel only:

- A stable release reaches stable players.
- A beta release reaches beta players.
- An alpha release reaches alpha players.

The release workflow publishes one update manifest per platform, for the
tag's channel, and nothing else. `tools/check_channel_manifests.py` fails
the release before anything is published if dist/ holds a manifest for
another channel, and CI fails if the workflow could publish one. A beta or
stable package leaves out alpha-only games, so offering it to alpha
players hides those games (1.4.0-beta.3 and 1.3.2 did this).

Alpha copies also ignore an alpha manifest whose package was not built
for alpha (`OWN_BUILD` in `crates/parlor-core/src/channel.rs`), as if
nothing were published. Copies from before this have no such check.

The updater only offers versions newer than the installed copy, so to
reach alpha players an alpha release must sort after the newest release
they may have installed.

## Commands (Windows, PowerShell or cmd)

From the Echo-Parlor checkout, on `main`, up to date:

```
git pull
git tag v1.4.0-alpha.1
git push origin v1.4.0-alpha.1
```

The Release workflow builds Windows, macOS and Linux, and publishes them to
`orinks-games/wickets` as a prerelease named "Wicket's 1.4.0-alpha.1". To
fix its notes afterwards:

```
gh release edit v1.4.0-alpha.1 --repo orinks-games/wickets --notes-file docs\release-notes\v1.4.0-alpha.1.md
```

To build packages for a channel without a release, run the Release workflow
by hand from the Actions tab and pick the channel.

## Patch releases while the next version is in development

Use a patch version, such as 1.3.1, for corrections to text, instructions,
settings or existing behavior. Use a minor version, such as 1.4.0, for new
player features. Hearken accepts patch versions and offers them as updates
when they are newer than the installed copy.

If main already contains work for the next minor version, create a patch
branch from the latest stable tag. Cherry-pick only the relevant fixes,
set the workspace version to the patch version, and update Cargo.lock with
`cargo update --workspace`. Keep the engine dependency pinned unless the
patch needs an engine fix. Run the normal checks on this release branch.

When branching from an older tag, carry over the current channel policy in
`.github/workflows/release.yml` and `tools/check_channel_manifests.py`: a
release publishes only its own channel's manifest.

Add notes in `docs/release-notes/v1.3.1.md`, then push the branch and its
matching tag. The Release workflow uses the tagged branch's workspace
version, so main can stay on 1.4.0. Keep the same fixes on main for the next
minor release; do not merge the patch branch's older version number into main.

Players already running a newer alpha or beta do not downgrade to the patch.
They receive the fixes in a later release of their development version.
