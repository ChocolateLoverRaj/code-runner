#!/usr/bin/env bash
NIX_PATH="nixpkgs=./nixpkgs" nix-build -E 'with import <nixpkgs> { }; callPackage ./nixpkgs/pkgs/by-name/ip/ipxe/package.nix { embedScript = ./iPXE/http_boot.ipxe; additionalOptions = ["CERT_CMD" "CONSOLE_CMD" "DOWNLOAD_PROTO_HTTPS" "FCMGMT_CMD" "GDBSERIAL" "GDBUDP" "IMAGE_GZIP" "IMAGE_PNG" "IMAGE_PNM" "IMAGE_UCODE"]; }'
echo "Now copy ./result/ipxe.efi to your test computer and add boot it"
