{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, flake-utils, crane, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        craneLib = crane.lib.${system};

        pkgs = (import nixpkgs) {
          inherit system;
        };

        frameworks = if pkgs.stdenv.isDarwin then pkgs.darwin.apple_sdk.frameworks else null;
      in
      {
        packages.default = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
          buildInputs = [
            pkgs.libiconv
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
            [
              frameworks.Cocoa
              frameworks.AppKit
            ]
          );

          NIX_LDFLAGS =
            if pkgs.stdenv.isDarwin
            then "-F${frameworks.Cocoa}/Library/Frameworks -F ${frameworks.AppKit}/Library/Frameworks -framework Cocoa -framework AppKit"
            else "";
        };
      }
    );
}
