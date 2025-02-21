{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default =
          with pkgs;
          let
            ovmf = OVMF.fd;
          in
          mkShell {
            buildInputs = [
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              cargo-bootimage
              qemu
              cargo-binutils
              ovmf
              gdb
              rust-analyzer
              lldb
              pkgsCross.riscv64.stdenv.cc
            ];
            shellHook = ''
              export OVMF_PATH="${ovmf}/FV/OVMF.fd"
            '';
          };
      }
    );
}
