# rtt-daemon

[![CI](https://github.com/newAM/rtt-daemon/workflows/CI/badge.svg)](https://github.com/newAM/rtt-daemon/actions?query=branch%3Amain)

A daemon to retrieve RTT logs using [probe-rs].

## Usage

```text
rtt-daemon 0.1.0
A daemon to retrieve RTT logs.

USAGE:
    rtt-daemon [OPTIONS] <CHIP> <PROBE>

ARGS:
    <CHIP>     Target chip
    <PROBE>    Probe to use, 'VID:PID' or 'VID:PID:Serial'

OPTIONS:
        --connect-under-reset    Connect to the target under reset
        --elf <ELF>              Path to the ELF file to speed up locating the RTT control block
    -h, --help                   Print help information
    -V, --version                Print version information
```

```bash
rtt-daemon STM32H743ZITx 0483:374e:005500353438511834313939 --connect-under-reset --elf ~/project/target/thumbv7em-none-eabihf/debug/cec
```

[probe-rs]: https://probe.rs/
