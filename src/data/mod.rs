mod http;

use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub enum StreamDirection {
    ClientToServer,
    ServerToClient,
}

impl Display for StreamDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamDirection::ClientToServer => f.write_str("ClientToServer"),
            StreamDirection::ServerToClient => f.write_str("ServerToClient"),
        }
    }
}

pub struct ProxyData {
    direction: StreamDirection,
    buffer: [u8; 4096],
    len: usize,
}

impl ProxyData {
    pub fn new(direction: StreamDirection, buffer: [u8; 4096], len: usize) -> Self {
        Self { direction, buffer, len }
    }

    pub fn direction(&self) -> &StreamDirection {
        &self.direction
    }

    pub fn buffer(&self) -> [u8; 4096] {
        self.buffer
    }

    pub fn len(&self) -> usize {
        self.len
    }
}


