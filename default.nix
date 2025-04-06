{ pkgs ? import <nixpkgs> { }, src ? ./. }:
let
    theSource = src;
in
pkgs.rustPlatform.buildRustPackage rec {
    pname = "respeaker-rs";
    version = "0.1.0";
    src = pkgs.lib.cleanSource "${theSource}";
    cargoLock.lockFile = "${theSource}/Cargo.lock";
    cargoHash = ""
}