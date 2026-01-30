//! Slimproto protocol implementation
//! Ported from Slim/Networking/Slimproto.pm
//!
//! Protocol structure: [4-byte opcode][4-byte length][payload]

pub mod codec;
pub mod discovery;
pub mod messages;
pub mod server;

pub use codec::SlimprotoCodec;
pub use discovery::{DiscoveryServer, DiscoveryRequest, DiscoveryResponse, TlvEntry};
pub use messages::{SlimprotoMessage, HeloMessage, StatMessage, IrMessage, ButtonMessage, StreamCommand, AudioGainCommand};
pub use server::SlimprotoServer;

/// Slimproto port constant
pub const SLIMPROTO_PORT: u16 = 3483;

/// Device IDs (from Slimproto.pm line 28)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceId {
    Squeezebox = 2,
    SoftSqueeze = 3,
    Squeezebox2 = 4,
    Transporter = 5,
    SoftSqueeze3 = 6,
    Receiver = 7,
    SqueezeSlave = 8,
    Controller = 9,
    Boom = 10,
    SoftBoom = 11,
    SqueezePlay = 12,
}

impl DeviceId {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            2 => Some(Self::Squeezebox),
            3 => Some(Self::SoftSqueeze),
            4 => Some(Self::Squeezebox2),
            5 => Some(Self::Transporter),
            6 => Some(Self::SoftSqueeze3),
            7 => Some(Self::Receiver),
            8 => Some(Self::SqueezeSlave),
            9 => Some(Self::Controller),
            10 => Some(Self::Boom),
            11 => Some(Self::SoftBoom),
            12 => Some(Self::SqueezePlay),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Squeezebox => "squeezebox",
            Self::SoftSqueeze => "softsqueeze",
            Self::Squeezebox2 => "squeezebox2",
            Self::Transporter => "transporter",
            Self::SoftSqueeze3 => "softsqueeze3",
            Self::Receiver => "receiver",
            Self::SqueezeSlave => "squeezeslave",
            Self::Controller => "controller",
            Self::Boom => "boom",
            Self::SoftBoom => "softboom",
            Self::SqueezePlay => "squeezeplay",
        }
    }
}
