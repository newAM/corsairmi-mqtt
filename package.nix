{ lib, rustPlatform }:

let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
rustPlatform.buildRustPackage {
  inherit (cargoToml.package) version;
  pname = cargoToml.package.name;

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  extraInstallCommands = ''
    mkdir -p $out/etc/udev/rules.d
    echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1b1c", ATTRS{idProduct}=="1c06", GROUP="corsairmi", MODE="0660"' >> $out/etc/udev/rules.d/99-corsair.rules
  '';

  meta = with lib; {
    inherit (cargoToml.package) description;
    homepage = cargoToml.package.repository;
    licenses = with licenses; [ mit ];
  };
}
