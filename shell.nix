{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.libiconv
    pkgs.bacon
  ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
    Cocoa
  ]);

  CARGO_TARGET_DIR = "target";
}
