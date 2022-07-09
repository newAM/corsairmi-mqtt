# corsairmi-mqtt

[![CI](https://github.com/newAM/corsairmi-mqtt/workflows/CI/badge.svg)](https://github.com/newAM/corsairmi-mqtt/actions?query=branch%3Amain)

This is a simple daemon to read power measurements from my power supply, and publish them to a MQTT server.

This code is for reference only, it will not be suitable for most people because it is tailored for my specific setup, consisting of:

* A [NixOS] client.
* A MQTT server with TLS-PSK.

## Configuration Example

```nix
{ config, ... }:

{
  sops.secrets.corsairmi-mqtt-psk = {
    mode = "0440";
    group = "psu";
  };
  services.corsairmi-mqtt = {
    enable = true;
    pskFilePath = config.sops.secrets.corsairmi-mqtt-psk.path;
    ip = "192.168.2.3";
    port = 8883;
    topic = "/home/room/pc_name/psu";
  };
}
```

[NixOS]: https://nixos.org/
