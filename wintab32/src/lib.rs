use std::{
    collections::{HashMap, VecDeque}, ffi::c_void, fs::OpenOptions, io::{Read, Write}, net::{TcpListener, TcpStream}, ops::BitXor, sync::{LazyLock, Mutex}, time::Instant, u32
};

use color_eyre::eyre::{ContextCompat, bail};
use log::{debug, error, info, warn};
use static_init::{constructor, destructor};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::*,
};

use crate::{config::Config, ffi::*};
use psm_common::netcode::{COMPATIBLE_VERSION, PSMPacketC2S, PSMPacketS2C};

pub mod config;
pub mod ffi;
pub mod info_write;
pub mod netcompat;
pub mod ptr;

static STATE: LazyLock<Mutex<Option<PSM>>> = LazyLock::new(|| Mutex::new(None));

#[constructor(0)]
extern "C" fn init_main() {
    if let Ok(log_path) = std::env::var("PSM_LOG_FILE") {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .expect("failed to open log file (PSM_LOG_FILE) for writing");
        colog::default_builder()
            .target(env_logger::Target::Pipe(Box::new(file)))
            .init();
    } else {
        colog::init();
    }
    color_eyre::install().ok();
    panic_pipe();
    if let Err(err) = main() {
        error!("{:?}", err);
        error!("PSM's main thread failed! It's recommended to restart the app.");
    }
}

fn panic_pipe() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("psm.panic.log");
        if let Ok(mut file) = file {
            file.write_all((&format!("{:#?}\n{:#?}", info.payload_as_str(), info)).as_bytes())
                .ok();
        }
        error!("{:#?}\n{:#?}", info.payload_as_str(), info);
        default_panic(info);
    }));
}

#[destructor(0)]
extern "C" fn free_main() {
    info!("bye!");
}

pub fn main() -> color_eyre::Result<()> {
    init()?;

    Ok(())
}

pub fn get_state_or_init()
-> color_eyre::Result<std::sync::MutexGuard<'static, std::option::Option<PSM>>> {
    let no_state = STATE.lock().unwrap().is_none();
    if no_state {
        init()?;
    }
    Ok(STATE.lock().unwrap())
}

pub fn init() -> color_eyre::Result<()> {
    if STATE.lock().unwrap().is_some() {
        return Ok(());
    }

    debug!("PSM debug");

    {
        let config: Config = config::find_config()?;
        let mut state = STATE.lock().unwrap();
        *state = Some(PSM::new(config));
    }

    info!("PSM v{} is loaded!", env!("CARGO_PKG_VERSION"));

    std::thread::spawn(tcp_thread);

    Ok(())
}

