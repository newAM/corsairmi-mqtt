use anyhow::Context;
use corsairmi::PowerSupply;
use static_assertions::const_assert;
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    thread::sleep,
    time::Duration,
};

/// MQTT control packet types.
///
/// See [MQTT Control Packet format](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901019).
#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
enum ControlPacket {
    /// Connection request.
    CONNECT = 1,
    /// Connect acknowledgment.
    CONNACK = 2,
    /// Publish message.
    PUBLISH = 3,
}

#[repr(u8)]
enum ConnectFlags {
    // UserName = 0b1000_0000,
    // Password = 0b0100_0000,
    // WillRetain = 0b0010_0000,
    // WillQos = 0b0001_1000,
    // WillFlag = 0b0000_0100,
    CleanStart = 0b0000_0010,
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
enum ConnectReasonCode {
    Success = 0x00,
    UnspecifiedError = 0x80,
    MalformedPacket = 0x81,
    ProtocolError = 0x82,
    ImplementationSpecificError = 0x83,
    UnsupportedProtocolVersion = 0x84,
    ClientIdentifierNotValid = 0x85,
    BadUsernameOrPassword = 0x86,
    NotAuthorized = 0x87,
    ServerUnavailable = 0x88,
    ServerBusy = 0x89,
    Banned = 0x8A,
    BadAuthenticationMethod = 0x8C,
    TopicNameInvalid = 0x90,
    PacketTooLarge = 0x95,
    QuotaExceeded = 0x97,
    PayloadFormatInvalid = 0x99,
    RetainNotSupported = 0x9A,
    QosNotSupported = 0x9B,
    UseAnotherServer = 0x9C,
    ServerMoved = 0x9D,
    ConnectionRateExceeded = 0x9F,
}

impl TryFrom<u8> for ConnectReasonCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == ConnectReasonCode::Success as u8 => Ok(ConnectReasonCode::Success),
            x if x == ConnectReasonCode::UnspecifiedError as u8 => {
                Ok(ConnectReasonCode::UnspecifiedError)
            }
            x if x == ConnectReasonCode::MalformedPacket as u8 => {
                Ok(ConnectReasonCode::MalformedPacket)
            }
            x if x == ConnectReasonCode::ProtocolError as u8 => {
                Ok(ConnectReasonCode::ProtocolError)
            }
            x if x == ConnectReasonCode::ImplementationSpecificError as u8 => {
                Ok(ConnectReasonCode::ImplementationSpecificError)
            }
            x if x == ConnectReasonCode::UnsupportedProtocolVersion as u8 => {
                Ok(ConnectReasonCode::UnsupportedProtocolVersion)
            }
            x if x == ConnectReasonCode::ClientIdentifierNotValid as u8 => {
                Ok(ConnectReasonCode::ClientIdentifierNotValid)
            }
            x if x == ConnectReasonCode::BadUsernameOrPassword as u8 => {
                Ok(ConnectReasonCode::BadUsernameOrPassword)
            }
            x if x == ConnectReasonCode::NotAuthorized as u8 => {
                Ok(ConnectReasonCode::NotAuthorized)
            }
            x if x == ConnectReasonCode::ServerUnavailable as u8 => {
                Ok(ConnectReasonCode::ServerUnavailable)
            }
            x if x == ConnectReasonCode::ServerBusy as u8 => Ok(ConnectReasonCode::ServerBusy),
            x if x == ConnectReasonCode::Banned as u8 => Ok(ConnectReasonCode::Banned),
            x if x == ConnectReasonCode::BadAuthenticationMethod as u8 => {
                Ok(ConnectReasonCode::BadAuthenticationMethod)
            }
            x if x == ConnectReasonCode::TopicNameInvalid as u8 => {
                Ok(ConnectReasonCode::TopicNameInvalid)
            }
            x if x == ConnectReasonCode::PacketTooLarge as u8 => {
                Ok(ConnectReasonCode::PacketTooLarge)
            }
            x if x == ConnectReasonCode::QuotaExceeded as u8 => {
                Ok(ConnectReasonCode::QuotaExceeded)
            }
            x if x == ConnectReasonCode::PayloadFormatInvalid as u8 => {
                Ok(ConnectReasonCode::PayloadFormatInvalid)
            }
            x if x == ConnectReasonCode::RetainNotSupported as u8 => {
                Ok(ConnectReasonCode::RetainNotSupported)
            }
            x if x == ConnectReasonCode::QosNotSupported as u8 => {
                Ok(ConnectReasonCode::QosNotSupported)
            }
            x if x == ConnectReasonCode::UseAnotherServer as u8 => {
                Ok(ConnectReasonCode::UseAnotherServer)
            }
            x if x == ConnectReasonCode::ServerMoved as u8 => Ok(ConnectReasonCode::ServerMoved),
            x if x == ConnectReasonCode::ConnectionRateExceeded as u8 => {
                Ok(ConnectReasonCode::ConnectionRateExceeded)
            }
            _ => Err(value),
        }
    }
}

