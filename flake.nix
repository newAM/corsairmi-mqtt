{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    {
      packages = {
        x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.callPackage ./package.nix { };
        aarch64-linux.default = nixpkgs.legacyPackages.aarch64-linux.callPackage ./package.nix { };
      };

      nixosModules.default = import ./module.nix;
    };
}
