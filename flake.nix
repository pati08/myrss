{
  description = "My rust devenv nix flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [];
          };
        in {
          # devShells.default = import ./shell.nix { inherit pkgs; inherit inputs; };
          devShells.default = pkgs.mkShell rec {
            buildInputs = with pkgs; [
              cargo-shuttle
              sqlx-cli
              tailwindcss
              dive
            ];
          };
        }
      );
}