const TOPIC: &str = "/home/5950x/psu/in_power";

const SERVER_IP: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 4);
const SERVER_PORT: u16 = 1883;

const PROTOCOL_NAME_LEN: u16 = 4;
const PROTOCOL_NAME: [u8; PROTOCOL_NAME_LEN as usize] = [b'M', b'Q', b'T', b'T'];
const PROTOCOL_NAME_LEN_BYTES: [u8; 2] = PROTOCOL_NAME_LEN.to_be_bytes();

const CLIENT_ID: &str = "5950xpsu";
const_assert!(CLIENT_ID.len() > 1);
const_assert!(CLIENT_ID.len() < 23);
const CLIENT_ID_LEN: u8 = CLIENT_ID.len() as u8;
const CLIENT_ID_LEN_BYTES: [u8; 2] = (CLIENT_ID_LEN as u16).to_be_bytes();

const KEEPALIVE: u16 = 60; // seconds
const KEEPALIVE_BYTES: [u8; 2] = KEEPALIVE.to_be_bytes();

const CONNECT_PACKET_LEN: u8 = 15 + CLIENT_ID_LEN;
const CONNECT_PACKET: [u8; CONNECT_PACKET_LEN as usize] = [
    (ControlPacket::CONNECT as u8) << 4,
    CONNECT_PACKET_LEN - 2,
    PROTOCOL_NAME_LEN_BYTES[0],
    PROTOCOL_NAME_LEN_BYTES[1],
    PROTOCOL_NAME[0],
    PROTOCOL_NAME[1],
    PROTOCOL_NAME[2],
    PROTOCOL_NAME[3],
    5, // protocol version 5
    ConnectFlags::CleanStart as u8,
    KEEPALIVE_BYTES[0], // keepalive
    KEEPALIVE_BYTES[1], // keepalive
    0,                  // properties
    CLIENT_ID_LEN_BYTES[0],
    CLIENT_ID_LEN_BYTES[1],
    CLIENT_ID.as_bytes()[0],
    CLIENT_ID.as_bytes()[1],
    CLIENT_ID.as_bytes()[2],
    CLIENT_ID.as_bytes()[3],
    CLIENT_ID.as_bytes()[4],
    CLIENT_ID.as_bytes()[5],
    CLIENT_ID.as_bytes()[6],
    CLIENT_ID.as_bytes()[7],
];

fn mqtt_connect() -> anyhow::Result<TcpStream> {
    log::debug!("Opening stream");
    let mut stream = TcpStream::connect_timeout(
        &SocketAddr::new(SERVER_IP.into(), SERVER_PORT),
        Duration::from_secs(2),
    )?;

    log::debug!("Writing connect");
    stream.write_all(&CONNECT_PACKET)?;

    log::debug!("Waiting for CONNACK");
    let mut connack: Vec<u8> = vec![0; 64];
    log::debug!("Reading CONNACK");
    let len: usize = stream.read(&mut connack)?;
    log::debug!("Read CONNACK len={len}");

    let byte0: &u8 = connack.get(0).context("failed to get CONNACK byte 0")?;
    if byte0 >> 4 != (ControlPacket::CONNACK as u8) {
        return Err(anyhow::anyhow!("Response is not CONNACK: {byte0}"));
    }

    const MIN_CONNACK_LEN: u8 = 4;
    let byte1: &u8 = connack.get(1).context("failed to get CONNACK byte 1")?;
    if byte1 < &MIN_CONNACK_LEN {
        return Err(anyhow::anyhow!(
            "CONNACK minimum length is {MIN_CONNACK_LEN} got {byte1}"
        ));
    }

    let byte3: &u8 = connack.get(3).context("failed to get CONNACK byte 3")?;
    match ConnectReasonCode::try_from(*byte3) {
        Ok(ConnectReasonCode::Success) => {
            log::info!("Sucessfully connected to MQTT server");
            Ok(stream)
        }
        x => Err(anyhow::anyhow!("Server did not accept connection: {x:?}")),
    }
}

