{
  description = "A Nix flake to build a Rust program and package it into a Docker container for ARM";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # The name of the Rust package (derived from Cargo.toml)
        rustPackageName = "myrss";

          assetsDir = builtins.path { path = ./assets; name = "assets"; };
          assetsPkg = pkgs.runCommand "assets" {} ''
            mkdir -p $out/assets-derivations
            cp -r ${assetsDir} $out/assets-derivations/
          '';

        makeRustPackage = targetPkgs: targetPkgs.rustPlatform.buildRustPackage {
          name = rustPackageName;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
        makeDockerImage = targetPkgs: targetPkgs.dockerTools.buildImage {
          name = rustPackageName;
          tag = "latest";
          # contents = [ (makeRustPackage targetPkgs) ./assets ];
          copyToRoot = [ (makeRustPackage targetPkgs) assetsPkg ];
          runAsRoot = /* sh */ ''
            #!${pkgs.runtimeShell}
            mkdir -p assets
            mv assets-derivations/*/* assets/
            rm -rf assets-derivations
          '';
          config = {
            Cmd = [ "${makeRustPackage targetPkgs}/bin/${rustPackageName}" ];
          };
        };

        # Cross-compilation for ARM (aarch64)
        armPkgs = import nixpkgs {
          system = "x86_64-linux"; # Build system (change if needed)
          crossSystem = {
            config = "aarch64-unknown-linux-gnu"; # Target system (ARM)
          };
        };
      in
      {
        packages = rec {
          myrss = makeRustPackage pkgs;
          myrss-arm = makeRustPackage armPkgs;

          docker = makeDockerImage pkgs;
          docker-arm = makeDockerImage armPkgs;

          default = myrss;
        };
      }
    );
}
