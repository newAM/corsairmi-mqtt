{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    crane.inputs.flake-utils.follows = "flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
  }:
    nixpkgs.lib.recursiveUpdate
    (flake-utils.lib.eachSystem ["x86_64-linux" "aarch64-linux"]
      (
        system: let
          pkgs = nixpkgs.legacyPackages.${system};
          cargoToml = nixpkgs.lib.importTOML ./Cargo.toml;
          craneLib = crane.lib.${system};

          commonArgs = {
            src = craneLib.cleanCargoSource ./.;

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            buildInputs = with pkgs; [
              openssl
            ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in {
          packages.default = craneLib.buildPackage (nixpkgs.lib.recursiveUpdate
            commonArgs
            {
              inherit cargoArtifacts;
            });

          devShells.default = pkgs.mkShellNoCC {
            inputsFrom = builtins.attrValues self.checks;
          };

          checks = let
            nixSrc = nixpkgs.lib.sources.sourceFilesBySuffices ./. [".nix"];
          in {
            pkg = self.packages.${system}.default;

            clippy = craneLib.cargoClippy (nixpkgs.lib.recursiveUpdate
              commonArgs
              {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "-- --deny warnings";
              });

            rustfmt = craneLib.cargoFmt {inherit (commonArgs) src;};

            alejandra = pkgs.runCommand "alejandra" {} ''
              ${pkgs.alejandra}/bin/alejandra --check ${nixSrc}
              touch $out
            '';

            statix = pkgs.runCommand "statix" {} ''
              ${pkgs.statix}/bin/statix check ${nixSrc}
              touch $out
            '';
          };
        }
      ))
    {
      overlays.default = final: prev: {
        corsairmi-mqtt = self.packages.${prev.system}.default;
      };
      nixosModules.default = import ./module.nix;
    };
}
