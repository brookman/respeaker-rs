{ pkgs ? import <nixpkgs> { }, src ? ./. }:

let
  theSource = src;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = "respeaker-rs";
  version = "0.1.0";
  src = pkgs.lib.cleanSource "${theSource}";

  nativeBuildInputs = [ pkgs.pkg-config ];  # Needed for build.rs of alsa-sys
  buildInputs = [ pkgs.alsa-lib ];          # Needed so pkg-config can find alsa

  cargoLock.lockFile = "${theSource}/Cargo.lock";
}