pub fn tcp_thread() {
    let socket = TcpListener::bind("127.0.0.1:40302");
    let socket = match socket {
        Ok(v) => v,
        Err(err) => {
            error!("{:?}", err);
            error!("Failed to bind! PSM WILL NOT WORK.");
            return;
        }
    };
    loop {
        info!("PSM is now listening on 127.0.0.1:40302");
        let stream = socket.accept();
        match stream {
            Ok((stream, addr)) => {
                info!("Accepted connection from {}", addr);
                match handle_client(stream) {
                    Ok(_) => {}
                    Err(err) => {
                        info!("{:?}", err);
                        info!("Connection from {} ended", addr);
                    }
                }
            }
            Err(err) => error!("Connection failed! {:?}", err),
        }
    }
}
pub fn handle_client(mut socket: TcpStream) -> color_eyre::Result<()> {
    loop {
        let mut packet_size_buf = [0u8; 4];
        socket.read_exact(&mut packet_size_buf)?;
        let size = u32::from_be_bytes(packet_size_buf);
        let mut buf: Vec<u8> = vec![0u8; size as usize];
        socket.read_exact(&mut buf)?;

        let packet = serde_json::from_slice::<PSMPacketC2S>(&buf)?;
        debug!("Packet received: {:#?}", packet);
        match packet {
            PSMPacketC2S::Hi { name } => {
                info!("Client: {}", name);
                send_packet(
                    &mut socket,
                    &PSMPacketS2C::Hi {
                        compatible: COMPATIBLE_VERSION,
                    },
                )?;
            }
            PSMPacketC2S::TabletEvent {
                status,
                buttons,
                x,
                y,
                z,
                normal_pressure,
                tangential_pressure,
            } => {
                let mut state = get_state_or_init().unwrap();
                let state = state.as_mut().unwrap();
                for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                    if let Err(err) = ctx.handle_packet(Packet {
                        context: ctx.handle as u32,
                        status,
                        time: 0,
                        changed: 0xFFFFFFFF,
                        serial: 0,
                        cursor: 0,
                        buttons,
                        x,
                        y,
                        z,
                        normal_pressure,
                        tangential_pressure,
                        orientation: Orientation::default(),
                        rotation: Rotation::default(),
                    }) {
                        error!("Couldn't send the packet! {:?}", err);
                    }
                }
            }
            PSMPacketC2S::Proximity { value } => {
                let mut state = get_state_or_init().unwrap();
                let state = state.as_mut().unwrap();
                for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                    if let Err(err) = ctx.proximity(value) {
                        error!("Couldn't send the proximity update! {:?}", err);
                    }
                }
            }
            PSMPacketC2S::ConfigureContext {
                status,
                packet_rate,
                packet_mode,
                move_mask,
                in_org_x,
                in_org_y,
                in_org_z,
                in_ext_x,
                in_ext_y,
                in_ext_z,
                out_org_x,
                out_org_y,
                out_org_z,
                out_ext_x,
                out_ext_y,
                out_ext_z,
                sys_org_x,
                sys_org_y,
                sys_ext_x,
                sys_ext_y,
            } => {
                let mut state = get_state_or_init().unwrap();
                let state = state.as_mut().unwrap();
                for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                    ctx.logical_context.status = status;
                    ctx.logical_context.packet_rate = packet_rate;
                    ctx.logical_context.packet_mode = packet_mode;
                    ctx.logical_context.move_mask = move_mask;
                    ctx.logical_context.in_org_x = in_org_x;
                    ctx.logical_context.in_org_y = in_org_y;
                    ctx.logical_context.in_org_z = in_org_z;
                    ctx.logical_context.in_ext_x = in_ext_x;
                    ctx.logical_context.in_ext_y = in_ext_y;
                    ctx.logical_context.in_ext_z = in_ext_z;
                    ctx.logical_context.out_org_x = out_org_x;
                    ctx.logical_context.out_org_y = out_org_y;
                    ctx.logical_context.out_org_z = out_org_z;
                    ctx.logical_context.out_ext_x = out_ext_x;
                    ctx.logical_context.out_ext_y = out_ext_y;
                    ctx.logical_context.out_ext_z = out_ext_z;
                    ctx.logical_context.sys_org_x = sys_org_x;
                    ctx.logical_context.sys_org_y = sys_org_y;
                    ctx.logical_context.sys_ext_x = sys_ext_x;
                    ctx.logical_context.sys_ext_y = sys_ext_y;
                    if let Err(err) = ctx.context_update() {
                        error!("Couldn't send the context update! {:?}", err);
                    }
                }
            }
            PSMPacketC2S::ConfigureDevice {
                hardware,
                packet_rate,
                packet_mode,
                x_margin,
                y_margin,
                z_margin,
                device_x,
                device_y,
                device_z,
                normal_pressure,
                tangential_pressure,
                orientation,
                rotation,
            } => {
                let mut state = get_state_or_init().unwrap();
                let state = state.as_mut().unwrap();
                state.device.hardware = hardware;
                state.device.packet_rate = packet_rate;
                state.device.packet_mode = packet_mode;
                state.device.x_margin = x_margin;
                state.device.y_margin = y_margin;
                state.device.z_margin = z_margin;
                state.device.device_x = device_x.into();
                state.device.device_y = device_y.into();
                state.device.device_z = device_z.into();
                state.device.normal_pressure = normal_pressure.into();
                state.device.tangential_pressure = tangential_pressure.into();
                state.device.orientation = orientation.map(|x| x.into());
                state.device.rotation = rotation.map(|x| x.into());
                for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                    if let Err(err) = ctx.info_update() {
                        error!("Couldn't send the info update! {:?}", err);
                    }
                }
            }
            PSMPacketC2S::Debug { msg: _ } => {}
        }
    }
    // Ok(())
}
pub fn send_packet(stream: &mut impl Write, packet: &PSMPacketS2C) -> color_eyre::Result<()> {
    let data = serde_json::to_vec(packet)?;
    let bytes: [u8; 4] = (data.len() as u32).to_be_bytes();
    stream.write_all(&bytes)?;
    stream.write_all(&data)?;
    Ok(())
}

pub struct PSM {
    pub contexts: HashMap<usize, Context>,
    pub counter: usize,
    pub default_context: WtiLogicalContext,
    // support for multiple devices/cursors?
    pub device: WtiDevice,
    pub cursor: WtiCursor,
    pub config: Config,
}
impl PSM {
    pub fn new(config: Config) -> Self {
        let mut state = Self {
            contexts: Default::default(),
            counter: Default::default(),
            default_context: WtiLogicalContext::psm_default(),
            device: WtiDevice::psm_default(),
            cursor: WtiCursor::psm_default(),
            config,
        };
        state.apply_config();
        state
    }

    fn apply_config(&mut self) {
        // TODO: remove unused config.preset options
        self.default_context.status = 0; // self.config.preset.status;
        self.default_context.packet_rate = self.config.preset.packet_rate;
        self.default_context.packet_mode = self.config.preset.packet_mode;
        self.default_context.move_mask = self.config.preset.move_mask;
        self.default_context.in_org_x = self.config.preset.in_org_x;
        self.default_context.in_org_y = self.config.preset.in_org_y;
        self.default_context.in_org_z = self.config.preset.in_org_z;
        self.default_context.in_ext_x = self.config.preset.in_ext_x.abs();
        self.default_context.in_ext_y = self.config.preset.in_ext_y.abs();
        self.default_context.in_ext_z = self.config.preset.in_ext_z.abs();
        self.default_context.out_org_x = self.config.preset.out_org_x;
        self.default_context.out_org_y = self.config.preset.out_org_y;
        self.default_context.out_org_z = self.config.preset.out_org_z;
        self.default_context.out_ext_x = self.config.preset.out_ext_x.abs();
        self.default_context.out_ext_y = self.config.preset.out_ext_y.abs();
        self.default_context.out_ext_z = self.config.preset.out_ext_z.abs();
        self.default_context.sys_org_x = 0; // self.config.preset.sys_org_x;
        self.default_context.sys_org_y = 0; // self.config.preset.sys_org_y;
        self.default_context.sys_ext_x = self.config.preset.sys_ext_x.abs();
        self.default_context.sys_ext_y = self.config.preset.sys_ext_y.abs();
        self.device.hardware = self.config.preset.hardware;
        self.device.packet_rate = self.config.preset.packet_rate;
        self.device.packet_mode = self.config.preset.packet_mode;
        self.device.x_margin = self.config.preset.x_margin;
        self.device.y_margin = self.config.preset.y_margin;
        self.device.z_margin = self.config.preset.z_margin;
        self.device.device_x = self.config.preset.device_x.into();
        self.device.device_y = self.config.preset.device_y.into();
        self.device.device_z = self.config.preset.device_z.into();
        self.device.normal_pressure = self.config.preset.normal_pressure.into();
        self.device.tangential_pressure = self.config.preset.tangential_pressure.into();
        self.device.orientation = self.config.preset.orientation.map(|x| x.into());
        self.device.rotation = self.config.preset.rotation.map(|x| x.into());
        // self.device.packet_data = 0x1ff;
        // self.device.csr_data = 0x1e00;
    }

