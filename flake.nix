{
  description = "bwu_redux_devtools: Redux DevTools GUI + gRPC server for bwu_redux stores";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      devenv,
      ...
    }@inputs:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # Stable toolchain matching the crate's `rust-version = "1.88"`; no
        # nightly needed since the crate builds on stable.
        rust-toolchain = pkgs.rust-bin.stable."1.88.0".default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain rust-toolchain;

        # wasm-bindgen-cli must match the wasm-bindgen crate version in
        # Cargo.lock, but nixpkgs lags behind, so build the exact version
        # ourselves. After a wasm-bindgen bump this fails with a hash
        # mismatch — paste the two "got:" hashes from the error over these.
        wasmBindgenCli =
          let
            version =
              (pkgs.lib.findFirst (p: p.name == "wasm-bindgen") null
                (builtins.fromTOML (builtins.readFile ./Cargo.lock)).package
              ).version;
          in
          pkgs.buildWasmBindgenCli rec {
            src = pkgs.fetchCrate {
              pname = "wasm-bindgen-cli";
              inherit version;
              hash = "sha256-zRawtjxMOdTMX+mZaiNR3YYfTiZJhf9qj7kXSSeMxrc=";
            };
            cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
              inherit src;
              inherit (src) pname version;
              hash = "sha256-aZCfgR23Qb0Pn4Mm4ToMtuuRQqSJjXCR9li/VvP5CTM=";
            };
          };

        # GTK/WebKit stack needed by the Dioxus desktop GUI.
        dioxusBuildInputs = with pkgs; [
          atk
          cairo
          gdk-pixbuf
          glib
          gtk3
          harfbuzz
          libappindicator
          libsoup_3
          webkitgtk_4_1
        ];

        commonArgs = {
          src = pkgs.lib.cleanSourceWith { src = ./.; };
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            pkg-config
            protobuf
          ];
          # Scope both the deps-only build and the final build to just the
          # server binary — without this, `buildDepsOnly` would build the
          # default (desktop) feature set instead, pulling in the whole
          # GTK/WebKit/tao stack (and openssl-sys via dioxus-desktop's
          # hot-reload websocket client) for no reason.
          pname = "bwu_redux_devtools_server";
          cargoExtraArgs = "--bin bwu_redux_devtools_server --no-default-features --features standalone-server";
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        bwu_redux_devtools_server = craneLib.buildPackage (
          commonArgs // { inherit cargoArtifacts; }
        );
      in
      {
        packages = {
          default = bwu_redux_devtools_server;
          inherit bwu_redux_devtools_server;
        };

        apps.default = {
          type = "app";
          program = "${bwu_redux_devtools_server}/bin/bwu_redux_devtools_server";
        };

        devShells.default = devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [
            {
              packages =
                with pkgs;
                [
                  dioxus-cli
                  tailwindcss_4
                  binaryen # wasm-opt, needed by `dx build --release`
                  wasmBindgenCli
                  cargo-binutils
                  lld_22
                  nodejs_22
                  protobuf
                  pkg-config
                ]
                ++ dioxusBuildInputs;

              languages.rust = {
                enable = true;
                toolchain = {
                  cargo = rust-toolchain;
                  clippy = rust-toolchain;
                  rust-analyzer = rust-toolchain;
                  rustc = rust-toolchain;
                  rustfmt = pkgs.rust-bin.nightly.latest.rustfmt; # rustfmt.toml uses unstable options
                };
              };

              env.GDK_BACKEND = "x11";

              scripts."dxdesk".exec = ''
                exec dx serve --platform desktop
              '';
              scripts."dxweb".exec = ''
                exec dx serve --platform web --port 33333 --no-default-features --features web
              '';
              scripts."dxbuild-web".exec = ''
                set -euo pipefail
                npm install
                npm run css
                # --debug-symbols false: dx defaults to true, which makes
                # wasm-opt keep DWARF; binaryen can crash rewriting DWARF
                # emitted by a newer rustc. Not needed for a release bundle.
                RUSTFLAGS="" RUSTC_WRAPPER="" dx build --release --debug-symbols false \
                  --platform web --no-default-features --features web
                echo "Built: target/dx/bwu_redux_devtools/release/web/public"
              '';
              scripts."deploy".exec = ''
                set -euo pipefail
                out_dir="target/dx/bwu_redux_devtools/release/web/public"
                if [ ! -d "$out_dir" ]; then
                  echo "No release build found; run dxbuild-web first." >&2
                  exit 1
                fi
                if [ -z "''${BWU_REDUX_DEVTOOLS_DEPLOY_DIR:-}" ]; then
                  echo "Built bundle at: $out_dir"
                  echo "Set BWU_REDUX_DEVTOOLS_DEPLOY_DIR to copy it somewhere."
                  exit 0
                fi
                mkdir -p "$BWU_REDUX_DEVTOOLS_DEPLOY_DIR"
                cp -r "$out_dir/." "$BWU_REDUX_DEVTOOLS_DEPLOY_DIR/"
                echo "Deployed $out_dir -> $BWU_REDUX_DEVTOOLS_DEPLOY_DIR"
              '';
            }
          ];
        };
      }
    );
}
