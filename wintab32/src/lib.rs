use std::{
    collections::{HashMap, VecDeque},
    ffi::c_void,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{LazyLock, Mutex},
    time::Instant,
};

use color_eyre::eyre::{ContextCompat, bail};
use log::{debug, error, info};
use static_init::{constructor, destructor};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::*,
};

use crate::{config::Config, ffi::*};
use psm_common::netcode::{PSMPacketC2S, PSMPacketS2C, COMPATIBLE_VERSION};

pub mod config;
pub mod ffi;
pub mod info_write;
pub mod netcompat;
pub mod ptr;

static STATE: LazyLock<Mutex<Option<PSM>>> = LazyLock::new(|| Mutex::new(None));

#[constructor(0)]
extern "C" fn init_main() {
    colog::init();
    color_eyre::install().ok();
    if let Err(err) = main() {
        error!("{:?}", err);
        error!("PSM's main thread failed! It's recommended to restart the app.");
    }
}

#[destructor(0)]
extern "C" fn free_main() {
    info!("bye!");
}

pub fn main() -> color_eyre::Result<()> {
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
                send_packet(&mut socket, &PSMPacketS2C::Hi { compatible: COMPATIBLE_VERSION })?;
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
                let mut state = STATE.lock().unwrap();
                let state = state.as_mut().unwrap();
                for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                    if let Err(err) = ctx.send_packet(Packet {
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
                let mut state = STATE.lock().unwrap();
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
                let mut state = STATE.lock().unwrap();
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
                let mut state = STATE.lock().unwrap();
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
        self.default_context.status = self.config.preset.status;
        self.default_context.packet_rate = self.config.preset.packet_rate;
        self.default_context.packet_mode = self.config.preset.packet_mode;
        self.default_context.move_mask = self.config.preset.move_mask;
        self.default_context.in_org_x = self.config.preset.in_org_x;
        self.default_context.in_org_y = self.config.preset.in_org_y;
        self.default_context.in_org_z = self.config.preset.in_org_z;
        self.default_context.in_ext_x = self.config.preset.in_ext_x;
        self.default_context.in_ext_y = self.config.preset.in_ext_y;
        self.default_context.in_ext_z = self.config.preset.in_ext_z;
        self.default_context.out_org_x = self.config.preset.out_org_x;
        self.default_context.out_org_y = self.config.preset.out_org_y;
        self.default_context.out_org_z = self.config.preset.out_org_z;
        self.default_context.out_ext_x = self.config.preset.out_ext_x;
        self.default_context.out_ext_y = self.config.preset.out_ext_y;
        self.default_context.out_ext_z = self.config.preset.out_ext_z;
        self.default_context.sys_org_x = self.config.preset.sys_org_x;
        self.default_context.sys_org_y = self.config.preset.sys_org_y;
        self.default_context.sys_ext_x = self.config.preset.sys_ext_x;
        self.default_context.sys_ext_y = self.config.preset.sys_ext_y;
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
    }
}

pub struct Context {
    pub handle: usize,
    pub enabled: bool,
    pub window: ThreadHWND,
    pub logical_context: WtiLogicalContext,
    pub packets: VecDeque<Packet>,
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
            queue_size: 1024,
            serial: 0,
            time: Instant::now(),
        }
    }

    pub fn send_packet(&mut self, mut packet: Packet) -> color_eyre::Result<()> {
        if !self.enabled {
            bail!("packet sent when context is disabled");
        }
        if self.window.0.0.is_null() {
            bail!("packet sent without a valid window");
        }
        self.serial += 1;
        packet.context = self.handle as u32;
        packet.serial = self.serial as u32;
        packet.orientation.altitude = 900;
        packet.time = self.time.elapsed().as_millis() as u32;
        debug!("wtpacket: {:?}", packet);
        // limiting by queue size
        let queue_size = self.queue_size.max(1) as isize;
        for _overflow in 0..((self.packets.len() as isize) - queue_size + 1) {
            self.packets.pop_front();
        }
        self.packets.push_back(packet);
        // posting WT_PACKET(serial, ctx_handle)
        unsafe {
            PostMessageW(
                Some(self.window.0),
                WindowMessage::Packet.value(self.logical_context.msg_base),
                WPARAM(self.serial),
                LPARAM(self.handle as isize),
            )?
        };
        Ok(())
    }

    pub fn context_update(&mut self) -> color_eyre::Result<()> {
        if self.window.0.0.is_null() {
            bail!("update sent without a valid window");
        }
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
}

