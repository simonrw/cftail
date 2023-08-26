# custom default.nix to support non-flake environments
{ pkgs ? import <nixpkgs> { } }:
let
  frameworks = pkgs.darwin.apple_sdk.frameworks;
in
pkgs.rustPlatform.buildRustPackage {
  pname = "cftail";
  version = "0.9.0";

  src = pkgs.nix-gitignore.gitignoreSource [ ] ./.;

  cargoHash = "sha256-qHDREJlvJT4Ya6eyoxWRvHX0rjfgwoNBSKwjr5GSD4M=";

  buildInputs = [
    pkgs.libiconv
  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin ([
    frameworks.Cocoa
    frameworks.AppKit
  ]
  );

  NIX_LDFLAGS =
    if pkgs.stdenv.isDarwin
    then "-F${frameworks.Cocoa}/Library/Frameworks -F ${frameworks.AppKit}/Library/Frameworks -framework Cocoa -framework AppKit"
    else "";
}