    // Invalid or out-of-range attribute values in the logical context structure
    // will either be validated, or cause the open to fail, depending on the attributes involved.
    // Upon a successful return from the function, the context specification pointed to by lpLogCtx
    // will contain the validated values.
    pub fn validate_context(
        &self,
        context: &WtiLogicalContext,
    ) -> color_eyre::Result<WtiLogicalContext> {
        let mut context = context.clone();
        context.in_ext_x = context.in_ext_x.abs();
        context.in_ext_y = context.in_ext_y.abs();
        context.in_ext_z = context.in_ext_z.abs();
        context.out_ext_x = context.out_ext_x.abs();
        context.out_ext_y = context.out_ext_y.abs();
        context.out_ext_z = context.out_ext_z.abs();
        context.sys_org_x = context.sys_org_x.abs();
        context.sys_org_y = context.sys_org_y.abs();
        context.sys_ext_x = context.sys_ext_x.abs();
        context.sys_ext_y = context.sys_ext_y.abs();
        self.debug_default_context_diff(&context);
        Ok(context)
    }

    fn debug_default_context_diff(&self, context: &WtiLogicalContext) {
        debug!("Differences between default and application contexts (*new* != default):");
        if context.options != self.default_context.options {
            debug!(
                "options: {} != {}",
                context.options, self.default_context.options
            );
        }
        if context.status != self.default_context.status {
            debug!(
                "status: {} != {}",
                context.status, self.default_context.status
            );
        }
        if context.locks != self.default_context.locks {
            debug!("locks: {} != {}", context.locks, self.default_context.locks);
        }
        if context.msg_base != self.default_context.msg_base {
            debug!(
                "msg_base: {} != {}",
                context.msg_base, self.default_context.msg_base
            );
        }
        if context.device != self.default_context.device {
            debug!(
                "device: {} != {}",
                context.device, self.default_context.device
            );
        }
        if context.packet_rate != self.default_context.packet_rate {
            debug!(
                "packet_rate: {} != {}",
                context.packet_rate, self.default_context.packet_rate
            );
        }
        if context.packet_data != self.default_context.packet_data {
            debug!(
                "packet_data: {} != {}",
                context.packet_data, self.default_context.packet_data
            );
        }
        if context.packet_mode != self.default_context.packet_mode {
            debug!(
                "packet_mode: {} != {}",
                context.packet_mode, self.default_context.packet_mode
            );
        }
        if context.move_mask != self.default_context.move_mask {
            debug!(
                "move_mask: {} != {}",
                context.move_mask, self.default_context.move_mask
            );
        }
        if context.btn_dn_mask != self.default_context.btn_dn_mask {
            debug!(
                "btn_dn_mask: {} != {}",
                context.btn_dn_mask, self.default_context.btn_dn_mask
            );
        }
        if context.btn_up_mask != self.default_context.btn_up_mask {
            debug!(
                "btn_up_mask: {} != {}",
                context.btn_up_mask, self.default_context.btn_up_mask
            );
        }
        if context.in_org_x != self.default_context.in_org_x {
            debug!(
                "in_org_x: {} != {}",
                context.in_org_x, self.default_context.in_org_x
            );
        }
        if context.in_org_y != self.default_context.in_org_y {
            debug!(
                "in_org_y: {} != {}",
                context.in_org_y, self.default_context.in_org_y
            );
        }
        if context.in_org_z != self.default_context.in_org_z {
            debug!(
                "in_org_z: {} != {}",
                context.in_org_z, self.default_context.in_org_z
            );
        }
        if context.in_ext_x != self.default_context.in_ext_x {
            debug!(
                "in_ext_x: {} != {}",
                context.in_ext_x, self.default_context.in_ext_x
            );
        }
        if context.in_ext_y != self.default_context.in_ext_y {
            debug!(
                "in_ext_y: {} != {}",
                context.in_ext_y, self.default_context.in_ext_y
            );
        }
        if context.in_ext_z != self.default_context.in_ext_z {
            debug!(
                "in_ext_z: {} != {}",
                context.in_ext_z, self.default_context.in_ext_z
            );
        }
        if context.out_org_x != self.default_context.out_org_x {
            debug!(
                "out_org_x: {} != {}",
                context.out_org_x, self.default_context.out_org_x
            );
        }
        if context.out_org_y != self.default_context.out_org_y {
            debug!(
                "out_org_y: {} != {}",
                context.out_org_y, self.default_context.out_org_y
            );
        }
        if context.out_org_z != self.default_context.out_org_z {
            debug!(
                "out_org_z: {} != {}",
                context.out_org_z, self.default_context.out_org_z
            );
        }
        if context.out_ext_x != self.default_context.out_ext_x {
            debug!(
                "out_ext_x: {} != {}",
                context.out_ext_x, self.default_context.out_ext_x
            );
        }
        if context.out_ext_y != self.default_context.out_ext_y {
            debug!(
                "out_ext_y: {} != {}",
                context.out_ext_y, self.default_context.out_ext_y
            );
        }
        if context.out_ext_z != self.default_context.out_ext_z {
            debug!(
                "out_ext_z: {} != {}",
                context.out_ext_z, self.default_context.out_ext_z
            );
        }
        if context.sens_x != self.default_context.sens_x {
            debug!(
                "sens_x: {} != {}",
                context.sens_x, self.default_context.sens_x
            );
        }
        if context.sens_y != self.default_context.sens_y {
            debug!(
                "sens_y: {} != {}",
                context.sens_y, self.default_context.sens_y
            );
        }
        if context.sens_z != self.default_context.sens_z {
            debug!(
                "sens_z: {} != {}",
                context.sens_z, self.default_context.sens_z
            );
        }
        if context.sys_mode != self.default_context.sys_mode {
            debug!(
                "sys_mode: {} != {}",
                context.sys_mode, self.default_context.sys_mode
            );
        }
        if context.sys_org_x != self.default_context.sys_org_x {
            debug!(
                "sys_org_x: {} != {}",
                context.sys_org_x, self.default_context.sys_org_x
            );
        }
        if context.sys_org_y != self.default_context.sys_org_y {
            debug!(
                "sys_org_y: {} != {}",
                context.sys_org_y, self.default_context.sys_org_y
            );
        }
        if context.sys_ext_x != self.default_context.sys_ext_x {
            debug!(
                "sys_ext_x: {} != {}",
                context.sys_ext_x, self.default_context.sys_ext_x
            );
        }
        if context.sys_ext_y != self.default_context.sys_ext_y {
            debug!(
                "sys_ext_y: {} != {}",
                context.sys_ext_y, self.default_context.sys_ext_y
            );
        }
        if context.sys_sens_x != self.default_context.sys_sens_x {
            debug!(
                "sys_sens_x: {} != {}",
                context.sys_sens_x, self.default_context.sys_sens_x
            );
        }
        if context.sys_sens_y != self.default_context.sys_sens_y {
            debug!(
                "sys_sens_y: {} != {}",
                context.sys_sens_y, self.default_context.sys_sens_y
            );
        }
    }
}

