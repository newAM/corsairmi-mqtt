{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.corsairmi-mqtt;
in {
  options.services.corsairmi-mqtt = {
    enable = mkEnableOption "corsairmi-mqtt";

    pskFilePath = mkOption {
      type = types.path;
      description = ''
        Path to the PSK file.

        This file has lines in the form of
        <literal>client_id:hex_encoded_psk</literal>;
        the same format used by the MQTT broker.
      '';
    };

    pid = mkOption {
      type = types.str;
      example = "1c06";
      description = ''
        USB product ID as a hex string without a leading "0x".
      '';
    };

    ip = mkOption {
      type = types.str;
      description = ''
        IPv4 of the MQTT server.
      '';
    };

    port = mkOption {
      type = types.ints.u16;
      default = 8883;
      description = ''
        Port of the MQTT server.
      '';
    };

    topic = mkOption {
      type = types.str;
      example = "/home/room/pc_name/psu";
      description = ''
        MQTT topic to publish samples on.
      '';
    };
  };

  config = mkIf cfg.enable {
    users.groups.psu = {};

    services.udev.extraRules = ''
      SUBSYSTEM=="hidraw", \
        ATTRS{idVendor}=="1b1c", \
        ATTRS{idProduct}=="${lib.toLower cfg.pid}", \
        TAG+="systemd", \
        ENV{SYSTEMD_ALIAS}+="/dev/psu", \
        ENV{SYSTEMD_WANTS}+="corsairmi-mqtt.service", \
        GROUP="psu", \
        MODE="0660"
    '';

    systemd.services.corsairmi-mqtt = let
      configFile = pkgs.writeText "corsairmi-mqtt-config.json" (builtins.toJSON {
        inherit (cfg) ip port topic;
        psk_file_path = cfg.pskFilePath;
      });
    in {
      wantedBy = ["multi-user.target"];
      after = ["network-online.target" "dev-psu.device"];
      bindsTo = ["dev-psu.device"];
      description = "Corsair Mi MQTT";
      unitConfig.ReloadPropagatedFrom = "dev-psu.device";
      serviceConfig = {
        Type = "idle";
        KillSignal = "SIGINT";
        ExecStart = "${pkgs.corsairmi-mqtt}/bin/corsairmi-mqtt ${configFile}";
        Restart = "on-failure";
        RestartSec = 10;

        # hardening
        SupplementaryGroups = ["psu"];
        DynamicUser = true;
        DevicePolicy = "closed";
        CapabilityBoundingSet = "";
        RestrictAddressFamilies = ["AF_INET" "AF_INET6" "AF_UNIX"];
        DeviceAllow = [
          "char-usb_device rwm"
          "/dev/hidraw0 rwm"
          "/dev/hidraw1 rwm"
          "/dev/hidraw2 rwm"
          "/dev/hidraw3 rwm"
          "/dev/hidraw4 rwm"
          "/dev/hidraw5 rwm"
        ];
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateMounts = true;
        PrivateTmp = true;
        PrivateUsers = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectSystem = "strict";
        BindPaths = [
          "/dev/bus/usb"
          "/dev/hidraw0"
          "/dev/hidraw1"
          "/dev/hidraw2"
          "/dev/hidraw3"
          "/dev/hidraw4"
          "/dev/hidraw5"
          "/sys/class/hidraw"
        ];
        MemoryDenyWriteExecute = true;
        LockPersonality = true;
        RemoveIPC = true;
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "~@debug"
          "~@mount"
          "~@privileged"
          "~@resources"
          "~@cpu-emulation"
          "~@obsolete"
        ];
        ProtectProc = "invisible";
        ProtectHostname = true;
        ProcSubset = "pid";
      };
    };
  };
}
