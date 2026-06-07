{
  description = "stanchion — network-flow simulator and stanchion queue optimizer";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenixPkgs = fenix.packages.${system};

        # default: latest nightly via fenix (no sha256 pin needed)
        nightly-toolchain = fenixPkgs.latest.withComponents [
          "rustc"
          "cargo"
          "rustfmt"
          "clippy"
          "rust-src"
        ];

        # Kani requires a specific nightly with rustc-dev and llvm-tools.
        # sha256 comes from: nix develop .#kani (first run will print correct hash).
        kani-toolchain = fenixPkgs.fromToolchainFile {
          file   = ./rust-toolchain.toml;
          sha256 = "sha256-mOtNqBEPX2ZFjRP3jOltQ9apjUWxoDzBuw+uN3hFioc=";
        };

        # Verus requires Rust 1.95.0 stable with rustc-dev and llvm-tools.
        verus-toolchain = (fenixPkgs.toolchainOf {
          channel = "1.95.0";
          sha256  = pkgs.lib.fakeHash;
        }).withComponents [
          "rustc"
          "cargo"
          "rustfmt"
          "rust-src"
          "rustc-dev"
          "llvm-tools"
        ];

        common-deps = with pkgs; [
          pkg-config
          openssl
        ];

        # pre-commit hook script (clippy clean check)
        clippy-hook = pkgs.writeShellScript "pre-commit-clippy" ''
          set -e
          cargo clippy --all-targets --all-features -- -D warnings
        '';

      in {
        # default shell: latest nightly for day-to-day dev and testing
        devShells.default = pkgs.mkShell {
          name = "stanchion-dev";
          buildInputs = common-deps ++ [ nightly-toolchain ];
          shellHook = ''
            echo "stanchion dev shell (nightly)"
            echo "  cargo test         -- run all tests"
            echo "  cargo clippy       -- lint"
            echo "  cargo bench        -- benchmarks"
            echo "  nix develop .#kani -- switch to Kani shell"
            # install clippy pre-commit hook
            if [ -d .git ] && [ ! -f .git/hooks/pre-commit ]; then
              ln -sf ${clippy-hook} .git/hooks/pre-commit
              echo "installed clippy pre-commit hook"
            fi
          '';
        };

        # kani shell: pinned nightly for formal verification
        devShells.kani = pkgs.mkShell {
          name = "stanchion-kani";
          buildInputs = common-deps ++ [ kani-toolchain ];
          shellHook = ''
            echo "stanchion dev shell (Kani / pinned nightly)"
            echo "  cargo kani         -- run Kani proofs"
            echo "  cargo test         -- run all tests"
          '';
        };

        # verus shell: stable 1.95.0 for Verus verification
        devShells.verus = pkgs.mkShell {
          name = "stanchion-verus";
          buildInputs = common-deps ++ [
            verus-toolchain
          ];
          shellHook = ''
            echo "stanchion dev shell (Verus / stable 1.95.0)"
            echo "  \$VERUS_BIN <file.rs>  -- verify a file"
            echo "Build Verus first: cd ~/srcs/verus && ./build.sh"
            export VERUS_BIN="$HOME/srcs/verus/target-verus/release/verus"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname        = "stanchion";
          version      = "0.1.0";
          src          = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
      });
}