pub struct Context {
    pub handle: usize,
    pub enabled: bool,
    pub window: ThreadHWND,
    pub logical_context: WtiLogicalContext,
    pub packets: VecDeque<Packet>,
    pub last_packet: Option<Packet>,
    pub queue_size: usize,
    pub serial: usize,
    pub time: Instant,
}
impl Context {
    pub fn new(handle: usize, enabled: bool) -> Self {
        Self {
            handle,
            enabled,
            window: ThreadHWND::default(),
            logical_context: WtiLogicalContext::psm_default(),
            packets: VecDeque::new(),
            last_packet: None,
            queue_size: 1024,
            serial: 0,
            time: Instant::now(),
        }
    }

    pub fn handle_packet(&mut self, mut packet: Packet) -> color_eyre::Result<()> {
        if !self.enabled {
            bail!("packet sent when context is disabled");
        }
        if self.window.0.0.is_null() {
            bail!("packet sent without a valid window");
        }
        if (packet.x as i32) > self.logical_context.out_ext_x
            || (packet.y as i32) > self.logical_context.out_ext_y
            || (packet.x as i32) < self.logical_context.out_org_x
            || (packet.y as i32) < self.logical_context.out_org_y
        {
            warn!(
                "Ignoring packet with out of range coordinates! You might need to check your psm.json."
            );
            return Ok(());
        }
        self.serial += 1;
        packet.context = self.handle as u32;
        packet.serial = self.serial as u32;
        packet.time = self.time.elapsed().as_millis() as u32;
        packet.changed = self.find_packet_changes(&packet);
        debug!("Original WinTab packet: {:?}", packet);
        packet = self.relativize_packet(packet);
        debug!("Relativized WinTab packet: {:?}", packet);
        // limiting by queue size
        let queue_size = self.queue_size.max(1) as isize;
        let overflow_size = (self.packets.len() as isize) - queue_size + 1;
        // if overflow_size > 0 {
        //     debug!("packet overflow: {}", overflow_size);
        // }
        for _overflow in 0..overflow_size {
            self.packets.pop_front();
        }
        self.packets.push_back(packet);
        if (self.logical_context.options & CXO_MESSAGES) > 0 {
            // posting WT_PACKET(serial, ctx_handle)
            unsafe {
                PostMessageW(
                    Some(self.window.0),
                    WindowMessage::Packet.value(self.logical_context.msg_base),
                    WPARAM(self.serial),
                    LPARAM(self.handle as isize),
                )?
            };
        }
        Ok(())
    }

