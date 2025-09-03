use std::{collections::{HashMap, VecDeque}, ffi::c_void, sync::{LazyLock, Mutex}};

use color_eyre::eyre::bail;
use log::{debug, error, info};
use static_init::constructor;
use windows::{
    core::*,
    Win32::{Foundation::{HWND, LPARAM, WPARAM}, UI::WindowsAndMessaging::*},
};

/// Returns [WtiInterface]
pub const WTI_INTERFACE: u32 = 1;
pub const WTI_STATUS: u32 = 2;
/// Returns [WtiLogicalContext]
pub const WTI_DEFCONTEXT: u32 = 3;
/// Returns [WtiLogicalContext]
pub const WTI_DEFSYSCTX: u32 = 4;
pub const WTI_DEVICES: u32 = 100;
pub const WTI_CURSORS: u32 = 200;
pub const WTI_EXTENSIONS: u32 = 300;
/// Returns [WtiLogicalContext]
pub const WTI_DDCTXS: u32 = 400;
/// Returns [WtiLogicalContext]
pub const WTI_DSCTXS: u32 = 500;

pub const WT_DEFBASE: u32 = 0x7FF0;

// WTPKT bits
/// Reporting context
pub const PK_CONTEXT: u32 = 0x0001;
/// Status bits
pub const PK_STATUS: u32 = 0x0002;
/// Timestamp
pub const PK_TIME: u32 = 0x0004;
/// Change bit vector
pub const PK_CHANGED: u32 = 0x0008;
/// Packet serial number
pub const PK_SERIAL_NUMBER: u32 = 0x0010;
/// Reporting cursor
pub const PK_CURSOR: u32 = 0x0020;
/// Button information
pub const PK_BUTTONS: u32 = 0x0040;
/// X axis
pub const PK_X: u32 = 0x0080;
/// Y axis
pub const PK_Y: u32 = 0x0100;
/// Z axis
pub const PK_Z: u32 = 0x0200;
/// Normal (tip) pressure
pub const PK_NORMAL_PRESSURE: u32 = 0x0400;
/// Tangential (barrel) pressure
pub const PK_TANGENT_PRESSURE: u32 = 0x0800;
/// Orientation info (tilt)
pub const PK_ORIENTATION: u32 = 0x1000;
/// Rotation info (added in spec 1.1)
pub const PK_ROTATION: u32 = 0x2000;

// Context option values
pub const CXO_SYSTEM: u32 = 0x0001;
pub const CXO_PEN: u32 = 0x0002;
pub const CXO_MESSAGES: u32 = 0x0004;
pub const CXO_MARGIN: u32 = 0x8000;
pub const CXO_MGNINSIDE: u32 = 0x4000;
pub const CXO_CSRMESSAGES: u32 = 0x0008;

// Context status values
pub const CXS_DISABLED: u32 = 0x0001;
pub const CXS_OBSCURED: u32 = 0x0002;
pub const CXS_ONTOP: u32 = 0x0004;

// Context lock values
pub const CXL_INSIZE: u32 = 0x0001;
pub const CXL_INASPECT: u32 = 0x0002;
pub const CXL_SENSITIVITY: u32 = 0x0004;
pub const CXL_MARGIN: u32 = 0x0008;
pub const CXL_SYSOUT: u32 = 0x0010;

