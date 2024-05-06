{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rustFlags = (if pkgs.stdenv.isLinux then ''
            -C linker=clang -C link-arg=-fuse-ld=${pkgs.mold}/bin/mold
          '' else ''
            -C linker=clang -C link-arg=-fuse-ld=lld -Z threads=8
          '');
      in
      {
        devShell = with pkgs; mkShell {
          buildInputs = [
            just
            cargo-watch
            rustup
            tailwindcss
          ] ++ lib.optionals stdenv.isDarwin [libiconv llvmPackages.bintools]
          ++ lib.optionals stdenv.isLinux [mold clang];

          RUSTFLAGS = "${rustFlags}";
        };
      });
}

