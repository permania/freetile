{
  description = "Tile on your terms.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            pkg-config

            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
          ];
        };

        packages.default = craneLib.buildPackage {
          src = ./.;
          buildInputs = [];
        };
      }
    );
}