pub const INTERFACE_WINTABID_LEN: usize = 34;
#[repr(C)]
pub struct WtiInterface {
    /// Returns a copy of the null-terminated tablet hardware identification string in the user buffer.
    /// This string should include make, model, and revision information in user-readable format.
    pub wintabid: [u8; INTERFACE_WINTABID_LEN],
    /// Returns the specification version number.
    /// The high-order byte contains the major version number; the low-order byte contains the minor version number.
    pub spec_version: u16,
    /// Returns the implementation version number.
    /// The high-order byte contains the major version number; the low-order byte contains the minor version number.
    pub impl_version: u16,
    /// Returns the number of devices supported.
    pub num_devices: u32,
    /// Returns the total number of cursor types supported.
    pub num_cursors: u32,
    /// Returns the number of contexts supported.
    pub num_contexts: u32,
    /// Returns flags indicating which context options are supported.
    pub ctx_options: u32,
    /// Returns the size of the save information returned from WTSave.
    pub ctx_save_size: u32,
    /// Returns the number of extension data items supported.
    pub num_extensions: u32,
    /// Returns the number of manager handles supported.
    pub num_managers: u32,
}
impl WtiInterface {
    pub fn psm_default() -> Self {
        let wintabids = "PAIN STUDIO MASK".encode_utf16().collect::<Vec<u16>>();
        let mut wintabid = [0u8; INTERFACE_WINTABID_LEN];
        for i in 0..INTERFACE_WINTABID_LEN {
            if i % 2 == 1 {
                continue;
            }
            let u16i = i / 2;
            if wintabids.len() <= u16i {
                break;
            }
            wintabid[i] = wintabids[u16i] as u8;
            wintabid[i + 1] = (wintabids[u16i] << 8) as u8;
        }
        // println!("{:#?}", wintabid);

        WtiInterface {
            wintabid,
            spec_version: 0b00000001_00000001,
            impl_version: 0b00000000_00000001,
            num_devices: 1,
            num_cursors: 1,
            num_contexts: 1,
            ctx_options: 0,
            ctx_save_size: 0,
            num_extensions: 1,
            num_managers: 1,
        }
    }

    pub fn handle_info(&self, index: u32, lp_output: *mut c_void) -> u32 {
        match index {
            0 => info_write(self, lp_output),
            1 => info_write_array(&self.wintabid, lp_output, INTERFACE_WINTABID_LEN),
            // 1 => info_write(&self.wintabid, lp_output),
            2 => info_write(&self.spec_version, lp_output),
            3 => info_write(&self.impl_version, lp_output),
            4 => info_write(&self.num_devices, lp_output),
            5 => info_write(&self.num_cursors, lp_output),
            6 => info_write(&self.num_contexts, lp_output),
            7 => info_write(&self.ctx_options, lp_output),
            8 => info_write(&self.ctx_save_size, lp_output),
            9 => info_write(&self.num_extensions, lp_output),
            10 => info_write(&self.num_managers, lp_output),
            _ => 0
        }
    }
}

const HWC_INTEGRATED: u32 = 0x0001;
const HWC_TOUCH: u32 = 0x0002;
const HWC_HARDPROX: u32 = 0x0004;
const HWC_PHYSID_CURSORS: u32 = 0x0008;

pub const DEVICES_NAME_LEN: usize = 48;
#[repr(C)]
pub struct WtiDevices {
    /// Returns a displayable null-terminated string describing the device, manufacturer, and revision level.
    pub name: [u8; DEVICES_NAME_LEN],
    /// Returns flags indicating hardware and driver capabilities, as defined below:
	/// [HWC_INTEGRATED]: Indicates that the display and digitizer share the same surface.
    /// [HWC_TOUCH]: Indicates that the cursor must be in physical contact with the device to report position.
	/// [HWC_HARDPROX]: Indicates that device can generate events when the cursor is entering and leaving the physical detection range.
	/// [HWC_PHYSID_CURSORS]: Indicates that device can uniquely identify the active cursor in hardware.
    pub hardware: u32,
    /// Returns the number of supported cursor types.
    pub num_cursor_types: u32,
    /// Returns the first cursor type number for the device.
    pub first_cursor_type: u32,
    /// Returns the maximum packet report rate in Hertz.
    pub packet_rate: u32,
    /// (WTPKT) Returns a bit mask indicating which packet data items are always available.
    pub packet_data: u32,
    /// (WTPKT) Returns a bit mask indicating which packet data items are physically relative
    /// (i.e. items for which the hardware can only report change, not absolute measurement).
    pub packet_mode: u32,
    /// (WTPKT) Returns a bit mask indicating which packet data items are only available when certain cursors are connected.
    /// The individual cursor descriptions must be consulted to determine which cursors return which data.
    pub csr_data: u32,
    /// Size of tablet context margins in tablet native coordinates. (X)
    pub x_margin: i32,
    /// Size of tablet context margins in tablet native coordinates. (Y)
    pub y_margin: i32,
    /// Size of tablet context margins in tablet native coordinates. (Z)
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
    /// Null-terminated string containing the device’s Plug and Play ID.
    pub pnp_id: [u8; 8],
}
impl WtiDevices {
    pub fn psm_default() -> Self {
        let wintabids = "PAIN STUDIO MASK".encode_utf16().collect::<Vec<u16>>();
        let mut wintabid = [0u8; DEVICES_NAME_LEN];
        for i in 0..DEVICES_NAME_LEN {
            if i % 2 == 1 {
                continue;
            }
            let u16i = i / 2;
            if wintabids.len() <= u16i {
                break;
            }
            wintabid[i] = wintabids[u16i] as u8;
            wintabid[i + 1] = (wintabids[u16i] << 8) as u8;
        }
        // println!("{:#?}", wintabid);

        WtiDevices {
            name: wintabid,
            hardware: 0,
            num_cursor_types: 0,
            first_cursor_type: 0,
            packet_rate: 100,
            packet_data: 0,
            packet_mode: 0,
            csr_data: 0,
            x_margin: 0,
            y_margin: 0,
            z_margin: 0,
            device_x: Axis::psm_default(),
            device_y: Axis::psm_default(),
            device_z: Axis::psm_default(),
            normal_pressure: Axis::psm_default(),
            tangential_pressure: Axis::psm_default(),
            orientation: [Axis::psm_default(), Axis::psm_default(), Axis::psm_default()],
            rotation: [Axis::psm_default(), Axis::psm_default(), Axis::psm_default()],
            pnp_id: [0u8; 8],
        }
    }

