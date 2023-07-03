{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flake-utils, nixpkgs, ...}:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        frameworks = if pkgs.stdenv.isDarwin then pkgs.darwin.apple_sdk.frameworks else null;
      in
      {
        packages.default = import ./default.nix {
          inherit pkgs;
        };

        devShells.default = import ./shell.nix {
          inherit pkgs;
        };
      }
    );
}
