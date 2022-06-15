use anyhow::Context;
use clap::Parser;
use probe_rs::{config::MemoryRegion, Core, DebugProbeSelector, Probe, Session, Target};
use probe_rs_rtt::{Rtt, ScanRegion, UpChannel};
use std::{
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
    use object::{Object, ObjectSymbol, Symbol};

    let bin_data: Vec<u8> = std::fs::read(elf_path).context("failed to read ELF file")?;
    let obj_file = object::File::parse(&*bin_data).context("failed to parse ELF file")?;

    let symbol: Symbol = obj_file
        .symbols()
        .find(|symbol| symbol.name() == Ok("_SEGGER_RTT"))
        .context("_SEGGER_RTT symbol not found")?;
    let addr: u32 = symbol
        .address()
        .try_into()
        .context("_SEGGER_RTT symbol is not located at a 32-bit address")?;
    Ok(ScanRegion::Exact(addr))
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// Target chip.
    #[clap(value_parser)]
    chip: String,
    /// Probe to use, 'VID:PID' or 'VID:PID:Serial'.
    #[clap(arg_enum, value_parser)]
    probe: DebugProbeSelector,
    /// Path to the ELF file to speed up locating the RTT control block.
    #[clap(long, value_parser)]
    elf: Option<PathBuf>,
    /// Connect to the target under reset
    #[clap(long, action)]
    connect_under_reset: bool,
}

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| std::process::exit(0)).context("failed to set SIGINT handler")?;

    systemd_journal_logger::init().context("failed to initialize logging")?;
    log::set_max_level(log::LevelFilter::Info);

    let args: Args = Args::parse();

    let target: Target =
        probe_rs::config::get_target_by_name(args.chip).context("chip not found")?;

    log::info!("Opening probe");
    let probe: Probe = Probe::open(args.probe).context("failed to open probe")?;

    log::info!("Attaching to target");
    let mut session: Session = if args.connect_under_reset {
        probe.attach_under_reset(target)
    } else {
        probe.attach(target)
    }
    .context("failed to attach to the target")?;

    let memory_map: Vec<MemoryRegion> = session.target().memory_map.clone();

    let mut core: Core = session.core(0).context("failed to attach to core 0")?;

    let scan_region: Option<ScanRegion> = if let Some(elf) = args.elf {
        match rtt_region(elf) {
            Ok(region) => Some(region),
            Err(e) => {
                log::error!("failed to get RTT region from ELF: {e}");
                None
            }
        }
    } else {
        None
    };

    let mut rtt: Rtt = if let Some(scan_region) = scan_region {
        log::info!("Attaching to RTT region: {scan_region:?}");
        Rtt::attach_region(&mut core, &memory_map, &scan_region)
    } else {
        log::info!("Attaching to RTT region");
        Rtt::attach(&mut core, &memory_map)
    }
    .context("failed to attach to RTT")?;

    let upch: UpChannel = rtt
        .up_channels()
        .take(0)
        .context("failed to attach to RTT up channel 0")?;

    let mut sleep_time: Duration = DEFAULT_SLEEP;
    log::info!("Entering main loop");
    loop {
        let buf: Vec<u8> = {
            let mut buf: Vec<u8> = vec![0; 64 * 1024];
            let n_bytes: usize = upch
                .read(&mut core, &mut buf)
                .context("failed to read RTT channel")?;
            buf.truncate(n_bytes);
            buf
        };

        if buf.is_empty() {
            sleep(sleep_time);
            if sleep_time < MAX_SLEEP {
                sleep_time *= 2;
            }
        } else {
            sleep_time = DEFAULT_SLEEP;

            let data: String = match std::str::from_utf8(&buf) {
                Ok(s) => s.to_string(),
                Err(e) => {
                    log::warn!("RTT data is not valid UTF-8: {e}");
                    String::from_utf8_lossy(&buf).to_string()
                }
            };

            data.lines().for_each(|line| log::info!("[RTT] {line}"));
        }
    }
}