    pub fn handle_info(&self, index: u32, lp_output: *mut c_void) -> u32 {
        match index {
            0 => info_write(self, lp_output),
            1 => info_write_array(&self.name, lp_output, DEVICES_NAME_LEN),
            2 => info_write(&self.hardware, lp_output),
            3 => info_write(&self.num_cursor_types, lp_output),
            4 => info_write(&self.first_cursor_type, lp_output),
            5 => info_write(&self.packet_rate, lp_output),
            6 => info_write(&self.packet_data, lp_output),
            7 => info_write(&self.packet_mode, lp_output),
            8 => info_write(&self.csr_data, lp_output),
            9 => info_write(&self.x_margin, lp_output),
            10 => info_write(&self.y_margin, lp_output),
            11 => info_write(&self.z_margin, lp_output),
            12 => info_write(&self.device_x, lp_output),
            13 => info_write(&self.device_y, lp_output),
            14 => 0, //info_write(&self.device_z, lp_output),
            15 => info_write(&self.normal_pressure, lp_output),
            16 => 0, //info_write(&self.tangential_pressure, lp_output),
            17 => info_write(&self.orientation, lp_output),
            18 => info_write(&self.rotation, lp_output),
            19 => info_write_array(&self.pnp_id, lp_output, 8),
            _ => 0
        }
    }
}

pub const CRC_MULTIMODE: u32 = 0x0001;
pub const CRC_AGGREGATE: u32 = 0x0002;
pub const CRC_INVERT: u32 = 0x0004;

