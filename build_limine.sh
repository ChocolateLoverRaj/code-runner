#!/usr/bin/env bash
LIMINE_PATH=$(NIX_PATH="nixpkgs=./nixpkgs" nix-build -E "with import <nixpkgs> { }; callPackage ./nixpkgs/pkgs/by-name/li/limine/package.nix { enableAll = true; }")
rm ./limine
ln -s $LIMINE_PATH ./limine
