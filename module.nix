{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.corsairmi-mqtt;
  pkg = pkgs.callPackage ./package.nix { };
in
{
  options.services.corsairmi-mqtt = {
    enable = mkEnableOption "corsairmi-mqtt";
  };

  config = mkIf cfg.enable {
    users.groups.corsairmi = { };

    services.udev.extraRules = ''
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1b1c", ATTRS{idProduct}=="1c06", GROUP="corsairmi", MODE="0660"
    '';

    systemd.services.corsairmi-mqtt = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      description = "corsairmi-mqtt";
      serviceConfig = {
        Type = "idle";
        KillSignal = "SIGINT";
        ExecStart = "${pkg}/bin/corsairmi-mqtt";

        # hardening
        SupplementaryGroups = [ "corsairmi" ];
        DynamicUser = true;
        DevicePolicy = "closed";
        CapabilityBoundingSet = "";
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];
        DeviceAllow = [
          "char-usb_device rwm"
          "/dev/hidraw0 rwm"
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
          "~@clock"
          "~@debug"
          "~@module"
          "~@mount"
          "~@raw-io"
          "~@reboot"
          "~@swap"
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