pub const CURSORS_NAME_LEN: usize = 48;
#[repr(C)]
pub struct WtiCursors {
    /// Returns a displayable zero-terminated string containing the name of the cursor.
    pub name: [u8; CURSORS_NAME_LEN],
    /// Returns whether the cursor is currently connected.
    /// BUT ITS 4 BYTES???
    pub active: u32,
    /// (WTPKT) Returns a bit mask indicating the packet data items supported when this cursor is connected.
    pub packet_data: u32,
    /// Returns the number of buttons on this cursor.
    pub buttons: u8,
    /// Returns the number of bits of raw button data returned by the hardware.
    pub button_bits: u8,
    /// Returns a list of zero-terminated strings containing the names of the cursor's buttons.
    /// The number of names in the list is the same as the number of buttons on the cursor.
    /// The names are separated by a single zero character; the list is terminated by two zero characters.
    /// Replaced by a single u8 to zero it out.
    pub button_names: u8,
    /// Returns a 32 byte array of logical button numbers, one for each physical button.
    pub button_map: [u8; 32],
    /// Returns a 32 byte array of button action codes, one for each logical button.
    pub system_button_map: [u8; 32],
    /// Returns the physical button number of the button that is controlled by normal pressure.
    pub physical_button: u8,
    /// Returns an array of two UINTs, specifying the button marks for the normal pressure button.
    /// The first UINT contains the release mark; the second contains the press mark.
    pub npbtnmarks: [u32; 2],
    /// Returns an array of UINTs describing the pressure response curve for normal pressure.
    pub npresponse: [u32; 2],
    /// Returns the physical button number of the button that is controlled by tangential pressure.
    pub tangential_button: u8,
    /// Returns an array of two UINTs, specifying the button marks for the tangential pressure button.
    /// The first UINT contains the release mark; the second contains the press mark.
    pub tpbtnmarks: [u32; 2],
    /// Returns an array of UINTs describing the pressure response curve for tangential pressure.
    pub tpresponse: [u32; 2],
    /// Returns a manufacturer-specific physical identifier for the cursor.
    /// This value will distinguish the physical cursor from others on the same device.
    /// This physical identifier allows applications to bind functions to specific physical cursors,
    /// even if category numbers change and multiple, otherwise identical, physical cursors are present.
    pub physical_id: u32,
    /// Returns the cursor mode number of this cursor type, if this cursor type has the CRC_MULTIMODE capability.
    pub csr_mode: u32,
    /// Returns the minimum set of data available from a physical cursor in this cursor type, if this cursor type has the CRC_AGGREGATE capability.
    pub minpktdata: u32,
    /// Returns the minimum number of buttons of physical cursors in the cursor type, if this cursor type has the CRC_AGGREGATE capability.
    pub min_buttons: u32,
    /// Returns flags indicating cursor capabilities, as defined by the values and their meanings, below:
    /// [CRC_MULTIMODE]: Indicates this cursor type describes one of several modes of a single physical cursor. Consecutive cursor type categories describe the modes; the CSR_MODE data item gives the mode number of each cursor type.
	/// [CRC_AGGREGATE]: Indicates this cursor type describes several physical cursors that cannot be distinguished by software.
	/// [CRC_INVERT]: Indicates this cursor type describes the physical cursor in its inverted orientation; the previous consecutive cursor type category describes the normal orientation.
    pub capabilities: u32,
}
impl WtiCursors {
    pub fn psm_default() -> Self {
        let wintabids = "PAIN STUDIO MASK".encode_utf16().collect::<Vec<u16>>();
        let mut wintabid = [0u8; CURSORS_NAME_LEN];
        for i in 0..CURSORS_NAME_LEN {
            if i % 2 == 1 {
                continue;
            }
            let u16i = i / 2;
            if wintabids.len() <= u16i {
                break;
            }
            wintabid[i] = wintabids[u16i] as u8;
            wintabid[i + 1] = (wintabids[u16i] << 8) as u8;
        }
        // println!("{:#?}", wintabid);

        WtiCursors {
            name: wintabid,
            active: 0xFFFFFFFF,
            packet_data: 0,
            buttons: 1,
            button_bits: 1,
            button_names: 0,
            button_map: [0u8; 32],
            system_button_map: [0u8; 32],
            physical_button: 0,
            npbtnmarks: [0, 0],
            npresponse: [0, 0],
            tangential_button: 0,
            tpbtnmarks: [0, 0],
            tpresponse: [0, 0],
            physical_id: 69420,
            csr_mode: 0,
            minpktdata: 0,
            min_buttons: 0,
            capabilities: 0,
        }
    }

