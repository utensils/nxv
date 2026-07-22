---
layout: home
title: nxv docs
titleTemplate: ':title — Find any version of any Nix package, instantly.'
hero:
  name: nxv
  text: Any version. Any package. Instantly.
  tagline: Find any version of any Nix package, instantly.
  image:
    src: /nix-snowflake.svg
    alt: Nix snowflake
  actions:
    - theme: brand
      text: $ try it now
      link: https://nxv.urandom.io/
    - theme: alt
      text: 'read the guide →'
      link: /guide/
    - theme: alt
      text: source
      link: https://github.com/utensils/nxv
features:
  - icon:
      src: /icons/zap.svg
      alt: ''
      width: 21
      height: 21
    title: Blazingly fast
    details:
      A Bloom filter returns instant "not found" responses; SQLite FTS5 powers
      full-text search across millions of versions.
  - icon:
      src: /icons/grid.svg
      alt: ''
      width: 21
      height: 21
    title: Complete history
    details:
      Every channel-released nixpkgs version since 2016 — including nested sets
      like <code>python3Packages</code> — at Hydra-built, cache-backed commits.
  - icon:
      src: /icons/shield.svg
      alt: ''
      width: 21
      height: 21
    title: Offline-first
    details:
      Download the ~220MB index once and query locally — no network required
      after the initial sync.
  - icon:
      src: /icons/shield-alert.svg
      alt: ''
      width: 21
      height: 21
    title: Security aware
    details:
      Clear indicators for insecure packages and known CVEs, with an opt-in
      toggle to include them.
---

<div class="nxv-section-kicker">// quick example</div>

## Stop spelunking through commits

```bash
# which nixpkgs commit shipped python 2.7?
nxv search python 2.7

# find a specific version
nxv search python --version 3.11

# get the full version timeline
nxv history python311

# pin it in one line
nix shell nixpkgs/e4a45f9#python27
```
