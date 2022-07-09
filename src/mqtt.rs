use openssl::ssl::SslStream;
use std::{
    io::{self, Write},
    net::TcpStream,
};

pub fn connect(client_id: &str) -> Vec<u8> {
    let client_id_len: u8 = client_id.len().try_into().expect("Client ID is too long");
    let expected_len: usize = 15_usize.saturating_add(client_id.len());
    let mut pkt: Vec<u8> = Vec::with_capacity(expected_len);

    pkt.push((ControlPacket::CONNECT as u8) << 4);
    pkt.push(expected_len.saturating_sub(2).try_into().unwrap());

    const PROTOCOL_NAME_LEN: u16 = 4;
    const PROTOCOL_NAME: [u8; PROTOCOL_NAME_LEN as usize] = *b"MQTT";
    const PROTOCOL_NAME_LEN_BYTES: [u8; 2] = PROTOCOL_NAME_LEN.to_be_bytes();
    pkt.extend_from_slice(&PROTOCOL_NAME_LEN_BYTES);
    pkt.extend_from_slice(&PROTOCOL_NAME);
    pkt.push(5);
    pkt.push(ConnectFlags::CleanStart as u8);

    const KEEPALIVE: u16 = 60; // seconds
    const KEEPALIVE_BYTES: [u8; 2] = KEEPALIVE.to_be_bytes();
    pkt.extend_from_slice(&KEEPALIVE_BYTES);

    // properties length
    pkt.push(0);

    pkt.extend_from_slice(u16::from(client_id_len).to_be_bytes().as_ref());
    pkt.extend_from_slice(client_id.as_bytes());

    assert_eq!(expected_len, pkt.len());

    pkt
}

/// MQTT control packet types.
///
/// See [MQTT Control Packet format](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901019).
#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
pub enum ControlPacket {
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
pub enum ConnectReasonCode {
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

pub fn publish(stream: &mut SslStream<TcpStream>, topic: &str, payload: &str) -> io::Result<()> {
    const PROPERTY_LEN: usize = 2;
    let packet_len: usize = topic.len() + payload.len() + PROPERTY_LEN + 1 + 2;
    assert!(packet_len < usize::from(u8::MAX));
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

#[cfg(test)]
mod tests {
    use super::connect;

    #[test]
    fn connect_smoke_test() {
        connect("client_id");
    }
}
