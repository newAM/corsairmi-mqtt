{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    {
      packages = {
        x86_64-linux.corsairmi-mqtt = nixpkgs.legacyPackages.x86_64-linux.callPackage ./package.nix { };
        aarch64-linux.corsairmi-mqtt = nixpkgs.legacyPackages.aarch64-linux.callPackage ./package.nix { };
      };

      defaultPackage = {
        x86_64-linux = self.packages.x86_64-linux.corsairmi-mqtt;
        aarch64-linux = self.packages.aarch64-linux.corsairmi-mqtt;
      };

      nixosModule = import ./module.nix { };
    };
}