fn psu_connect() -> anyhow::Result<PowerSupply> {
    Ok(PowerSupply::open(
        corsairmi::list()?.first().context("No PSU found")?,
    )?)
}

fn mqtt_publish(stream: &mut TcpStream, topic: &str, payload: &str) -> io::Result<()> {
    const PROPERTY_LEN: usize = 2;
    let packet_len: usize = topic.len() + payload.len() + PROPERTY_LEN + 1 + 2;
    debug_assert!(packet_len < usize::from(u8::MAX));
    let mut buf: Vec<u8> = Vec::with_capacity(2 + packet_len);
    buf.push((ControlPacket::PUBLISH as u8) << 4);
    buf.push((packet_len) as u8);
    buf.extend_from_slice(&(topic.len() as u16).to_be_bytes());
    buf.extend_from_slice(topic.as_bytes());
    buf.push(PROPERTY_LEN as u8); // property length
    buf.push(0x01); // payload format
    buf.push(0x01); // payload format: utf-8
    buf.extend_from_slice(payload.as_bytes());

    log::trace!("PUBLISH: {topic} {payload}");
    stream.write_all(&buf)?;
    Ok(())
}

fn connect_loop() -> (PowerSupply, TcpStream) {
    const MAX_SLEEP: Duration = Duration::from_secs(3600);
    let mut sleep_time: Duration = Duration::from_secs(5);
    loop {
        let psu: PowerSupply = match psu_connect() {
            Err(e) => {
                log::error!("Failed to connect to PSU: {e}");
                if sleep_time < MAX_SLEEP {
                    sleep_time *= 2;
                }
                log::info!("Sleeping for {sleep_time:?} before retrying");
                sleep(sleep_time);
                continue;
            }
            Ok(psu) => psu,
        };

        sleep_time = Duration::from_secs(5);
        let mqtt: TcpStream = match mqtt_connect() {
            Err(e) => {
                log::error!("Failed to connect to MQTT server: {e}");
                if sleep_time < MAX_SLEEP {
                    sleep_time *= 2;
                }
                log::info!("Sleeping for {sleep_time:?} before retrying");
                sleep(sleep_time);
                continue;
            }
            Ok(mqtt) => mqtt,
        };

        break (psu, mqtt);
    }
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

fn sample_loop(psu: &mut PowerSupply, mqtt: &mut TcpStream) {
    const SAMPLE_RATE: Duration = Duration::from_secs(1);
    const SAMPLES_PER_PUBLISH: usize = 15;

    loop {
        let mut samples: Vec<f32> = Vec::with_capacity(SAMPLES_PER_PUBLISH);
        for _ in 0..SAMPLES_PER_PUBLISH {
            let sample: f32 = match sample_retry_loop(psu) {
                Ok(power) => power,
                Err(e) => {
                    log::error!("Failed to sample PSU: {e}");
                    return;
                }
            };

            samples.push(sample);
            sleep(SAMPLE_RATE);
        }

        let sum: f32 = samples.iter().sum::<f32>();
        let mean: f32 = sum / (SAMPLES_PER_PUBLISH as f32);

        if let Err(e) = mqtt_publish(mqtt, TOPIC, &format!("{mean:.0}")) {
            log::error!("Failed to publish: {e}");
            return;
        }
    }
}

fn main() -> anyhow::Result<()> {
    systemd_journal_logger::init().context("failed to initialize logging")?;
    log::set_max_level(log::LevelFilter::Trace);

    log::info!("Hello world");

    ctrlc::set_handler(move || {
        let mut mqtt = mqtt_connect().unwrap();
        mqtt_publish(&mut mqtt, TOPIC, "0.0").unwrap();
        std::process::exit(0);
    })
    .context("failed to set SIGINT handler")?;

    const MAX_SLEEP: Duration = Duration::from_secs(300);
    let mut sleep_time: Duration = Duration::from_millis(250);
    loop {
        log::info!("Connect loop");
        let (mut psu, mut mqtt) = connect_loop();
        log::info!("Sleeping for {sleep_time:?}");
        sleep(sleep_time);
        log::info!("Sample loop");
        sample_loop(&mut psu, &mut mqtt);
        drop(psu);
        drop(mqtt);
        if sleep_time < MAX_SLEEP {
            sleep_time *= 2;
        }
    }
}
