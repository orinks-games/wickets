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

A release reaches its own channel and every less steady one:

| Release | Stable players | Beta players | Alpha players |
|---------|----------------|--------------|---------------|
| stable  | yes            | yes          | yes           |
| beta    |                | yes          | yes           |
| alpha   |                |              | yes           |

A copy shows what is ready for the channel it was built for, so each
channel gets a copy built for it. A stable tag builds three copies of the
same code per platform: the plain one for stable, and
`wickets-1.9.0-windows-x64-for-beta.zip` and `...-for-alpha.zip`, each with
its own `update-<channel>-<platform>.json` in the same GitHub release. An
alpha player who takes the stable release keeps every alpha game.
(Offering alpha players a stable package once hid their alpha games:
1.4.0-beta.3 and 1.3.2 did this.)

`tools/release_channels.py` decides which channels get a copy. A channel
is left out when it already has a release as new as this one or newer,
such as a 1.8.3 patch while 1.9.0 alphas are out. A player's copy looks
only at the newest release that carries its channel's manifest, so
publishing an older version there would hide the newer one. Those players
get the fixes in their channel's next release.

Each build job runs `tools/check_channel_manifests.py`, which fails before
anything is published unless the job's one manifest is for its own
channel and names a package built for that channel. Alpha copies also
ignore any manifest whose package was not built for alpha (`OWN_BUILD` in
`crates/parlor-core/src/channel.rs`).

The updater only offers versions newer than the installed copy, so bump
`Cargo.toml` right after each stable release, before the next beta or
alpha tag.

## Tagging from the cloud

The Tag release workflow (`.github/workflows/tag-release.yml`) tags and
releases with no one's computer involved. Run it from the Actions tab or
the API with `tag` (like `v1.9.0-alpha.1`) and `commit` (the full SHA).
It checks the tag against Cargo.toml at that commit, and for a stable tag
that `docs/release-notes/<tag>.md` is there. Then it pushes the tag and
starts the Release workflow on it. Set `dry_run` to only run the checks.
It doesn't tag wickets-secrets: the Release workflow builds the overlay
commit that release.yml names (its main when none is named).

## Commands (Windows, PowerShell or cmd)

From the Echo-Parlor checkout, on `main`, up to date:

```
git pull
git tag v1.4.0-alpha.1
git push origin v1.4.0-alpha.1
```

The Release workflow builds Windows, macOS and Linux, a copy for each
channel the release reaches, and publishes them to `orinks-games/wickets`
as a prerelease named "Wicket's 1.4.0-alpha.1". To
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

When branching from an older tag, copy main's `.github/workflows/`
unchanged, with `tools/release_channels.py` and
`tools/check_channel_manifests.py`: the cloud tag workflow can only tag a
commit whose workflows match main's.

Add notes in `docs/release-notes/v1.3.1.md`, then push the branch and its
matching tag. The Release workflow uses the tagged branch's workspace
version, so main can stay on 1.4.0. Keep the same fixes on main for the next
minor release; do not merge the patch branch's older version number into main.

Players already running a newer alpha or beta do not downgrade to the patch,
and the patch builds no copy for their channel. They receive the fixes in a
later release of their development version.
