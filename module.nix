{ config, lib, pkgs, ... }:


let
  cfg = config.services.rtt-daemon;
in
{
  options.services.rtt-daemon = with lib; {
    enable = mkEnableOption "rtt-daemon";

    probePid = mkOption {
      type = types.str;
      description = "Probe USB product ID.";
      example = "3748";
    };

    probeVid = mkOption {
      type = types.str;
      description = "Probe USB vendor ID.";
      example = "0483";
    };

    probeSerial = mkOption {
      type = types.str;
      description = "Probe serial number.";
      example = "005500353438511834313939";
    };

    chip = mkOption {
      type = types.str;
      example = "STM32H743ZITx";
      description = "Chip to attach to.";
    };

    elf = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "/path/to/binary";
      description = ''
        Path to the ELF file loaded on the target.

        This is used to locate the RTT memory section.
      '';
    };

    group = mkOption {
      type = types.str;
      description = "Group with permissions to use the debug probe.";
      default = "rttdprobe";
    };

    connectUnderReset = mkOption {
      type = types.bool;
      default = false;
      description = "Connect to the target under reset.";
    };

    maxPollRateMillis = mkOption {
      type = types.ints.positive;
      description = "Maximum polling rate in milliseconds.";
      default = 3000;
    };

    minPollRateMillis = mkOption {
      type = types.ints.positive;
      description = "Minimum polling rate in milliseconds.";
      default = 10;
    };
  };

  config = lib.mkIf cfg.enable {
    users.groups."${cfg.group}" = { };

    services.udev.extraRules = ''
      SUBSYSTEMS=="usb", \
        ATTRS{idVendor}=="${cfg.probeVid}", \
        ATTRS{idProduct}=="${cfg.probePid}", \
        TAG+="systemd", \
        ENV{SYSTEMD_ALIAS}+="/dev/rttdprobe", \
        GROUP="${cfg.group}", \
        MODE="0660"
    '';

    systemd.services.rtt-daemon =
      let
        configFile = pkgs.writeText "rtt-daemon-config.json" (builtins.toJSON {
          inherit (cfg) chip elf;
          probe = "${cfg.probeVid}:${cfg.probePid}:${cfg.probeSerial}";
          connect_under_reset = cfg.connectUnderReset;
          min_poll_rate_millis = cfg.minPollRateMillis;
          max_poll_rate_millis = cfg.maxPollRateMillis;
        });
      in
      {
        wantedBy = [ "multi-user.target" ];
        after = [ "dev-rttdprobe.device" ];
        requires = [ "dev-rttdprobe.device" ];
        description = "RTT daemon";
        unitConfig.ReloadPropagatedFrom = "dev-rttdprobe.device";
        serviceConfig = {
          Type = "idle";
          KillSignal = "SIGINT";
          ExecStart = "${pkgs.rtt-daemon}/bin/rtt-daemon ${configFile}";
          Restart = "on-failure";
          RestartSec = 10;

          # hardening
          SupplementaryGroups = [ cfg.group ];
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
