{
  description = "Tile on your terms.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      crane,
      ...
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ inputs.rust-overlay.overlays.default ];
        };

        craneLib = crane.mkLib pkgs;
      in
      {

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            pkg-config

            cargo
            rustc
            rust-bin.nightly.latest.rustfmt
            clippy
            rust-analyzer

            libxkbcommon
          ];
        };

        packages.default = craneLib.buildPackage {
          src = ./.;
          buildInputs = [ ];
        };
      }
    );
}
