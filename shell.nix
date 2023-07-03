{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.libiconv
  ];

  CARGO_TARGET_DIR = "target";
}
