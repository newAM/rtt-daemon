# rtt-daemon

[![CI](https://github.com/newAM/rtt-daemon/workflows/CI/badge.svg)](https://github.com/newAM/rtt-daemon/actions?query=branch%3Amain)

A daemon to retrieve RTT logs using [probe-rs].

## Usage

This is designed to be used with [NixOS].

* Add this repository to your flake inputs:

```nix
{
  inputs = {
    unstable.url = "github:nixos/nixpkgs/nixos-unstable";

    rtt-daemon = {
      url = "github:newam/rtt-daemon/main";
      inputs.nixpkgs.follows = "unstable";
    };
  };
}
```

* Add `rtt-daemon.overlays.default` to `nixpkgs.overlays`.
* Import the `rtt-daemon.nixosModules.default` module.
* Configure:

```nix
{
  services.rtt-daemon = {
    enable = true;
    probeVid = "1209";
    probePid = "4853";
    probeSerial = "130018001650563641333620";
    chip = "STM32WLE5JCIx";
    elf = "/path/to/my/binary";
  };
}
```

[probe-rs]: https://probe.rs/
[NixOS]: https://nixos.org/