#[derive(Debug, Default)]
pub struct ThreadHWND(pub HWND);
unsafe impl Send for ThreadHWND {}
unsafe impl Sync for ThreadHWND {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn WTOpenA(
    hwnd: HWND,
    lp_log_ctx: *mut WtiLogicalContext,
    f_enable: bool,
) -> usize {
    debug!("WTOpenA({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    unsafe { WTOpen(hwnd, lp_log_ctx, f_enable) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WTOpenW(
    hwnd: HWND,
    lp_log_ctx: *mut WtiLogicalContext,
    f_enable: bool,
) -> usize {
    debug!("WTOpenW({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    unsafe { WTOpen(hwnd, lp_log_ctx, f_enable) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WTOpen(
    hwnd: HWND,
    lp_log_ctx: *mut WtiLogicalContext,
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

    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    state.counter += 1;

    let handle = state.counter;

    let mut context = Context::new(handle, f_enable);
    context.window = ThreadHWND(hwnd);
    unsafe {
        std::ptr::copy(lp_log_ctx, &mut context.logical_context, 1);
    }
    state.contexts.insert(handle, context);
    debug!(
        "new context registered at {} (enabled = {})",
        handle, f_enable
    );

    handle
}

#[unsafe(no_mangle)]
pub extern "C" fn WTEnable(ctx_id: usize, enable: bool) -> bool {
    debug!("WTEnable({:#?}, {})", ctx_id, enable);
    let mut state = STATE.lock().unwrap();
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
pub extern "C" fn WTPacketsGet(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> u32 {
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
    let mut state = STATE.lock().unwrap();
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
pub extern "C" fn WTPacketsPeek(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> u32 {
// pub extern "C" fn WTPacketsPeek(ctx_id: usize, ext: u32, ptr: *mut c_void) -> i32 {
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
    let mut state = STATE.lock().unwrap();
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
    for i in 0..max_packets {
        let packet = match packets.next() {
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

#[unsafe(no_mangle)]
pub extern "C" fn WTPacket(ctx_id: usize, serial: u32, ptr: *mut c_void) -> bool {
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
    let mut state = STATE.lock().unwrap();
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
pub extern "C" fn WTOverlap(ctx_id: usize, overlap: bool) -> bool {
    debug!("!STUB! WTOverlap({:#?}, {:#?}) -> true", ctx_id, overlap);
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn WTClose(ctx_id: usize) -> bool {
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
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    state.contexts.retain(|i, _| *i != ctx_id);
    Ok(true)
}

#[unsafe(no_mangle)]
pub extern "C" fn WTGetA(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTGetA({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTGetW(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTGetW({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTGet(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTGet({:#?}, {:#?})", ctx_id, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTSetA(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSetA({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTSetW(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSetW({:#?}, {:#?})", ctx_id, ptr);
    WTGet(ctx_id, ptr)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTSet(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSet({:#?}, {:#?})", ctx_id, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTExtGet(ctx_id: usize, ext: u32, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTExtGet({:#?}, {:#?}, {:#?})", ctx_id, ext, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTExtSet(ctx_id: usize, ext: u32, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTExtSet({:#?}, {:#?}, {:#?})", ctx_id, ext, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTSave(ctx_id: usize, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTSave({:#?}, {:#?})", ctx_id, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTRestore(hwnd: HWND, ptr: *mut c_void, value: bool) -> usize {
    debug!("!STUB! WTRestore({:#?}, {:#?}, {:#?})", hwnd, ptr, value);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn WTDataGet(
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
pub extern "C" fn WTDataPeek(
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
pub extern "C" fn WTQueuePacketsEx(ctx_id: usize, old: *mut c_void, new: *mut c_void) -> bool {
    debug!(
        "!STUB! WTQueuePacketsEx({:#?}, {:#?}, {:#?})",
        ctx_id, old, new
    );
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTQueueSizeGet(ctx_id: usize) -> u32 {
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
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    Ok(ctx.queue_size as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn WTQueueSizeSet(ctx_id: usize, num_packets: u32) -> bool {
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
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    let ctx = state
        .contexts
        .get_mut(&ctx_id)
        .wrap_err("context not found")?;
    ctx.queue_size = num_packets as usize;
    Ok(true)
}

#[unsafe(no_mangle)]
pub extern "C" fn WTMgrOpen(hwnd: HWND, msg_base: u32) -> usize {
    debug!("!STUB! WTMgrOpen({:#?}, {:#?})", hwnd, msg_base);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn WTMgrExt(mgr: usize, value: u32, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTMgrExt({:#?}, {:#?}, {:#?})", mgr, value, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTMgrClose(mgr: usize) -> bool {
    debug!("!STUB! WTMgrClose({:#?})", mgr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTMgrDefContextEx(mgr: usize, device: u32, system: bool) -> usize {
    debug!(
        "!STUB! WTMgrDefContextEx({:#?}, {:#?}, {:#?})",
        mgr, device, system
    );
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn WTMgrPacketHookDefProc(
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
pub unsafe extern "C" fn WTInfoA(w_category: u32, n_index: u32, lp_output: *mut c_void) -> u32 {
    debug!("WTInfoA({}, {}, {:#?});", w_category, n_index, lp_output);
    unsafe { WTInfo(w_category, n_index, lp_output) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WTInfoW(w_category: u32, n_index: u32, lp_output: *mut c_void) -> u32 {
    // TODO: THERE IS A SEGFAULT HAPPENING (only in wtinfo.exe as far as i can tell)
    // THIS info! IS THE FIX (or RUST_LOG=debug)
    // WHAT
    // info!("WTInfoW({}, {}, {:#?});", w_category, n_index, lp_output);
    debug!("WTInfoW({}, {}, {:#?});", w_category, n_index, lp_output);
    unsafe { WTInfo(w_category, n_index, lp_output) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn WTInfo(w_category: u32, n_index: u32, lp_output: *mut c_void) -> u32 {
    debug!("WTInfo({}, {}, {:#?});", w_category, n_index, lp_output);

    unsafe {
        match w_category {
            // If the wCategory argument is zero, the function copies no data to the output buffer,
            // but returns the size in bytes of the buffer necessary to hold the largest complete category.
            0 => size_of::<WtiLogicalContext>() as u32,
            WTI_INTERFACE => WtiInterface::psm_default().handle_info(n_index, lp_output),

            WTI_DEFCONTEXT => handle_logctx(n_index, lp_output, false),
            WTI_DEFSYSCTX => handle_logctx(n_index, lp_output, true),
            // WTI_STATUS => todo!(),
            WTI_DEVICES => handle_device(n_index, lp_output),
            WTI_CURSORS => handle_cursor(n_index, lp_output),
            // WTI_EXTENSIONS => todo!(),
            WTI_DDCTXS => handle_logctx(n_index, lp_output, false),
            WTI_DSCTXS => handle_logctx(n_index, lp_output, true),
            _ => 0,
        }
    }
}

pub unsafe fn handle_logctx(index: u32, lp_output: *mut c_void, system: bool) -> u32 {
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    state.default_context.options = if system { CXO_SYSTEM } else { 0 };
    unsafe { state.default_context.handle_info(index, lp_output) }
}

pub unsafe fn handle_device(index: u32, lp_output: *mut c_void) -> u32 {
    let state = STATE.lock().unwrap();
    let state = state.as_ref().unwrap();
    unsafe { state.device.handle_info(index, lp_output) }
}

pub unsafe fn handle_cursor(index: u32, lp_output: *mut c_void) -> u32 {
    let state = STATE.lock().unwrap();
    let state = state.as_ref().unwrap();
    unsafe { state.cursor.handle_info(index, lp_output) }
}
