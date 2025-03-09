#!/usr/bin/env bash
NIX_PATH="nixpkgs=./nixpkgs" nix-build -E "with import <nixpkgs> { }; callPackage ./nixpkgs/pkgs/by-name/ip/ipxe/package.nix { embedScript = ./iPXE/http_boot.ipxe; }"
echo "Now copy ./result/ipxe.efi to your test computer and add boot it"
