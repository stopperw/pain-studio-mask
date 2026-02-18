{
  description = "Pain Studio Mask development flake";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
	rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
	rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
		rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
          targets = [ "x86_64-pc-windows-gnu" "i686-pc-windows-gnu" ];
        });
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.pkg-config
			pkgs.just
			pkgs.pkgsCross.mingwW64.stdenv.cc
			pkgs.pkgsCross.mingwW64.windows.pthreads
			pkgs.pkgsCross.mingw32.stdenv.cc
			pkgs.pkgsCross.mingw32.windows.pthreads
			pkgs.pkgsCross.mingw32.windows.mcfgthreads
            rust
          ];

		  CARGO_TARGET_I686_PC_WINDOWS_GNU_RUSTFLAGS = "-Clink-args=-lmcfgthread -C panic=abort -C lto -C embed-bitcode=yes -Zpanic_abort_tests";
		  # cargo build --package wintab32 --target i686-pc-windows-gnu -Zbuild-std=panic_abort,std

          shellHook = ''
		    cp ${pkgs.pkgsCross.mingw32.windows.mcfgthreads}/bin/libmcfgthread-2.dll ./libmcfgthread-2.dll
		    echo PSM shell
          '';
        };
      }
    );
}