    pub fn handle_info(&self, index: u32, lp_output: *mut c_void) -> u32 {
        match index {
            0 => info_write(self, lp_output),
            1 => info_write_array(&self.name, lp_output, DEVICES_NAME_LEN),
            2 => info_write(&self.active, lp_output),
            3 => info_write(&self.packet_data, lp_output),
            4 => info_write(&self.buttons, lp_output),
            5 => info_write(&self.button_bits, lp_output),
            6 => info_write(&self.button_names, lp_output),
            7 => info_write(&self.button_map, lp_output),
            8 => info_write(&self.system_button_map, lp_output),
            9 => info_write(&self.physical_button, lp_output),
            10 => info_write(&self.npbtnmarks, lp_output),
            11 => 0, //info_write(&self.npresponse, lp_output),
            12 => info_write(&self.tangential_button, lp_output),
            13 => info_write(&self.tpbtnmarks, lp_output),
            14 => 0, //info_write(&self.tpresponse, lp_output),
            15 => info_write(&self.physical_id, lp_output),
            16 => info_write(&self.csr_mode, lp_output),
            17 => info_write(&self.minpktdata, lp_output),
            18 => info_write(&self.min_buttons, lp_output),
            19 => info_write(&self.capabilities, lp_output),
            _ => 0
        }
    }
}

pub const LOGICAL_CONTEXT_NAMELEN: usize = 80;
#[derive(Debug)]
#[repr(C, align(4))]
pub struct WtiLogicalContext {
    /// Returns a 40 character array containing the default name.
    /// The name may occupy zero to 39 characters; the remainder of the array is padded with zeroes.
    pub name: [u8; LOGICAL_CONTEXT_NAMELEN],
    /// Returns option flags.
    /// For the default digitizing context, CXO_MARGIN and CXO_MGNINSIDE are allowed.
    /// For the default system context, CXO_SYSTEM is required; CXO_PEN, CXO_MARGIN, and CXO_MGNINSIDE are allowed.
    pub options: u32,
    /// Returns zero.
    pub status: u32,
    /// Returns which attributes of the default context are locked.
    pub locks: u32,
    /// Returns the value [WT_DEFBASE].
    pub msg_base: u32,
    /// Returns the default device. If this value is -1, then it also known as a "virtual device".
    pub device: u32,
    /// Returns the default context packet report rate, in Hertz.
    pub pkt_rate: u32,
    /// Returns which optional data items will be in packets returned from the context.
    /// For the default digitizing context, this field must at least indicate buttons, x, and y data. ??????
    pub pkt_data: u32,
    /// Returns whether the packet data items will be returned in absolute or relative mode.
    pub pkt_mode: u32,
    /// Returns which packet data items can generate motion events in the context.
    pub move_mask: u32,
    /// Returns the buttons for which button press events will be processed in the context.
    /// The default context must at least select button press events for one button.
    pub btn_dn_mask: u32,
    /// Returns the buttons for which button release events will be processed in the context.
    pub btn_up_mask: u32,
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
    /// Relative-mode sensitivity factor. (X)
    pub out_sens_x: i32,
    /// Relative-mode sensitivity factor. (Y)
    pub out_sens_y: i32,
    /// Relative-mode sensitivity factor. (Z)
    pub out_sens_z: i32,
    /// Returns the default system cursor tracking mode.
    pub sys_mode: i32,
    /// Returns 0.
    pub sys_org_x: i32,
    /// Returns 0.
    pub sys_org_y: i32,
    /// Returns the current screen display size in pixels. (X)
    pub sys_ext_x: i32,
    /// Returns the current screen display size in pixels. (Y)
    pub sys_ext_y: i32,
    /// Returns the system cursor relative-mode sensitivity factor. (X)
    pub sys_sens_x: i32,
    /// Returns the system cursor relative-mode sensitivity factor. (Y)
    pub sys_sens_y: i32,
}
impl WtiLogicalContext {
    pub fn psm_default() -> Self {
        let wintabids = "LOGCTX".encode_utf16().collect::<Vec<u16>>();
        let mut wintabid = [0u8; LOGICAL_CONTEXT_NAMELEN];
        for i in 0..LOGICAL_CONTEXT_NAMELEN {
            if i % 2 == 1 {
                continue;
            }
            let u16i = i / 2;
            if wintabids.len() <= u16i {
                break;
            }
            wintabid[i] = wintabids[u16i] as u8;
            wintabid[i + 1] = (wintabids[u16i] << 8) as u8;
        }
        // println!("{:#?}", wintabid);

        WtiLogicalContext {
            name: wintabid,
            options: 0,
            status: 0, // !
            locks: 0,
            msg_base: WT_DEFBASE,
            device: 0,
            pkt_rate: 100,
            pkt_data: 0,
            pkt_mode: 0,
            move_mask: 0,
            btn_dn_mask: 0,
            btn_up_mask: 0,
            in_org_x: 0,
            in_org_y: 0,
            in_org_z: 0,
            in_ext_x: 0,
            in_ext_y: 0,
            in_ext_z: 0,
            out_org_x: 0,
            out_org_y: 0,
            out_org_z: 0,
            out_ext_x: 0,
            out_ext_y: 0,
            out_ext_z: 0,
            out_sens_x: 0,
            out_sens_y: 0,
            out_sens_z: 0,
            sys_mode: 0,
            sys_org_x: 0,
            sys_org_y: 0,
            sys_ext_x: 0,
            sys_ext_y: 0,
            sys_sens_x: 0,
            sys_sens_y: 0,
        }
    }