    fn relativize_packet(&mut self, packet: Packet) -> Packet {
        if let None = self.last_packet {
            self.last_packet = Some(packet.clone());
            return packet;
        }
        let last_packet = self.last_packet.as_ref().unwrap().clone();

        let mut relative_packet = packet.clone();
        if (self.logical_context.packet_mode & PK_STATUS) > 0 {
            relative_packet.status = packet.status.bitxor(last_packet.status);
        }
        if (self.logical_context.packet_mode & PK_TIME) > 0 {
            let current_time = self.time.elapsed().as_millis() as u32;
            relative_packet.time = current_time - last_packet.time;
        }
        if (self.logical_context.packet_mode & PK_BUTTONS) > 0 {
            relative_packet.buttons = packet.buttons.bitxor(last_packet.buttons);
        }
        if (self.logical_context.packet_mode & PK_X) > 0 {
            relative_packet.x = packet.x - last_packet.x;
        }
        if (self.logical_context.packet_mode & PK_Y) > 0 {
            relative_packet.y = packet.y - last_packet.y;
        }
        if (self.logical_context.packet_mode & PK_Z) > 0 {
            relative_packet.z = packet.z - last_packet.z;
        }
        if (self.logical_context.packet_mode & PK_NORMAL_PRESSURE) > 0 {
            relative_packet.normal_pressure = packet.normal_pressure - last_packet.normal_pressure;
        }
        if (self.logical_context.packet_mode & PK_TANGENT_PRESSURE) > 0 {
            relative_packet.tangential_pressure = packet.tangential_pressure - last_packet.tangential_pressure;
        }
        if (self.logical_context.packet_mode & PK_ORIENTATION) > 0 {
            relative_packet.orientation = Orientation {
                azimuth: packet.orientation.azimuth - last_packet.orientation.azimuth,
                altitude: packet.orientation.altitude - last_packet.orientation.altitude,
                twist: packet.orientation.twist - last_packet.orientation.twist,
            };
        }
        if (self.logical_context.packet_mode & PK_ROTATION) > 0 {
            relative_packet.rotation = Rotation {
                pitch: packet.rotation.pitch - last_packet.rotation.pitch,
                roll: packet.rotation.roll - last_packet.rotation.roll,
                yaw: packet.rotation.yaw - last_packet.rotation.yaw,
            };
        }

        self.last_packet = Some(packet);
        relative_packet
    }

    fn find_packet_changes(&mut self, packet: &Packet) -> WTPKT {
        let mut changes: WTPKT = 0;
        if let None = self.last_packet {
            return WTPKT::MAX;
        }
        let last_packet = self.last_packet.as_ref().unwrap().clone();

        if packet.context != last_packet.context {
            changes |= PK_CONTEXT;
        }
        if packet.status != last_packet.status {
            changes |= PK_STATUS;
        }
        // if packet.time != last_packet.time {
        changes |= PK_TIME;
        // }
        if packet.serial != last_packet.serial {
            changes |= PK_SERIAL_NUMBER;
        }
        if packet.cursor != last_packet.cursor {
            changes |= PK_CURSOR;
        }
        if packet.buttons != last_packet.buttons {
            changes |= PK_BUTTONS;
        }
        if packet.x != last_packet.x {
            changes |= PK_X;
        }
        if packet.y != last_packet.y {
            changes |= PK_Y;
        }
        if packet.z != last_packet.z {
            changes |= PK_Z;
        }
        if packet.normal_pressure != last_packet.normal_pressure {
            changes |= PK_NORMAL_PRESSURE;
        }
        if packet.tangential_pressure != last_packet.tangential_pressure {
            changes |= PK_TANGENT_PRESSURE;
        }
        if packet.orientation != last_packet.orientation {
            changes |= PK_ORIENTATION;
        }
        if packet.rotation != last_packet.rotation {
            changes |= PK_ROTATION;
        }

        changes
    }

    #[deprecated]
    pub fn context_update(&mut self) -> color_eyre::Result<()> {
        if self.window.0.0.is_null() {
            bail!("update sent without a valid window");
        }
        self.absolute_ext();
        // posting WT_CTXUPDATE(ctx_handle, status)
        unsafe {
            PostMessageW(
                Some(self.window.0),
                WindowMessage::CtxUpdate.value(self.logical_context.msg_base),
                WPARAM(self.handle),
                LPARAM(if self.enabled { CXS_DISABLED } else { 0 } as isize),
            )?
        };
        Ok(())
    }

    pub fn post_open(&mut self) -> color_eyre::Result<()> {
        if self.window.0.0.is_null() {
            bail!("update sent without a valid window");
        }
        // posting WT_CTXOPEN(ctx_handle, status)
        unsafe {
            PostMessageW(
                Some(self.window.0),
                WindowMessage::CtxOpen.value(self.logical_context.msg_base),
                WPARAM(self.handle),
                LPARAM(self.logical_context.status as isize),
            )?
        };
        Ok(())
    }

    pub fn post_close(&mut self) -> color_eyre::Result<()> {
        if self.window.0.0.is_null() {
            bail!("update sent without a valid window");
        }
        // posting WT_CTXCLOSE(ctx_handle, status)
        unsafe {
            PostMessageW(
                Some(self.window.0),
                WindowMessage::CtxClose.value(self.logical_context.msg_base),
                WPARAM(self.handle),
                LPARAM(self.logical_context.status as isize),
            )?
        };
        Ok(())
    }

    pub fn info_update(&mut self) -> color_eyre::Result<()> {
        if self.window.0.0.is_null() {
            bail!("update sent without a valid window");
        }
        // posting WT_INFOCHANGE(0, categoryAndIndex)
        unsafe {
            PostMessageW(
                Some(self.window.0),
                WindowMessage::InfoChange.value(self.logical_context.msg_base),
                WPARAM(0),
                LPARAM(0x10004),
            )?;
        };
        Ok(())
    }

