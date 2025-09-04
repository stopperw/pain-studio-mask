use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PSMPacketC2S {
    /// First packet client must send to the server
    Hi {
        /// Display name and version of the client
        name: String,
    },
    /// deprecated?
    RawTabletPacket {
        status: u32,
        buttons: u32,
        x: u32,
        y: u32,
        z: u32,
        normal_pressure: u32,
        tangential_pressure: u32
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PSMPacketS2C {
    /// Server's response to [PSMPacketC2S::Hi]
    Hi {
        /// Server compatible version
        compatible: u32,
    },
}
        // let packet = Packet {
        //     context: self.handle as u32,
        //     status: 0,
        //     time: self.serial as u32,
        //     changed: 0xFFFFFFFF,
        //     serial: self.serial as u32,
        //     cursor: 0,
        //     buttons: 0b11111111_11111111_11111111_11111111,
        //     x: 960,
        //     y: 540,
        //     z: 0,
        //     normal_pressure: 65535,
        //     tangential_pressure: 65535,
        //     orientation: Orientation::default(),
        //     rotation: Rotation::default(),
        // };