    pub fn handle_info(&self, index: u32, lp_output: *mut c_void) -> u32 {
        match index {
            0 => info_write(self, lp_output),
            1 => info_write_array(&self.name, lp_output, LOGICAL_CONTEXT_NAMELEN),
            2 => info_write(&self.options, lp_output),
            3 => info_write(&self.status, lp_output),
            4 => info_write(&self.locks, lp_output),
            5 => info_write(&self.msg_base, lp_output),
            6 => info_write(&self.device, lp_output),
            7 => info_write(&self.pkt_rate, lp_output),
            8 => info_write(&self.pkt_data, lp_output),
            9 => info_write(&self.pkt_mode, lp_output),
            10 => info_write(&self.move_mask, lp_output),
            11 => info_write(&self.btn_dn_mask, lp_output),
            12 => info_write(&self.btn_up_mask, lp_output),
            13 => info_write(&self.in_org_x, lp_output),
            14 => info_write(&self.in_org_y, lp_output),
            15 => info_write(&self.in_org_z, lp_output),
            16 => info_write(&self.in_ext_x, lp_output),
            17 => info_write(&self.in_ext_y, lp_output),
            18 => info_write(&self.in_ext_z, lp_output),
            19 => info_write(&self.out_org_x, lp_output),
            20 => info_write(&self.out_org_y, lp_output),
            21 => info_write(&self.out_org_z, lp_output),
            22 => info_write(&self.out_ext_x, lp_output),
            23 => info_write(&self.out_ext_y, lp_output),
            24 => info_write(&self.out_ext_z, lp_output),
            25 => info_write(&self.out_sens_x, lp_output),
            26 => info_write(&self.out_sens_y, lp_output),
            27 => info_write(&self.out_sens_z, lp_output),
            28 => info_write(&self.sys_mode, lp_output),
            29 => info_write(&self.sys_org_x, lp_output),
            30 => info_write(&self.sys_org_y, lp_output),
            31 => info_write(&self.sys_ext_x, lp_output),
            32 => info_write(&self.sys_ext_y, lp_output),
            33 => info_write(&self.sys_sens_x, lp_output),
            34 => info_write(&self.sys_sens_y, lp_output),
            _ => 0
        }
    }
}

pub const TU_NONE: u32 = 0;
pub const TU_INCHES: u32 = 1;
pub const TU_CENTIMETERS: u32 = 2;
pub const TU_CIRCLE: u32 = 3;

#[repr(C)]
/// The AXIS data structure defines the range and resolution for many of the packet data items.
pub struct Axis {
    /// Specifies the minimum value of the data item in the tablet's native coordinates.
    pub min: i32,
    /// Specifies the maximum value of the data item in the tablet's native coordinates.
    pub max: i32,
    /// Indicates the units used in calculating the resolution for the data item.
    /// [TU_NONE], [TU_INCHES], [TU_CENTIMETERS], [TU_CIRCLE]
    pub units: u32,
    /// Is a fixed-point number giving the number of data item increments per physical unit.
    pub resolution: u32,
}
impl Axis {
    pub fn psm_default() -> Self {
        Axis {
            min: 0,
            max: 65535,
            units: TU_INCHES,
            resolution: 1,
        }
    }
}
