{ pkgs ? import <nixpkgs> { } }:

let
  respeaker-rs =
    let
      defaultNix = builtins.fetchurl {
        url = "https://raw.githubusercontent.com/brookman/respeaker-rs/daf0a6a828b1a413206fabfeada84810687fbe01/default.nix";
        sha256 = "sha256:005y36wvh83jj0niw2m89diskp70g3451hkb1ipc022fk68k6dfc";
      };
    in pkgs.callPackage defaultNix {
      src = pkgs.fetchFromGitHub {
        owner = "brookman";
        repo = "respeaker-rs";
        rev = "daf0a6a828b1a413206fabfeada84810687fbe01";  # REPLACE WITH A TAG!
        sha256 = "sha256-aG2lzH7Yfvsa113GJGILO0wumha6n4btChgGA3CxrU8=";
      };
    };
  in [
    respeaker-rs
  ]