use psm_common::netcode::Axis;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub preset: TabletPreset,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TabletPreset {
    /// Status.
    pub status: u32,
    /// Returns the default context packet report rate, in Hertz.
    pub packet_rate: u32,
    /// Returns whether the packet data items will be returned in absolute or relative mode.
    pub packet_mode: u32,
    /// Returns which packet data items can generate motion events in the context.
    pub move_mask: u32,
    /// Origin of the context's input area in the tablet's native coordinates. (X)
    pub in_org_x: i32,
    /// Origin of the context's input area in the tablet's native coordinates. (Y)
    pub in_org_y: i32,
    /// Origin of the context's input area in the tablet's native coordinates. (Z)
    pub in_org_z: i32,
    /// Extent of the context's input area in the tablet's native coordinates. (X)
    pub in_ext_x: i32,
    /// Extent of the context's input area in the tablet's native coordinates. (Y)
    pub in_ext_y: i32,
    /// Extent of the context's input area in the tablet's native coordinates. (Z)
    pub in_ext_z: i32,
    /// Origin of the context's output coordinate space in context output coordinates. (X)
    pub out_org_x: i32,
    /// Origin of the context's output coordinate space in context output coordinates. (Y)
    pub out_org_y: i32,
    /// Origin of the context's output coordinate space in context output coordinates. (Z)
    pub out_org_z: i32,
    /// Extent of the context's output coordinate space in context output coordinates. (X)
    pub out_ext_x: i32,
    /// Extent of the context's output coordinate space in context output coordinates. (Y)
    pub out_ext_y: i32,
    /// Extent of the context's output coordinate space in context output coordinates. (Z)
    pub out_ext_z: i32,
    /// Returns the current screen display origin in pixels. Typically at 0. (X)
    pub sys_org_x: i32,
    /// Returns the current screen display origin in pixels. Typically at 0. (Y)
    pub sys_org_y: i32,
    /// Returns the current screen display size in pixels. (X)
    pub sys_ext_x: i32,
    /// Returns the current screen display size in pixels. (Y)
    pub sys_ext_y: i32,
    /// Returns flags indicating hardware and driver capabilities, as defined below:
    /// HWC_INTEGRATED: Indicates that the display and digitizer share the same surface.
    /// HWC_TOUCH: Indicates that the cursor must be in physical contact with the device to report position.
    /// HWC_HARDPROX: Indicates that device can generate events when the cursor is entering and leaving the physical detection range.
    /// HWC_PHYSID_CURSORS: Indicates that device can uniquely identify the active cursor in hardware.
    pub hardware: u32,
    /// Size of tablet context margins in tablet native coordinates. You probably want it at 0. (X)
    pub x_margin: i32,
    /// Size of tablet context margins in tablet native coordinates. You probably want it at 0. (Y)
    pub y_margin: i32,
    /// Size of tablet context margins in tablet native coordinates. You probably want it at 0. (Z)
    pub z_margin: i32,
    /// Tablet's range and resolution capabilities. (X)
    pub device_x: Axis,
    /// Tablet's range and resolution capabilities. (Y)
    pub device_y: Axis,
    /// Tablet's range and resolution capabilities. (Z)
    pub device_z: Axis,
    /// Tablet's range and resolution capabilities for the normal pressure input.
    pub normal_pressure: Axis,
    /// Tablet's range and resolution capabilities for the tangential pressure input.
    pub tangential_pressure: Axis,
    /// 3-element array describing the tablet's orientation range and resolution capabilities.
    pub orientation: [Axis; 3],
    /// 3-element array describing the tablet's rotation range and resolution capabilities.
    pub rotation: [Axis; 3],
}