    pub fn proximity(&mut self, value: bool) -> color_eyre::Result<()> {
        if !self.enabled {
            bail!("packet sent when context is disabled");
        }
        if self.window.0.0.is_null() {
            bail!("packet sent without a valid window");
        }
        // posting WT_PROXIMITY(ctx_handle, value)
        unsafe {
            PostMessageW(
                Some(self.window.0),
                WindowMessage::Proximity.value(self.logical_context.msg_base),
                WPARAM(self.handle),
                LPARAM(if value { 0x00010001 } else { 0 }),
            )?
        };
        Ok(())
    }

    #[deprecated(since = "0.1.0", note = "use PSM::validate_context() instead")]
    pub fn absolute_ext(&mut self) {
        self.logical_context.in_ext_x = self.logical_context.in_ext_x.abs();
        self.logical_context.in_ext_y = self.logical_context.in_ext_y.abs();
        self.logical_context.in_ext_z = self.logical_context.in_ext_z.abs();
        self.logical_context.out_ext_x = self.logical_context.out_ext_x.abs();
        self.logical_context.out_ext_y = self.logical_context.out_ext_y.abs();
        self.logical_context.out_ext_z = self.logical_context.out_ext_z.abs();
        self.logical_context.sys_ext_x = self.logical_context.sys_ext_x.abs();
        self.logical_context.sys_ext_y = self.logical_context.sys_ext_y.abs();
    }
}

