mod mqtt;
mod psk;

use anyhow::Context;
use corsairmi::PowerSupply;
use mqtt::{ConnectReasonCode, ControlPacket};
use openssl::{
    error::ErrorStack,
    ssl::{Ssl, SslContext, SslContextBuilder, SslMethod, SslStream, SslVersion},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::File,
    io::{self, BufReader, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::PathBuf,
    str,
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct Config {
    psk_file_path: String,
    ip: Ipv4Addr,
    port: u16,
    topic: String,
}

fn mqtt_connect(
    ip: Ipv4Addr,
    port: u16,
    psks: HashMap<String, Vec<u8>>,
) -> anyhow::Result<SslStream<TcpStream>> {
    let mut ssl_context_builder: SslContextBuilder =
        SslContext::builder(SslMethod::tls_client()).context("Failed to create SSL builder")?;

    let any_psk_id: String = psks.iter().next().expect("psks is empty").0.to_string();

    // workaround for an OpenSSL bug
    // https://github.com/openssl/openssl/issues/8534
    ssl_context_builder
        .set_max_proto_version(Some(SslVersion::TLS1_2))
        .context("Failed to set max protocol version")?;

    ssl_context_builder.set_psk_client_callback(move |_, maybe_hint, identity_dst, psk_dst| {
        let (identity_src, key_src): (String, &Vec<u8>) = maybe_hint
            .and_then(|hint_raw| str::from_utf8(hint_raw).ok())
            .and_then(|hint| psks.get(hint).map(|key| (hint.to_string(), key)))
            .unwrap_or_else(|| {
                psks.iter()
                    .next()
                    .map(|(identity, key)| (identity.to_owned(), key))
                    .expect("psks is empty")
            });

        let identity_len_with_nul_term: usize = identity_src.len().saturating_add(1);

        if identity_len_with_nul_term > identity_dst.len() {
            log::error!(
                "Identity length {} greater than destination length {}",
                identity_len_with_nul_term,
                identity_dst.len()
            );
            return Err(ErrorStack::get());
        }

        if key_src.len() > psk_dst.len() {
            log::error!(
                "Key length {} greater than destination length {}",
                key_src.len(),
                psk_dst.len()
            );
            return Err(ErrorStack::get());
        }

        identity_dst[..identity_src.len()].copy_from_slice(identity_src.as_bytes());
        identity_dst[identity_src.len()] = 0;

        psk_dst[..key_src.len()].copy_from_slice(key_src);

        Ok(key_src.len())
    });

    let ssl_context: SslContext = ssl_context_builder.build();

    log::info!("Opening stream");
    let addr: SocketAddr = SocketAddr::new(ip.into(), port);
    let stream: TcpStream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
        .with_context(|| format!("Unable to open TCP stream to MQTT server at {addr}"))?;
    log::info!("TCP connection established");

    let ssl_state: Ssl = Ssl::new(&ssl_context).context("Failed to create SSL state")?;
    let mut ssl_stream: SslStream<TcpStream> = SslStream::new(ssl_state, stream)?;
    ssl_stream.connect().context("SSL handshake failed")?;
    log::info!("TLS connection established");

    let client_id: String = ssl_stream
        .ssl()
        .psk_identity()
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
        .unwrap_or(any_psk_id);
    let connect: Vec<u8> = mqtt::connect(&client_id);

    log::info!("Writing CONNECT");
    ssl_stream
        .write_all(connect.as_ref())
        .context("Failed to write stream")?;
    ssl_stream.flush().context("Failed to flush stream")?;

    let mut read_byte = || -> anyhow::Result<u8> {
        let mut buf: [u8; 1] = [0];
        let start: Instant = Instant::now();
        loop {
            let n: usize = ssl_stream
                .read(&mut buf)
                .context("Failed to read from stream")?;
            if n != 0 {
                return Ok(buf[0]);
            } else {
                let elapsed: Duration = Instant::now().duration_since(start);
                if elapsed > Duration::from_secs(5) {
                    anyhow::bail!("Failed to read byte from stream in {elapsed:?}")
                }
            }
        }
    };

    let byte0: u8 = read_byte().context("Failed to read CONNACK byte 0")?;
    if byte0 >> 4 != (ControlPacket::CONNACK as u8) {
        return Err(anyhow::anyhow!("Response is not CONNACK: {byte0}"));
    }

    const MIN_CONNACK_LEN: u8 = 4;
    let byte1: u8 = read_byte().context("Failed to read CONNACK byte 1")?;
    if byte1 < MIN_CONNACK_LEN {
        return Err(anyhow::anyhow!(
            "CONNACK minimum length is {MIN_CONNACK_LEN} got {byte1}"
        ));
    }

    let _byte2: u8 = read_byte().context("Failed to read CONNACK byte 2")?;
    let byte3: u8 = read_byte().context("Failed to read CONNACK byte 3")?;
    match ConnectReasonCode::try_from(byte3) {
        Ok(ConnectReasonCode::Success) => {
            log::info!("Sucessfully connected to MQTT server");
            Ok(ssl_stream)
        }
        x => Err(anyhow::anyhow!("Server did not accept connection: {x:?}")),
    }
}

fn psu_connect() -> anyhow::Result<PowerSupply> {
    let list: Vec<PathBuf> = corsairmi::list().context("Failed to list PSUs")?;
    let first: &PathBuf = list.first().context("No PSU found")?;
    PowerSupply::open(first).with_context(|| format!("Unable to open {}", first.to_string_lossy()))
}

fn sample_retry_loop(psu: &mut PowerSupply) -> io::Result<f32> {
    const MAX_ATTEMPTS: usize = 5;
    let mut attempt: usize = 0;
    loop {
        attempt += 1;
        match psu.input_power() {
            Ok(power) => {
                return Ok(power);
            }
            Err(e) => {
                if attempt > MAX_ATTEMPTS {
                    return Err(e);
                }
                // this seems to un-stick the PSU
                psu.pc_uptime().ok();
                psu.uptime().ok();
                psu.name().ok();
                log::warn!("Failed to sample PSU attempt {attempt}/{MAX_ATTEMPTS}: {e}");
            }
        }
    }
}

fn sample_loop(
    psu: &mut PowerSupply,
    mqtt: &mut SslStream<TcpStream>,
    topic: &str,
) -> anyhow::Result<()> {
    const SAMPLE_RATE: Duration = Duration::from_secs(1);
    const SAMPLES_PER_PUBLISH: usize = 15;

    loop {
        let mut samples: Vec<f32> = Vec::with_capacity(SAMPLES_PER_PUBLISH);
        for _ in 0..SAMPLES_PER_PUBLISH {
            let sample: f32 = sample_retry_loop(psu).context("Failed to sample PSU")?;

            samples.push(sample);
            sleep(SAMPLE_RATE);
        }

        let sum: f32 = samples.iter().sum::<f32>();
        let mean: f32 = sum / (SAMPLES_PER_PUBLISH as f32);

        mqtt::publish(mqtt, topic, &format!("{mean:.0}")).context("Failed to publish")?;
    }
}

fn main() -> anyhow::Result<()> {
    let config_file_path: OsString = match std::env::args_os().nth(1) {
        Some(x) => x,
        None => {
            eprintln!(
                "usage: {} [config-file.json]",
                std::env::args_os()
                    .next()
                    .unwrap_or_else(|| OsString::from("???"))
                    .to_string_lossy()
            );
            std::process::exit(1);
        }
    };

    systemd_journal_logger::JournalLog::default()
        .install()
        .context("Failed to initialize logging")?;
    log::set_max_level(log::LevelFilter::Trace);

    ctrlc::set_handler(|| std::process::exit(0)).context("Failed to set SIGINT handler")?;

    log::info!("Hello world");

    let file: File = File::open(&config_file_path).with_context(|| {
        format!(
            "Failed to open config file {}",
            config_file_path.to_string_lossy()
        )
    })?;
    let reader: BufReader<File> = BufReader::new(file);
    let config: Config =
        serde_json::from_reader(reader).context("Failed to deserialize config file")?;

    let psks: HashMap<String, Vec<u8>> = psk::load(&config.psk_file_path)?;

    let mut psu: PowerSupply = psu_connect()?;
    let mut mqtt: SslStream<TcpStream> = mqtt_connect(config.ip, config.port, psks)?;
    sample_loop(&mut psu, &mut mqtt, &config.topic)?;
    Ok(())
}
