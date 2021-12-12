use anyhow::Context;
use clap::Parser;
use probe_rs::{config::MemoryRegion, Core, DebugProbeSelector, Probe, Session, Target};
use probe_rs_rtt::{Rtt, ScanRegion, UpChannel};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

const DEFAULT_SLEEP: Duration = Duration::from_millis(10);
const MAX_SLEEP: Duration = Duration::from_secs(3);

fn rtt_region<P>(elf_path: P) -> anyhow::Result<ScanRegion>
where
    P: AsRef<Path>,
{
    use object::{Object, ObjectSymbol};

    let bin_data: Vec<u8> =
        std::fs::read(elf_path).with_context(|| "failed to read server-radio ELF file")?;
    let obj_file =
        object::File::parse(&*bin_data).with_context(|| "failed to parse server-radio ELF file")?;
    for symbol in obj_file.symbols() {
        if symbol.name() == Ok("_SEGGER_RTT") {
            let addr: u32 = symbol
                .address()
                .try_into()
                .with_context(|| "_SEGGER_RTT symbol is not located at a 32-bit address")?;
            return Ok(ScanRegion::Exact(addr));
        }
    }
    Err(anyhow::Error::msg("no _SEGGER_RTT symbol found"))
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// Target chip.
    chip: String,
    /// Probe to use, 'VID:PID' or 'VID:PID:Serial'.
    probe: DebugProbeSelector,
    /// Path to file to write RTT output.
    log: PathBuf,
    /// The path to the ELF file to be flashed.
    #[clap(long)]
    elf: Option<PathBuf>,
    /// Connect to the target under reset
    #[clap(long)]
    connect_under_reset: bool,
}

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| std::process::exit(0)).with_context(|| "failed to set SIGINT handler")?;

    let args: Args = Args::parse();

    let target: Target =
        probe_rs::config::get_target_by_name(args.chip).with_context(|| "chip not found")?;

    let probe: Probe = Probe::open(args.probe).with_context(|| "failed to open probe")?;

    let mut session: Session = if args.connect_under_reset {
        probe.attach_under_reset(target)
    } else {
        probe.attach(target)
    }
    .with_context(|| "failed to attach to the target")?;

    let memory_map: Vec<MemoryRegion> = session.target().memory_map.clone();

    let mut core: Core = session
        .core(0)
        .with_context(|| "failed to attach to core 0")?;

    let scan_region: Option<ScanRegion> = if let Some(elf) = args.elf {
        match rtt_region(elf) {
            Ok(region) => Some(region),
            Err(e) => {
                log::error!("failed to get RTT region from ELF: {}", e);
                None
            }
        }
    } else {
        None
    };

    let mut rtt: Rtt = if let Some(scan_region) = scan_region {
        Rtt::attach_region(&mut core, &memory_map, &scan_region)
    } else {
        Rtt::attach(&mut core, &memory_map)
    }
    .with_context(|| "failed to attach to RTT")?;

    let upch: UpChannel = rtt
        .up_channels()
        .take(0)
        .with_context(|| "failed to attach to RTT up channel 0")?;

    let mut file: File = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(args.log)
        .with_context(|| "failed to open log file")?;

    let mut sleep_time: Duration = DEFAULT_SLEEP;
    let mut prev_n_bytes: usize = 0;
    loop {
        let mut buf: Vec<u8> = vec![0; 16 * 1024];
        let n_bytes: usize = upch
            .read(&mut core, &mut buf)
            .with_context(|| "failed to read RTT channel")?;

        if n_bytes == 0 {
            if prev_n_bytes != 0 {
                file.flush()
                    .with_context(|| "failed to flush RTT log file")?;
            }
            sleep(sleep_time);
            if sleep_time < MAX_SLEEP {
                sleep_time *= 2;
            }
        } else {
            sleep_time = DEFAULT_SLEEP;
            let filled_buf: &[u8] = &buf[..n_bytes];
            file.write_all(filled_buf)
                .with_context(|| "failed to write RTT data to log file")?;
        }

        prev_n_bytes = n_bytes;
    }
}