#[derive(Debug, Default)]
pub struct ThreadHWND(pub HWND);
unsafe impl Send for ThreadHWND {}
unsafe impl Sync for ThreadHWND {}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn WTOpenA(
    hwnd: HWND,
    lp_log_ctx: *mut WtiLogicalContext,
    f_enable: bool,
) -> usize {
    debug!("WTOpenA({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    unsafe { WTOpen(hwnd, lp_log_ctx, f_enable) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn WTOpenW(
    hwnd: HWND,
    lp_log_ctx: *mut WtiLogicalContext,
    f_enable: bool,
) -> usize {
    debug!("WTOpenW({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    unsafe { WTOpen(hwnd, lp_log_ctx, f_enable) }
}
// This function establishes an active context on the tablet.
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn WTOpen(
    // Identifies the window that owns the tablet context, and receives messages from the context.
    hwnd: HWND,
    // Points to an application-provided LOGCONTEXT data structure describing the context to be opened.
    lp_log_ctx: *mut WtiLogicalContext,
    // Specifies whether the new context will immediately begin processing input data.
    f_enable: bool,
) -> usize {
    debug!("WTOpen({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    if lp_log_ctx.is_null() {
        error!("WTOpen lp_log_ctx is null");
        return 0;
    }
    unsafe {
        debug!("LogContext -> {:#?}", *lp_log_ctx);
    }

    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();

    let mut logical_context = WtiLogicalContext::psm_default();
    unsafe {
        std::ptr::copy(lp_log_ctx, &mut logical_context, 1);
    }
    logical_context = match state.validate_context(&logical_context) {
        Ok(ctx) => ctx,
        Err(err) => {
            error!("application logical context validation failed: {:?}", err);
            return 0;
        }
    };

    // Handle is incremental.
    state.counter += 1;
    let handle = state.counter;

    let mut context = Context::new(handle, f_enable);
    context.window = ThreadHWND(hwnd);
    context.logical_context = logical_context;
    if let Err(err) = context.post_open() {
        error!("failed to send the application WT_CTXOPEN: {:?}", err);
    }
    state.contexts.insert(handle, context);
    debug!(
        "new context registered at {} (enabled = {})",
        handle, f_enable
    );

    // TODO(overlap): The newly opened tablet context will be placed on the top of the context overlap order.

    handle
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTEnable(ctx_id: usize, enable: bool) -> bool {
    debug!("WTEnable({:#?}, {})", ctx_id, enable);
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = match state.contexts.get_mut(&ctx_id) {
        Some(ctx) => ctx,
        None => return false,
    };
    ctx.enabled = enable;
    true
}

// https://developer-docs.wacom.com/docs/icbt/windows/wintab/wintab-reference/#wtpacketsget
#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTPacketsGet(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> u32 {
    debug!(
        "WTPacketsGet({:#?}, {:#?}, {:#?})",
        ctx_id, max_packets, ptr
    );
    match packets_get(ctx_id, max_packets, ptr) {
        Ok(v) => v,
        Err(err) => {
            error!(
                "WTPacketsGet({:#?}, {:#?}, {:#?}) failed!",
                ctx_id, max_packets, ptr
            );
            error!("{:?}", err);
            0
        }
    }
}
pub fn packets_get(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> color_eyre::Result<u32> {
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    if ptr == std::ptr::null_mut() {
        // flush queue
        ctx.packets.clear();
        return Ok(0);
    }
    let mut count = 0;
    for i in 0..max_packets {
        let packet = match ctx.packets.pop_front() {
            Some(x) => x,
            None => break,
        };
        // TODO: FIXME: ooooh scary pointer arithmetics.
        let packet_size = size_of::<Packet>();
        packet.write(
            ptr.wrapping_add(packet_size * (i as usize)),
            ctx.logical_context.packet_data,
        );
        count += 1;
    }

    Ok(count as u32)
}

// only by hope the parameters of this function may be determined
// bask in the glory of https://developer-docs.wacom.com/docs/icbt/windows/wintab/wintab-reference/#wtpacketspeek
#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTPacketsPeek(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> u32 {
    // pub extern "C-unwind" fn WTPacketsPeek(ctx_id: usize, ext: u32, ptr: *mut c_void) -> i32 {
    debug!(
        "WTPacketsPeek({:#?}, {:#?}, {:#?})",
        ctx_id, max_packets, ptr
    );
    match packets_peek(ctx_id, max_packets, ptr) {
        Ok(v) => v,
        Err(err) => {
            error!(
                "WTPacketsPeek({:#?}, {:#?}, {:#?}) failed!",
                ctx_id, max_packets, ptr
            );
            error!("{:?}", err);
            0
        }
    }
}
pub fn packets_peek(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> color_eyre::Result<u32> {
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    if ptr == std::ptr::null_mut() {
        // flush queue
        ctx.packets.clear();
        return Ok(0);
    }
    let mut count = 0;
    let mut packets = ctx.packets.iter();
    let mut ptr = ptr.clone();
    for _ in 0..max_packets {
        let packet = match packets.next() {
            Some(x) => x,
            None => break,
        };
        let written = packet.write(
            ptr,
            ctx.logical_context.packet_data,
        );
        // TODO: FIXME: ooooh scary pointer arithmetics.
        // would you believe me if i said that the line that said "scary pointer arithmetics"
        // was the line that was responsible for crashes?
        ptr = ptr.wrapping_add(written as usize);
        count += 1;
    }

    Ok(count as u32)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTPacket(ctx_id: usize, serial: u32, ptr: *mut c_void) -> bool {
    debug!("WTPacket({:#?}, {:#?}, {:#?})", ctx_id, serial, ptr);
    match packet(ctx_id, serial, ptr) {
        Ok(v) => v,
        Err(err) => {
            error!("WTPacket({:#?}, {:#?}, {:#?}) failed!", ctx_id, serial, ptr);
            error!("{:?}", err);
            false
        }
    }
}
pub fn packet(ctx_id: usize, serial: u32, ptr: *mut c_void) -> color_eyre::Result<bool> {
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    ctx.packets.retain_mut(|x| x.serial >= serial);
    let packet = match ctx.packets.iter().find(|x| x.serial == serial) {
        Some(x) => x,
        None => return Ok(false),
    };
    packet.write(ptr, ctx.logical_context.packet_data);
    ctx.packets.retain_mut(|x| x.serial > serial);
    Ok(true)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTOverlap(ctx_id: usize, overlap: bool) -> bool {
    debug!("!STUB! WTOverlap({:#?}, {:#?}) -> true", ctx_id, overlap);
    true
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTClose(ctx_id: usize) -> bool {
    debug!("WTClose({:#?})", ctx_id);
    match close(ctx_id) {
        Ok(v) => v,
        Err(err) => {
            error!("WTClose({:#?}) failed!", ctx_id);
            error!("{:?}", err);
            false
        }
    }
}
pub fn close(ctx_id: usize) -> color_eyre::Result<bool> {
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    ctx.post_close().ok();
    state.contexts.retain(|i, _| *i != ctx_id);
    Ok(true)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTGetA(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTGetA({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTGetW(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTGetW({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTGet(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTGet({:#?}, {:#?})", ctx_id, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTSetA(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSetA({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTSetW(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSetW({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTSet(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSet({:#?}, {:#?})", ctx_id, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTExtGet(ctx_id: usize, ext: u32, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTExtGet({:#?}, {:#?}, {:#?})", ctx_id, ext, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTExtSet(ctx_id: usize, ext: u32, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTExtSet({:#?}, {:#?}, {:#?})", ctx_id, ext, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTSave(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSave({:#?}, {:#?})", ctx_id, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTRestore(hwnd: HWND, ptr: *mut c_void, value: bool) -> usize {
    debug!("!STUB! WTRestore({:#?}, {:#?}, {:#?})", hwnd, ptr, value);
    0
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTDataGet(
    ctx_id: usize,
    begin: u32,
    end: u32,
    max_packets: i32,
    ptr: *mut c_void,
    ints: *mut c_void,
) -> i32 {
    debug!(
        "!STUB! WTDataGet({:#?}, {:#?}, {:#?}, {:#?}, {:#?}, {:#?})",
        ctx_id, begin, end, max_packets, ptr, ints
    );
    0
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTDataPeek(
    ctx_id: usize,
    begin: u32,
    end: u32,
    max_packets: i32,
    ptr: *mut c_void,
    ints: *mut c_void,
) -> i32 {
    debug!(
        "!STUB! WTDataPeek({:#?}, {:#?}, {:#?}, {:#?}, {:#?}, {:#?})",
        ctx_id, begin, end, max_packets, ptr, ints
    );
    0
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTQueuePacketsEx(
    ctx_id: usize,
    old: *mut c_void,
    new: *mut c_void,
) -> bool {
    debug!(
        "!STUB! WTQueuePacketsEx({:#?}, {:#?}, {:#?})",
        ctx_id, old, new
    );
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTQueueSizeGet(ctx_id: usize) -> u32 {
    debug!("WTQueueSizeGet({:#?})", ctx_id);
    match queue_size_get(ctx_id) {
        Ok(v) => v,
        Err(err) => {
            error!("WTQueueSizeGet({:#?}) failed!", ctx_id);
            error!("{:?}", err);
            0
        }
    }
}
pub fn queue_size_get(ctx_id: usize) -> color_eyre::Result<u32> {
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    Ok(ctx.queue_size as u32)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTQueueSizeSet(ctx_id: usize, num_packets: u32) -> bool {
    debug!("WTQueueSizeSet({:#?}, {:#?})", ctx_id, num_packets);
    match queue_size_set(ctx_id, num_packets) {
        Ok(v) => v,
        Err(err) => {
            error!("WTQueueSizeSet({:#?}, {:#?}) failed!", ctx_id, num_packets);
            error!("{:?}", err);
            false
        }
    }
}
pub fn queue_size_set(ctx_id: usize, num_packets: u32) -> color_eyre::Result<bool> {
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    ctx.queue_size = num_packets as usize;
    Ok(true)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTMgrOpen(hwnd: HWND, msg_base: u32) -> usize {
    debug!("!STUB! WTMgrOpen({:#?}, {:#?})", hwnd, msg_base);
    0
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTMgrExt(mgr: usize, value: u32, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTMgrExt({:#?}, {:#?}, {:#?})", mgr, value, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTMgrClose(mgr: usize) -> bool {
    debug!("!STUB! WTMgrClose({:#?})", mgr);
    false
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTMgrDefContextEx(mgr: usize, device: u32, system: bool) -> usize {
    debug!(
        "!STUB! WTMgrDefContextEx({:#?}, {:#?}, {:#?})",
        mgr, device, system
    );
    0
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn WTMgrPacketHookDefProc(
    value1: i32,
    w: WPARAM,
    l: LPARAM,
    hook: *mut c_void,
) -> *mut c_void {
    debug!(
        "!STUB! WTMgrPacketHookDefProc({:#?}, {:#?}, {:#?}, {:#?})",
        value1, w, l, hook
    );
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn WTInfoA(
    w_category: u32,
    n_index: u32,
    lp_output: *mut c_void,
) -> u32 {
    debug!("WTInfoA({}, {}, {:#?});", w_category, n_index, lp_output);
    unsafe { WTInfo(w_category, n_index, lp_output) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn WTInfoW(
    w_category: u32,
    n_index: u32,
    lp_output: *mut c_void,
) -> u32 {
    // TODO: THERE IS A SEGFAULT HAPPENING (only in wtinfo.exe as far as i can tell)
    // THIS info! IS THE FIX (or RUST_LOG=debug)
    // WHAT
    // info!("WTInfoW({}, {}, {:#?});", w_category, n_index, lp_output);
    debug!("WTInfoW({}, {}, {:#?});", w_category, n_index, lp_output);
    unsafe { WTInfo(w_category, n_index, lp_output) }
}
// This function returns global information about the interface in an application-supplied buffer.
// Different types of information are specified by different index arguments.
// Applications use this function to receive information about tablet coordinates,
// physical dimensions, capabilities, and cursor types.
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn WTInfo(
    w_category: u32,
    n_index: u32,
    lp_output: *mut c_void,
) -> u32 {
    debug!("WTInfo({}, {}, {:#?});", w_category, n_index, lp_output);

    unsafe {
        // Some categories are multiplexed.
        // A single category code represents the first of a group of identically indexed categories,
        // one for each of a set of similar objects. Multiplexed categories include those for devices
        // and cursor types. One constructs the category number by adding the defined category code
        // to a zero-based device or cursor identification number.
        match w_category {
            // If the wCategory argument is zero, the function copies no data to the output buffer,
            // but returns the size in bytes of the buffer necessary to hold the largest complete category.
            0 => 8192, // should fit anything asked for; wacom driver returns 8790
            WTI_INTERFACE => WtiInterface::psm_default().handle_info(n_index, lp_output),

            WTI_DEFCONTEXT => handle_default_context_info(n_index, lp_output, false),
            WTI_DEFSYSCTX => handle_default_context_info(n_index, lp_output, true),
            WTI_DDCTXS..WTI_DDCTXS_MAX => handle_default_context_info(n_index, lp_output, false),
            WTI_DSCTXS..WTI_DSCTXS_MAX => handle_default_context_info(n_index, lp_output, true),

            // pass (w_category - WTI_{MULTIPLEXED_CATEGORY}) as the device/cursor/DDCTXS/DSCTXS index
            // when implementing multiple device/cursor support
            WTI_DEVICES..WTI_DEVICES_MAX => handle_device_info(n_index, lp_output),
            WTI_CURSORS..WTI_CURSORS_MAX => handle_cursor_info(n_index, lp_output),

            // TODO: implement
            // WTI_STATUS => todo!(),
            // WTI_EXTENSIONS..WTI_EXTENSIONS_MAX => todo!(),
            _ => 0,
        }
    }
}

pub unsafe fn handle_default_context_info(index: u32, lp_output: *mut c_void, system: bool) -> u32 {
    let mut state = get_state_or_init().unwrap();
    let state = state.as_mut().unwrap();
    state.default_context.options = if system { CXO_SYSTEM } else { 0 };
    // state.default_context.options |= CXO_MESSAGES;
    unsafe { state.default_context.handle_info(index, lp_output) }
}

pub unsafe fn handle_device_info(index: u32, lp_output: *mut c_void) -> u32 {
    let state = get_state_or_init().unwrap();
    let state = state.as_ref().unwrap();
    unsafe { state.device.handle_info(index, lp_output) }
}

pub unsafe fn handle_cursor_info(index: u32, lp_output: *mut c_void) -> u32 {
    let state = get_state_or_init().unwrap();
    let state = state.as_ref().unwrap();
    unsafe { state.cursor.handle_info(index, lp_output) }
}
