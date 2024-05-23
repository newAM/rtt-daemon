use anyhow::Context;
use probe_rs::{
    config::MemoryRegion,
    probe::{list::Lister, DebugProbeSelector, Probe},
    rtt::{Rtt, ScanRegion, UpChannel},
    Core, Session, Target,
};
use serde::Deserialize;
use std::{cmp::min, ffi::OsString, fs::File, io::BufReader, thread::sleep, time::Duration};

fn rtt_region(elf_path: &str) -> anyhow::Result<ScanRegion> {
    use object::{Object, ObjectSymbol, Symbol};

    let bin_data: Vec<u8> = std::fs::read(elf_path)
        .with_context(|| format!("Failed to read ELF file at {elf_path}"))?;
    let obj_file = object::File::parse(&*bin_data).context("Failed to parse ELF file")?;

    let symbol: Symbol = obj_file
        .symbols()
        .find(|symbol| symbol.name() == Ok("_SEGGER_RTT"))
        .context("_SEGGER_RTT symbol not found")?;
    let addr: u32 = symbol
        .address()
        .try_into()
        .context("_SEGGER_RTT symbol is not located at a 32-bit address")?;
    Ok(ScanRegion::Exact(addr.into()))
}

#[derive(Deserialize)]
struct Config {
    /// Target chip.
    chip: String,
    /// Probe to use, 'VID:PID' or 'VID:PID:Serial'.
    probe: String,
    /// Path to the ELF file to speed up locating the RTT control block.
    elf: Option<String>,
    /// Connect to the target under reset.
    connect_under_reset: bool,
    /// Minimum polling rate in milliseconds.
    min_poll_rate_millis: u64,
    /// Maximum polling rate in milliseconds.
    max_poll_rate_millis: u64,
}

fn main() -> anyhow::Result<()> {
    ctrlc::set_handler(|| std::process::exit(0)).context("failed to set SIGINT handler")?;

    systemd_journal_logger::JournalLog::new()
        .context("Failed to initialize logger")?
        .install()
        .context("Failed to install logger")?;
    log::set_max_level(log::LevelFilter::Info);

    let config_file_path: OsString = match std::env::args_os().nth(1) {
        Some(x) => x,
        None => {
            eprintln!(
                "usage: {} [config-file]",
                std::env::args_os()
                    .next()
                    .unwrap_or_else(|| OsString::from("???"))
                    .to_string_lossy()
            );
            std::process::exit(1);
        }
    };

    let config: Config = serde_json::from_reader(BufReader::new(
        File::open(config_file_path).context("Failed to open configuration file")?,
    ))
    .context("Failed to load configuration from file")?;

    if config.min_poll_rate_millis > config.max_poll_rate_millis {
        anyhow::bail!(
            "Minimum pollling rate of {:?} exceeds maximum polling rate of {:?}",
            Duration::from_millis(config.min_poll_rate_millis),
            Duration::from_millis(config.max_poll_rate_millis)
        );
    }

    let target: Target =
        probe_rs::config::get_target_by_name(config.chip).context("Chip not found")?;

    let selector: DebugProbeSelector = config
        .probe
        .try_into()
        .context("Probe selector is invalid")?;

    log::info!("Opening probe");
    let probe: Probe = Lister::new()
        .open(selector)
        .context("Failed to open probe")?;

    log::info!("Attaching to target");
    let permissions = probe_rs::Permissions::new();
    let mut session: Session = if config.connect_under_reset {
        probe.attach_under_reset(target, permissions)
    } else {
        probe.attach(target, permissions)
    }
    .context("Failed to attach to the target")?;

    let memory_map: Vec<MemoryRegion> = session.target().memory_map.clone();

    let mut core: Core = session.core(0).context("Failed to attach to core 0")?;

    let scan_region: Option<ScanRegion> = if let Some(elf) = config.elf {
        match rtt_region(&elf) {
            Ok(region) => Some(region),
            Err(e) => {
                log::error!("Failed to get RTT region from ELF: {e}");
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
        log::info!("Attaching to RTT");
        Rtt::attach(&mut core, &memory_map)
    }
    .context("Failed to attach to RTT")?;

    let upch: UpChannel = rtt
        .up_channels()
        .take(0)
        .context("Failed to attach to RTT up channel 0")?;

    let mut sleep_time_millis: u64 = config.min_poll_rate_millis;
    log::info!("Entering main loop");
    loop {
        let buf: Vec<u8> = {
            let mut buf: Vec<u8> = vec![0; 64 * 1024];
            let n_bytes: usize = upch
                .read(&mut core, &mut buf)
                .context("Failed to read RTT channel")?;
            buf.truncate(n_bytes);
            buf
        };

        if buf.is_empty() {
            sleep(Duration::from_millis(sleep_time_millis));
            sleep_time_millis = sleep_time_millis.saturating_mul(2);
            sleep_time_millis = min(sleep_time_millis, config.max_poll_rate_millis);
        } else {
            sleep_time_millis = config.min_poll_rate_millis;

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
