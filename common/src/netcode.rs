use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PSMPacketC2S {
    /// First packet that the client must send to the server
    Hi {
        /// Display name and version of the client
        name: String,
    },
    /// Tablet movement!
    TabletEvent {
        status: u32,
        buttons: u32,
        x: u32,
        y: u32,
        z: u32,
        normal_pressure: u32,
        tangential_pressure: u32,
    },
    /// Is stylus in proximity?
    Proximity {
        value: bool,
    },
    /// Set context options
    ConfigureContext {
        /// Returns the status.
        status: u32,
        /// Returns the default context packet report rate, in Hertz.
        packet_rate: u32,
        /// Returns whether the packet data items will be returned in absolute or relative mode.
        packet_mode: u32,
        /// Returns which packet data items can generate motion events in the context.
        move_mask: u32,
        /// Origin of the context's input area in the tablet's native coordinates. (X)
        in_org_x: i32,
        /// Origin of the context's input area in the tablet's native coordinates. (Y)
        in_org_y: i32,
        /// Origin of the context's input area in the tablet's native coordinates. (Z)
        in_org_z: i32,
        /// Extent of the context's input area in the tablet's native coordinates. (X)
        in_ext_x: i32,
        /// Extent of the context's input area in the tablet's native coordinates. (Y)
        in_ext_y: i32,
        /// Extent of the context's input area in the tablet's native coordinates. (Z)
        in_ext_z: i32,
        /// Origin of the context's output coordinate space in context output coordinates. (X)
        out_org_x: i32,
        /// Origin of the context's output coordinate space in context output coordinates. (Y)
        out_org_y: i32,
        /// Origin of the context's output coordinate space in context output coordinates. (Z)
        out_org_z: i32,
        /// Extent of the context's output coordinate space in context output coordinates. (X)
        out_ext_x: i32,
        /// Extent of the context's output coordinate space in context output coordinates. (Y)
        out_ext_y: i32,
        /// Extent of the context's output coordinate space in context output coordinates. (Z)
        out_ext_z: i32,
        /// Returns the current screen display origin in pixels. Typically at 0. (X)
        sys_org_x: i32,
        /// Returns the current screen display origin in pixels. Typically at 0. (Y)
        sys_org_y: i32,
        /// Returns the current screen display size in pixels. (X)
        sys_ext_x: i32,
        /// Returns the current screen display size in pixels. (Y)
        sys_ext_y: i32,
    },
    /// Set device options
    ConfigureDevice {
        /// Returns flags indicating hardware and driver capabilities, as defined below:
        /// HWC_INTEGRATED: Indicates that the display and digitizer share the same surface.
        /// HWC_TOUCH: Indicates that the cursor must be in physical contact with the device to report position.
        /// HWC_HARDPROX: Indicates that device can generate events when the cursor is entering and leaving the physical detection range.
        /// HWC_PHYSID_CURSORS: Indicates that device can uniquely identify the active cursor in hardware.
        hardware: u32,
        /// Returns the maximum packet report rate in Hertz.
        packet_rate: u32,
        /// (WTPKT) Returns a bit mask indicating which packet data items are physically relative
        /// (i.e. items for which the hardware can only report change, not absolute measurement).
        packet_mode: u32,
        /// Size of tablet context margins in tablet native coordinates. You probably want it at 0. (X)
        x_margin: i32,
        /// Size of tablet context margins in tablet native coordinates. You probably want it at 0. (Y)
        y_margin: i32,
        /// Size of tablet context margins in tablet native coordinates. You probably want it at 0. (Z)
        z_margin: i32,
        /// Tablet's range and resolution capabilities. (X)
        device_x: Axis,
        /// Tablet's range and resolution capabilities. (Y)
        device_y: Axis,
        /// Tablet's range and resolution capabilities. (Z)
        device_z: Axis,
        /// Tablet's range and resolution capabilities for the normal pressure input.
        normal_pressure: Axis,
        /// Tablet's range and resolution capabilities for the tangential pressure input.
        tangential_pressure: Axis,
        /// 3-element array describing the tablet's orientation range and resolution capabilities.
        orientation: [Axis; 3],
        /// 3-element array describing the tablet's rotation range and resolution capabilities.
        rotation: [Axis; 3],
    },
    Debug {
        msg: String,
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

#[derive(Debug, Serialize, Deserialize)]
/// The AXIS data structure defines the range and resolution for many of the packet data items.
pub struct Axis {
    /// Specifies the minimum value of the data item in the tablet's native coordinates.
    pub min: i32,
    /// Specifies the maximum value of the data item in the tablet's native coordinates.
    pub max: i32,
    /// Indicates the units used in calculating the resolution for the data item.
    /// TU_NONE, TU_INCHES, TU_CENTIMETERS, TU_CIRCLE
    pub units: u32,
    /// Is a fixed-point number giving the number of data item increments per physical unit.
    pub resolution: u32,
}
impl Default for Axis {
    fn default() -> Self {
        Axis {
            min: 0,
            max: 65535,
            units: 0,
            resolution: 0x03e8_0000, // 1000.0000
        }
    }
}
