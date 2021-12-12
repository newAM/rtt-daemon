# rtt-daemon

[![CI](https://github.com/newAM/rtt-daemon/workflows/CI/badge.svg)](https://github.com/newAM/rtt-daemon/actions?query=branch%3Amain)

A daemon to retrieve RTT logs using [probe-rs].

## Usage

```text
rtt-daemon 0.1.0

Alex Martens <alexmgit@protonmail.com>

A daemon to retrieve RTT logs.

USAGE:
    rtt-daemon [OPTIONS] <CHIP> <PROBE> <LOG>

ARGS:
    <CHIP>     Target chip
    <PROBE>    Probe to use, 'VID:PID' or 'VID:PID:Serial'
    <LOG>      Path to file to write RTT output

OPTIONS:
        --connect-under-reset    Connect to the target under reset
        --elf <ELF>              The path to the ELF file to be flashed
    -h, --help                   Print help information
    -V, --version                Print version information
```

```bash
rtt-daemon STM32H743ZITx 0483:374e:005500353438511834313939 log.txt --connect-under-reset --elf ~/project/target/thumbv7em-none-eabihf/debug/cec
```

[probe-rs]: https://probe.rs/
