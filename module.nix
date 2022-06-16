{ config, lib, pkgs, ... }:


let
  cfg = config.services.rtt-daemon;
in
{
  options.services.rtt-daemon = {
    enable = lib.mkEnableOption "rtt-daemon";

    probePid = lib.mkOption {
      type = lib.types.str;
      description = "Probe USB product ID.";
      example = "3748";
    };

    probeVid = lib.mkOption {
      type = lib.types.str;
      description = "Probe USB vendor ID.";
      example = "0483";
    };

    probeSerial = lib.mkOption {
      type = lib.types.str;
      description = "Probe serial number.";
      example = "005500353438511834313939";
    };

    chip = lib.mkOption {
      type = lib.types.str;
      example = "STM32H743ZITx";
    };

    elf = lib.mkOption {
      type = lib.types.str;
      example = "/path/to/binary";
    };
  };

  config = lib.mkIf cfg.enable {
    users.groups.rttdprobe = { };

    services.udev.extraRules = ''
      SUBSYSTEM=="usb", \
        ATTRS{idVendor}=="${cfg.probeVid}", \
        ATTRS{idProduct}=="${cfg.probePid}", \
        TAG+="systemd", \
        GROUP="rttdprobe", \
        MODE="0660"
    '';

    systemd.services.rtt-daemon = {
      wantedBy = [ "multi-user.target" ];
      after = [ "dev-rttdprobe.device" ];
      requires = [ "dev-rttdprobe.device" ];
      description = "RTT daemon";
      unitConfig.ReloadPropagatedFrom = "dev-rttdprobe.device";
      serviceConfig = {
        Type = "idle";
        KillSignal = "SIGINT";
        ExecStart = ''
          ${pkgs.rtt-daemon}/bin/rtt-daemon \
            ${cfg.chip} \
            ${cfg.probeVid}:${cfg.probePid}:${cfg.probeSerial} \
            --elf ${cfg.elf}
        '';
        Restart = "on-failure";
        RestartSec = 10;

        # hardening
        SupplementaryGroups = [ "rttdprobe" ];
        DynamicUser = true;
        DevicePolicy = "closed";
        CapabilityBoundingSet = "";
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
          "AF_NETLINK"
          "AF_UNIX"
        ];
        DeviceAllow = [
          "char-usb_device rwm"
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
          "-${cfg.elf}"
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
