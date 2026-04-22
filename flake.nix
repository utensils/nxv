{
  description = "nxv - Nix Version Index";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devshell.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      flake = {
        # Overlay for use in NixOS/home-manager configs
        overlays.default = final: prev: {
          nxv = inputs.self.packages.${prev.system}.nxv;
          nxv-indexer = inputs.self.packages.${prev.system}.nxv-indexer;
        };

        # NixOS module for running nxv as a service
        nixosModules.default = import ./nix/module.nix {
          flakePackages = inputs.self.packages;
        };
        nixosModules.nxv = inputs.self.nixosModules.default;
      };

      perSystem =
        {
          system,
          lib,
          ...
        }:
        let
          pkgs = import inputs.nixpkgs {
            localSystem = system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          # Derive a stable Docker image timestamp from the flake's lastModifiedDate.
          # Format: 20260108123456 -> 2026-01-08T12:34:56Z
          lastModified = toString (inputs.self.lastModifiedDate or "19700101000000");
          dockerTimestamp =
            "${builtins.substring 0 4 lastModified}-${builtins.substring 4 2 lastModified}-${
              builtins.substring 6 2 lastModified
            }"
            + "T${builtins.substring 8 2 lastModified}:${builtins.substring 10 2 lastModified}:${
              builtins.substring 12 2 lastModified
            }Z";

          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "rustfmt"
              "clippy"
            ];
          };

          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

          # Source filter: include Cargo sources plus the embedded `frontend/`
          # and the minisign public key under `keys/`.
          src = lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              (craneLib.filterCargoSources path type)
              || (builtins.match ".*frontend.*" path != null)
              || (builtins.match ".*keys/.*\\.pub$" path != null);
          };

          crateInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };

          # Git revision for version string (available when flake is in a git repo)
          gitRev = inputs.self.shortRev or inputs.self.dirtyShortRev or "";

          commonArgs = {
            inherit src;
            inherit (crateInfo) pname version;
            strictDeps = true;

            NXV_GIT_REV = gitRev;

            buildInputs = [
              pkgs.openssl
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];

            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.installShellFiles
            ];
          }
          // lib.optionalAttrs pkgs.stdenv.isDarwin {
            # Xcode clang needs an explicit path to Nix-provided libiconv;
            # build scripts and the final link both look it up via -liconv.
            LIBRARY_PATH = "${pkgs.libiconv}/lib";
            NIX_LDFLAGS = "-L${pkgs.libiconv}/lib";
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          installCompletions = ''
            installShellCompletion --cmd nxv \
              --bash <($out/bin/nxv completions bash) \
              --zsh <($out/bin/nxv completions zsh) \
              --fish <($out/bin/nxv completions fish)
          '';

          nxv = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;

              postInstall = installCompletions;

              meta = {
                description = "Nix Version Index";
                homepage = "https://github.com/utensils/nxv";
                license = lib.licenses.mit;
                maintainers = [ ];
                mainProgram = "nxv";
              };
            }
          );

          nxv-indexer = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--features indexer";
              pname = "nxv-indexer";

              buildInputs = commonArgs.buildInputs ++ [ pkgs.libgit2 ];

              nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
                pkgs.cmake
                pkgs.git
              ];

              postInstall = installCompletions;

              meta = {
                description = "Nix Version Index (with indexer feature)";
                homepage = "https://github.com/utensils/nxv";
                license = lib.licenses.mit;
                maintainers = [ ];
                mainProgram = "nxv";
              };
            }
          );

          # Static musl build (Linux only). Uses cross-compilation so build
          # scripts don't crash inside the sandbox.
          nxv-static =
            let
              isLinux = pkgs.stdenv.isLinux;
              target =
                if system == "aarch64-linux" then "aarch64-unknown-linux-musl" else "x86_64-unknown-linux-musl";
              pkgsMusl =
                if system == "aarch64-linux" then
                  pkgs.pkgsCross.aarch64-multiplatform-musl
                else
                  pkgs.pkgsCross.musl64;
              muslCC = "${pkgsMusl.stdenv.cc}/bin/${pkgsMusl.stdenv.cc.targetPrefix}cc";
              rustToolchainMusl = pkgs.rust-bin.stable.latest.default.override {
                targets = [ target ];
              };
              craneLibMusl = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchainMusl;

              muslBuildArgs = {
                inherit src;
                inherit (crateInfo) pname version;
                strictDeps = true;

                NXV_GIT_REV = gitRev;

                CARGO_BUILD_TARGET = target;
                CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static -C linker=${muslCC}";

                HOST_CC = "${pkgs.stdenv.cc}/bin/cc";
                TARGET_CC = muslCC;
                CC_x86_64_unknown_linux_musl = muslCC;
                CC_aarch64_unknown_linux_musl = muslCC;

                hardeningDisable = [ "fortify" ];

                nativeBuildInputs = [
                  pkgs.pkg-config
                  pkgsMusl.stdenv.cc
                ];

                buildInputs = [ ];
              };

              cargoArtifactsMusl = craneLibMusl.buildDepsOnly muslBuildArgs;
            in
            if isLinux then
              craneLibMusl.buildPackage (
                muslBuildArgs
                // {
                  pname = "nxv-static";
                  cargoArtifacts = cargoArtifactsMusl;

                  nativeBuildInputs = muslBuildArgs.nativeBuildInputs ++ [ pkgs.installShellFiles ];

                  postInstall = installCompletions;

                  meta = {
                    description = "Nix Version Index (static musl binary)";
                    homepage = "https://github.com/utensils/nxv";
                    license = lib.licenses.mit;
                    maintainers = [ ];
                    mainProgram = "nxv";
                    platforms = [
                      "x86_64-linux"
                      "aarch64-linux"
                    ];
                  };
                }
              )
            else
              pkgs.runCommand "nxv-static-unavailable" { } ''
                echo "nxv-static is only available on Linux" >&2
                exit 1
              '';

          # Cross-compile nxv to aarch64-linux-musl from x86_64-linux.
          nxv-static-aarch64 =
            let
              isLinuxX86 = pkgs.stdenv.isLinux && system == "x86_64-linux";
              target = "aarch64-unknown-linux-musl";
              pkgsMusl = pkgs.pkgsCross.aarch64-multiplatform-musl;
              muslCC = "${pkgsMusl.stdenv.cc}/bin/${pkgsMusl.stdenv.cc.targetPrefix}cc";
              rustToolchainMusl = pkgs.rust-bin.stable.latest.default.override {
                targets = [ target ];
              };
              craneLibMusl = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchainMusl;

              muslBuildArgs = {
                inherit src;
                inherit (crateInfo) pname version;
                strictDeps = true;

                NXV_GIT_REV = gitRev;

                CARGO_BUILD_TARGET = target;
                CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static -C linker=${muslCC}";

                HOST_CC = "${pkgs.stdenv.cc}/bin/cc";
                TARGET_CC = muslCC;
                CC_aarch64_unknown_linux_musl = muslCC;

                hardeningDisable = [ "fortify" ];

                nativeBuildInputs = [
                  pkgs.pkg-config
                  pkgsMusl.stdenv.cc
                ];

                buildInputs = [ ];
              };

              cargoArtifactsMusl = craneLibMusl.buildDepsOnly muslBuildArgs;
            in
            if isLinuxX86 then
              craneLibMusl.buildPackage (
                muslBuildArgs
                // {
                  pname = "nxv-static-aarch64";
                  cargoArtifacts = cargoArtifactsMusl;

                  # Skip tests — can't run aarch64 binary on x86_64 without QEMU.
                  doCheck = false;

                  nativeBuildInputs = muslBuildArgs.nativeBuildInputs ++ [ pkgs.installShellFiles ];

                  # Skip shell completions — can't run aarch64 binary on x86_64.
                  postInstall = "";

                  meta = {
                    description = "Nix Version Index (static aarch64 musl binary)";
                    homepage = "https://github.com/utensils/nxv";
                    license = lib.licenses.mit;
                    maintainers = [ ];
                    mainProgram = "nxv";
                    platforms = [ "x86_64-linux" ];
                  };
                }
              )
            else
              pkgs.runCommand "nxv-static-aarch64-unavailable" { } ''
                echo "nxv-static-aarch64 cross-compilation is only available on x86_64-linux" >&2
                exit 1
              '';

          nxv-docker =
            if pkgs.stdenv.isLinux then
              pkgs.dockerTools.buildLayeredImage {
                name = "nxv";
                tag = crateInfo.version;
                created = dockerTimestamp;

                contents = [
                  nxv-indexer
                  pkgs.cacert
                  pkgs.tzdata
                  pkgs.git
                ];

                config = {
                  Entrypoint = [ "${nxv-indexer}/bin/nxv" ];
                  Cmd = [ "serve" ];
                  ExposedPorts = {
                    "8080/tcp" = { };
                  };
                  Env = [
                    "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
                    "TZ=UTC"
                  ];
                  Labels = {
                    "org.opencontainers.image.title" = "nxv";
                    "org.opencontainers.image.description" = "Nix Version Index - search nixpkgs package history";
                    "org.opencontainers.image.source" = "https://github.com/utensils/nxv";
                    "org.opencontainers.image.version" = crateInfo.version;
                  };
                };
              }
            else
              pkgs.runCommand "nxv-docker-unavailable" { } ''
                echo "Docker images are only available on Linux" >&2
                exit 1
              '';
        in
        {
          _module.args.pkgs = pkgs;

          packages = {
            inherit
              nxv
              nxv-indexer
              nxv-static
              nxv-static-aarch64
              nxv-docker
              ;
            default = nxv;
          };

          apps = {
            default = {
              type = "app";
              program = "${nxv}/bin/nxv";
              meta = nxv.meta;
            };
            nxv = {
              type = "app";
              program = "${nxv}/bin/nxv";
              meta = nxv.meta;
            };
            nxv-indexer = {
              type = "app";
              program = "${nxv-indexer}/bin/nxv";
              meta = nxv-indexer.meta;
            };
          };

          checks = {
            inherit nxv;

            nxv-clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            nxv-test = craneLib.cargoTest (
              commonArgs
              // {
                inherit cargoArtifacts;
              }
            );

            nxv-fmt = craneLib.cargoFmt { inherit src; };

            # Static accessibility audit: html5 validity + custom WCAG script.
            # Runs fully offline, so safe for `nix flake check` / CI.
            nxv-a11y =
              let
                pythonWithDeps = pkgs.python3.withPackages (ps: [
                  ps.beautifulsoup4
                  ps.wcag-contrast-ratio
                ]);
              in
              pkgs.runCommand "nxv-a11y"
                {
                  nativeBuildInputs = [
                    pkgs.html5validator
                    pythonWithDeps
                  ];
                }
                ''
                  cp -r ${./frontend} frontend
                  cp ${./scripts/a11y_check.py} a11y_check.py
                  html5validator --root frontend --match "*.html" \
                    --ignore 'error: CSS:' \
                            'The only allowed value for the "type" attribute for the "style" element'
                  python3 a11y_check.py frontend/index.html
                  touch $out
                '';
          };

          devshells.default = {
            motd = ''
              {202}nxv{reset} — Nix Version Index ({bold}${system}{reset})
              $(type menu &>/dev/null && menu)
            '';

            packagesFrom = [ nxv ];

            packages = [
              rustToolchain
              pkgs.cargo-watch
              pkgs.cargo-edit
              pkgs.cargo-outdated
              pkgs.cargo-audit
              pkgs.cargo-llvm-cov
              pkgs.git
              pkgs.gh
              pkgs.jq
              pkgs.miniserve # Simple HTTP server for frontend dev
              pkgs.prettier # HTML/JS/CSS formatter
              pkgs.markdownlint-cli # Markdown linter
              pkgs.k6 # Load testing tool
              # Accessibility tooling — static checks run offline, the dynamic
              # pa11y-ci check uses `npx` which fetches from npm on first run.
              pkgs.html5validator
              (pkgs.python3.withPackages (ps: [
                ps.beautifulsoup4
                ps.wcag-contrast-ratio
              ]))
              pkgs.nodejs_22
            ];

            env = [
              {
                name = "RUST_BACKTRACE";
                value = "1";
              }
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              {
                name = "LIBRARY_PATH";
                value = "${pkgs.libiconv}/lib";
              }
              {
                name = "NIX_LDFLAGS";
                value = "-L${pkgs.libiconv}/lib";
              }
            ];

            commands = [
              {
                category = "build";
                name = "build";
                help = "cargo build (debug)";
                command = "cargo build \"$@\"";
              }
              {
                category = "build";
                name = "build-indexer";
                help = "cargo build with indexer feature";
                command = "cargo build --features indexer \"$@\"";
              }
              {
                category = "build";
                name = "build-release";
                help = "cargo build --release";
                command = "cargo build --release \"$@\"";
              }
              {
                category = "check";
                name = "check";
                help = "cargo check --features indexer";
                command = "cargo check --features indexer \"$@\"";
              }
              {
                category = "check";
                name = "clippy";
                help = "cargo clippy --features indexer -- -D warnings (matches CI)";
                command = "cargo clippy --features indexer \"$@\" -- -D warnings";
              }
              {
                category = "check";
                name = "fmt";
                help = "cargo fmt";
                command = "cargo fmt \"$@\"";
              }
              {
                category = "check";
                name = "fmt-check";
                help = "cargo fmt --check (matches CI)";
                command = "cargo fmt --check \"$@\"";
              }
              {
                category = "check";
                name = "run-tests";
                help = "cargo test --features indexer (matches CI)";
                command = "cargo test --features indexer \"$@\"";
              }
              {
                category = "check";
                name = "ci-local";
                help = "run the same sequence CI runs: fmt-check, clippy, test";
                command = ''
                  set -euo pipefail
                  cargo fmt --all -- --check
                  cargo clippy --features indexer --all-targets -- -D warnings
                  cargo test --features indexer
                '';
              }
              {
                category = "check";
                name = "flake-check";
                help = "nix flake check (full Nix CI checks)";
                command = "nix flake check \"$@\"";
              }
              {
                category = "check";
                name = "a11y";
                help = "static WCAG + HTML5 validation of frontend/index.html";
                command = ''
                  set -euo pipefail
                  # Ignore CSS messages — the bundled vnu.jar predates Tailwind v4
                  # (oklch, @theme, @layer, color-mix). Our static WCAG script
                  # handles the design-token checks instead.
                  # Ignore CSS messages — the bundled vnu.jar predates Tailwind v4
                  # (oklch, @theme, @layer, color-mix). Our static WCAG script
                  # handles the design-token checks instead. Also skip the
                  # `type="text/tailwindcss"` style-attribute complaint.
                  html5validator --root frontend --match "*.html" \
                    --ignore 'error: CSS:' \
                            'The only allowed value for the "type" attribute for the "style" element' \
                    "$@"
                  python3 scripts/a11y_check.py frontend/index.html
                '';
              }
              {
                category = "check";
                name = "a11y-live";
                help = "run pa11y-ci against a running nxv serve (needs `dev` in another terminal)";
                command = ''
                  set -euo pipefail
                  if ! command -v curl >/dev/null 2>&1 || ! curl -sf http://localhost:8080/ >/dev/null; then
                    echo "a11y-live: no server on http://localhost:8080" >&2
                    echo "           start one first with: dev" >&2
                    exit 1
                  fi
                  exec npx --yes pa11y-ci --config frontend/.pa11yci.json "$@"
                '';
              }
              {
                category = "check";
                name = "coverage";
                help = "test coverage summary (pass --html for a browsable report)";
                command = ''
                  set -euo pipefail
                  LLVM_COV="$(find /nix/store -maxdepth 3 -name llvm-cov 2>/dev/null | head -1)"
                  LLVM_PROFDATA="$(find /nix/store -maxdepth 3 -name llvm-profdata 2>/dev/null | head -1)"
                  export LLVM_COV LLVM_PROFDATA
                  if [ "''${1:-}" = "--html" ]; then
                    cargo llvm-cov --features indexer --html --output-dir target/coverage
                    echo "Report: target/coverage/html/index.html"
                  else
                    cargo llvm-cov --features indexer --summary-only
                  fi
                '';
              }
              {
                category = "run";
                name = "nxv";
                help = "run nxv";
                command = "cargo run -- \"$@\"";
              }
              {
                category = "run";
                name = "nxv-indexer";
                help = "run nxv with the indexer feature";
                command = "cargo run --features indexer -- \"$@\"";
              }
              {
                category = "run";
                name = "serve";
                help = "start the nxv API server";
                command = "cargo run -- serve \"$@\"";
              }
              {
                category = "run";
                name = "frontend-dev";
                help = "serve the static frontend from ./frontend on :3030";
                command = "miniserve --index index.html ./frontend \"$@\"";
              }
              {
                category = "run";
                name = "dev";
                help = "API server + live frontend reload (cargo watch on src, disk-served HTML/JS on :8080)";
                command = ''
                  set -euo pipefail
                  export NXV_FRONTEND_DIR="''${NXV_FRONTEND_DIR:-$PWD/frontend}"
                  echo "nxv dev · serving frontend from $NXV_FRONTEND_DIR"
                  echo "nxv dev · http://localhost:8080  (edit frontend/* → just refresh the browser)"
                  echo "nxv dev · src/ changes → cargo-watch auto-rebuilds and restarts"
                  exec cargo watch -q -w src -w Cargo.toml -x 'run -- serve --port 8080' "$@"
                '';
              }
              {
                category = "deps";
                name = "deps-outdated";
                help = "list outdated crates";
                command = "cargo outdated \"$@\"";
              }
              {
                category = "deps";
                name = "deps-audit";
                help = "run cargo audit";
                command = "cargo audit \"$@\"";
              }
              {
                category = "deps";
                name = "deps-update";
                help = "cargo update + nix flake update";
                command = ''
                  set -euo pipefail
                  cargo update
                  nix flake update
                '';
              }
            ];
          };

          treefmt = {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            programs.rustfmt = {
              enable = true;
              edition = "2024";
            };
          };
        };
    };
}
