{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "utils";
      };
    };
  };
  outputs = { self, nixpkgs, utils, rust-overlay, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustFlags = (if pkgs.stdenv.isLinux then ''
            -C linker=clang -C link-arg=-fuse-ld=${pkgs.mold}/bin/mold
          '' else ''
            -C linker=clang -C link-arg=-fuse-ld=lld -Z threads=8
          '');
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShell = with pkgs; mkShell {
          buildInputs = [
            just
            cargo-watch
            rustToolchain
            tailwindcss
          ] ++ lib.optionals stdenv.isDarwin [libiconv llvmPackages.bintools]
          ++ lib.optionals stdenv.isLinux [mold clang];

          RUSTFLAGS = "${rustFlags}";
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/src";
        };
      });
}

