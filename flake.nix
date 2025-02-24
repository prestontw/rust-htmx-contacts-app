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
            -C linker=clang -C link-arg=-fuse-ld=lld
          '');
      in
      {
        devShell = with pkgs; mkShell {
          buildInputs = [
            postgresql
            just
            diesel-cli
            cargo-watch
            rustup
            tailwindcss
            nodePackages.typescript-language-server
          ] ++ lib.optionals stdenv.isDarwin [libiconv llvmPackages.bintools]
          ++ lib.optionals stdenv.isLinux [mold clang];

          RUSTFLAGS = "${rustFlags}";
        };
      });
}

