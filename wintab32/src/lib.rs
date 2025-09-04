use std::{collections::{HashMap, VecDeque}, ffi::c_void, io::{Read, Write}, net::{TcpListener, TcpStream}, sync::{LazyLock, Mutex}};

use color_eyre::eyre::{bail, ContextCompat};
use log::{debug, error, info};
use static_init::{constructor, destructor};
use windows::{
    // core::*,
    Win32::{Foundation::{HWND, LPARAM, WPARAM}, UI::WindowsAndMessaging::*},
};

use crate::info_write::info_write;
use crate::ffi::*;
use psm_common::netcode::*;

pub mod info_write;
pub mod ffi;

static STATE: LazyLock<Mutex<PSM>> = LazyLock::new(|| Mutex::new(PSM::default()));
// static THREADS: LazyLock<Mutex<Vec<JoinHandle<()>>>> = LazyLock::new(|| Mutex::new(Vec::new()));
// static WE_SHOULD_DIE: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

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
    // WE_SHOULD_DIE.store(true, std::sync::atomic::Ordering::Relaxed);
    info!("bye!");
}

pub fn main() -> color_eyre::Result<()> {
    debug!("PSM debug");

    // TODO: TCP socket
    // TODO: config file
    info!("PSM is loaded!");

    // TODO: kill? threads
    std::thread::spawn(tcp_thread);
    // let _ptthread = std::thread::spawn(move || {
    //     loop {
    //         std::thread::sleep(std::time::Duration::from_secs(3));
    //         debug!("tick");
    //         // if WE_SHOULD_DIE.load(std::sync::atomic::Ordering::Relaxed) {
    //         //     break;
    //         // }
    //         let mut state = STATE.lock().unwrap();
    //         for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
    //             ctx.send_packet().ok();
    //         }
    //     }
    // });
    // THREADS.lock().unwrap().push(ptthread);

    Ok(())
}

pub fn tcp_thread() {
    let socket = TcpListener::bind("127.0.0.1:40302").unwrap();
    loop {
        info!("PSM is now listening on 127.0.0.1:40302");
        let stream = socket.accept();
        match stream {
            Ok((stream, addr)) => {
                info!("Accepted connection from {}", addr);
                match handle_client(stream) {
                    Ok(_) => {},
                    Err(err) => {
                        info!("{:?}", err);
                        info!("Connection from {} ended", addr);
                    },
                }
            },
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
                send_packet(&mut socket, &PSMPacketS2C::Hi { compatible: 1 })?;
            }
            PSMPacketC2S::RawTabletPacket { status, buttons, x, y, z, normal_pressure, tangential_pressure } => {
                let mut state = STATE.lock().unwrap();
                for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                    ctx.send_packet(Packet {
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
                    }).ok();
                }
            }
            PSMPacketC2S::Proximity { value } => {
                let mut state = STATE.lock().unwrap();
                for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                    ctx.proximity(value).ok();
                }
            }
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

#[derive(Default)]
pub struct PSM {
    pub contexts: HashMap<usize, Context>,
    pub counter: usize,
}

pub struct Context {
    pub handle: usize,
    pub enabled: bool,
    pub window: ThreadHWND,
    pub logical_context: WtiLogicalContext,
    pub packets: VecDeque<Packet>,
    pub queue_size: usize,
    pub serial: usize
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
            serial: 0
        }
    }

    pub fn send_packet(&mut self, mut packet: Packet) -> color_eyre::Result<()> {
        if !self.enabled {
            bail!("packet sent when context is disabled");
        }
        if self.window.0.0 == std::ptr::null_mut() {
            bail!("packet sent without a valid window");
        }
        self.serial += 1;
        packet.context = self.handle as u32;
        packet.serial = self.serial as u32;
        packet.orientation.altitude = 900;
        packet.time = self.serial as u32;
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
        // TODO: limit queue size by queue_size
        self.packets.push_back(packet);
        // posting WT_PACKET(serial, ctx_handle)
        unsafe { PostMessageW(Some(self.window.0), WindowMessage::Packet.value(self.logical_context.msg_base), WPARAM(self.serial), LPARAM(self.handle as isize))? };
        Ok(())
    }

    pub fn proximity(&mut self, value: bool) -> color_eyre::Result<()> {
        if !self.enabled {
            bail!("packet sent when context is disabled");
        }
        if self.window.0.0 == std::ptr::null_mut() {
            bail!("packet sent without a valid window");
        }
        // posting WT_PROXIMITY(ctx_handle, value)
        unsafe { PostMessageW(Some(self.window.0), WindowMessage::Proximity.value(self.logical_context.msg_base), WPARAM(self.handle), LPARAM(if value { 0x00010001 } else { 0 }))? };
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ThreadHWND(pub HWND);
unsafe impl Send for ThreadHWND {}
unsafe impl Sync for ThreadHWND {}

#[unsafe(no_mangle)]
pub extern "C" fn WTOpenA(hwnd: HWND, lp_log_ctx: *mut WtiLogicalContext, f_enable: bool) -> usize {
    debug!("WTOpenA({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    WTOpen(hwnd, lp_log_ctx, f_enable)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTOpenW(hwnd: HWND, lp_log_ctx: *mut WtiLogicalContext, f_enable: bool) -> usize {
    debug!("WTOpenW({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    WTOpen(hwnd, lp_log_ctx, f_enable)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTOpen(hwnd: HWND, lp_log_ctx: *mut WtiLogicalContext, f_enable: bool) -> usize {
    debug!("WTOpen({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
    if lp_log_ctx == std::ptr::null_mut() {
        error!("WTOpen lp_log_ctx is null");
        return 0;
    }
    unsafe {
        debug!("LogContext -> {:#?}", *lp_log_ctx);
        // std::ptr::copy(lp_log_ctx, &mut logical_context, 1);
    }

    let mut state = STATE.lock().unwrap();
    state.counter += 1;

    let handle = state.counter;

    let mut context = Context::new(handle, f_enable);
    context.window = ThreadHWND(hwnd);
    unsafe {
        std::ptr::copy(lp_log_ctx, &mut context.logical_context, 1);
    }
    state.contexts.insert(handle, context);
    debug!("new context registered at {} (enabled = {})", handle, f_enable);

    handle
}

#[unsafe(no_mangle)]
pub extern "C" fn WTEnable(ctx_id: usize, enable: bool) -> bool {
    debug!("WTEnable({:#?}, {})", ctx_id, enable);
    let mut state = STATE.lock().unwrap();
    let ctx = match state.contexts.get_mut(&ctx_id) {
        Some(ctx) => ctx,
        None => return false
    };
    ctx.enabled = enable;
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn WTPacketsGet(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> u32 {
    debug!("WTPacketsGet({:#?}, {:#?}, {:#?})", ctx_id, max_packets, ptr);
    match packets_get(ctx_id, max_packets, ptr) {
        Ok(v) => v,
        Err(err) => {
            error!("WTPacketsGet({:#?}, {:#?}, {:#?}) failed!", ctx_id, max_packets, ptr);
            error!("{:?}", err);
            0
        }
    }
}
pub fn packets_get(ctx_id: usize, max_packets: i32, ptr: *mut c_void) -> color_eyre::Result<u32> {
    let mut state = STATE.lock().unwrap();
    let ctx = state.contexts.get_mut(&ctx_id).wrap_err("context not found")?;
    let mut count = 0;
    for i in 0..max_packets {
        let packet = match ctx.packets.pop_front() {
            Some(x) => x,
            None => break
        };
        // TODO: FIXME: ooooh scary pointer arithmetics.
        let packet_size = size_of::<Packet>();
        info_write(&packet, ptr.wrapping_add(packet_size * (i as usize)));
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
    let ctx = state.contexts.get_mut(&ctx_id).wrap_err("context not found")?;
    ctx.packets.retain_mut(|x| x.serial >= serial);
    let packet = match ctx.packets.iter().find(|x| x.serial == serial) {
        Some(x) => x,
        None => return Ok(false)
    };
    info_write(&packet, ptr);
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
pub extern "C" fn WTPacketsPeek(ctx_id: usize, ext: u32, ptr: *mut c_void) -> i32 {
    debug!("!STUB! WTPacketsPeek({:#?}, {:#?}, {:#?})", ctx_id, ext, ptr);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn WTDataGet(ctx_id: usize, begin: u32, end: u32, max_packets: i32, ptr: *mut c_void, ints: *mut c_void) -> i32 {
    debug!("!STUB! WTDataGet({:#?}, {:#?}, {:#?}, {:#?}, {:#?}, {:#?})", ctx_id, begin, end, max_packets, ptr, ints);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn WTDataPeek(ctx_id: usize, begin: u32, end: u32, max_packets: i32, ptr: *mut c_void, ints: *mut c_void) -> i32 {
    debug!("!STUB! WTDataPeek({:#?}, {:#?}, {:#?}, {:#?}, {:#?}, {:#?})", ctx_id, begin, end, max_packets, ptr, ints);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn WTQueuePacketsEx(ctx_id: usize, old: *mut c_void, new: *mut c_void) -> bool {
    debug!("!STUB! WTQueuePacketsEx({:#?}, {:#?}, {:#?})", ctx_id, old, new);
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
    let ctx = state.contexts.get_mut(&ctx_id).wrap_err("context not found")?;
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
    let ctx = state.contexts.get_mut(&ctx_id).wrap_err("context not found")?;
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
    debug!("!STUB! WTMgrDefContextEx({:#?}, {:#?}, {:#?})", mgr, device, system);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn WTMgrPacketHookDefProc(value1: i32, w: WPARAM, l: LPARAM, hook: *mut c_void) -> *mut c_void {
    debug!("!STUB! WTMgrPacketHookDefProc({:#?}, {:#?}, {:#?}, {:#?})", value1, w, l, hook);
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn WTInfoA(w_category: u32, n_index: u32, lp_output: *mut c_void) -> u32 {
    debug!("WTInfoA({}, {}, {:#?});", w_category, n_index, lp_output);
    WTInfo(w_category, n_index, lp_output)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTInfoW(w_category: u32, n_index: u32, lp_output: *mut c_void) -> u32 {
    debug!("WTInfoW({}, {}, {:#?});", w_category, n_index, lp_output);
    WTInfo(w_category, n_index, lp_output)
}
#[unsafe(no_mangle)]
pub extern "C" fn WTInfo(w_category: u32, n_index: u32, lp_output: *mut c_void) -> u32 {
    debug!("WTInfo({}, {}, {:#?});", w_category, n_index, lp_output);

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
        // WTI_DDCTXS => todo!(),
        WTI_DSCTXS => handle_logctx(n_index, lp_output, true),
        // WTI_DSCTXS => todo!(),
        // _ => unreachable!()
        _ => 0
    }
}

pub fn handle_logctx(index: u32, lp_output: *mut c_void, system: bool) -> u32 {
    let mut ctx = WtiLogicalContext::psm_default();
    // ctx.status = CXS_ONTOP;
    if system {
        ctx.options = CXO_SYSTEM;
    }
    ctx.pkt_data = PK_CONTEXT | PK_STATUS | PK_TIME | PK_CHANGED | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_Z | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION | PK_ROTATION;
    ctx.move_mask = PK_CONTEXT | PK_STATUS | PK_TIME | PK_CHANGED | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_Z | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION | PK_ROTATION;
    // ctx.move_mask = PK_BUTTONS | PK_X | PK_Y | PK_Z | PK_NORMAL_PRESSURE | PK_ORIENTATION;
    ctx.btn_dn_mask = 0xFFFFFFFF;
    ctx.btn_up_mask = 0xFFFFFFFF;

    ctx.in_org_z = -1023;
    ctx.in_ext_x  = 15200;
    ctx.in_ext_y  = 9500;
    ctx.in_ext_z  = 2047;
    ctx.out_org_z = -1023;
    ctx.out_ext_x = 15200;
    ctx.out_ext_y = 9500;
    ctx.out_ext_z = 2047;
    // ctx.in_ext_x = 2048;
    // ctx.in_ext_y = 2048;
    // ctx.in_ext_z = 2048;
    // ctx.out_ext_x = 2048;
    // ctx.out_ext_y = 2048;
    // ctx.out_ext_z = 2048;
    ctx.out_sens_x = 0x00010000;
    ctx.out_sens_y = 0x00010000;
    ctx.out_sens_z = 0x00010000;
    ctx.sys_ext_x = 1920;
    ctx.sys_ext_y = 1080;
    ctx.sys_sens_x = 0x00010000;
    ctx.sys_sens_y = 0x00010000;
    ctx.handle_info(index, lp_output)
}

pub fn handle_device(index: u32, lp_output: *mut c_void) -> u32 {
    let mut ctx = WtiDevices::psm_default();
    ctx.hardware = HWC_HARDPROX | HWC_PHYSID_CURSORS;
    ctx.packet_data = PK_CONTEXT | PK_STATUS | PK_TIME | PK_CHANGED | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_Z | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION | PK_ROTATION;

    // DVC_X                          { 16} { 16} {min=0;max=15199;units=2 [CENTIMETERS];res=0x03e80000 [1000.0]}
    // DVC_Y                          { 16} { 16} {min=0;max=9499;units=2 [CENTIMETERS];res=0x03e80000 [1000.0]}
    // DVC_Z                          { 16} { 16} {min=-1023;max=1023;units=2 [CENTIMETERS];res=0x03e80000 [1000.0]}
    ctx.device_x.max = 15199;
    ctx.device_x.units = TU_CENTIMETERS;
    ctx.device_x.resolution = 0x03e80000;
    ctx.device_y.max = 9499;
    ctx.device_y.units = TU_CENTIMETERS;
    ctx.device_y.resolution = 0x03e80000;
    ctx.device_z.min = -1023;
    ctx.device_z.max = 1023;
    ctx.device_z.units = TU_CENTIMETERS;
    ctx.device_z.resolution = 0x03e80000;

    // ctx.device_x.max = 2048;
    // // ctx.device_x.resolution = 0x00010000;
    // ctx.device_x.resolution = 0x03e80000;
    // ctx.device_y.max = 2048;
    // // ctx.device_y.resolution = 0x00010000;
    // ctx.device_y.resolution = 0x03e80000;
    // ctx.device_z.max = 2048;
    // // ctx.device_z.resolution = 0x00010000;
    // ctx.device_z.resolution = 0x03e80000;
    ctx.normal_pressure.max = 32767;
    ctx.normal_pressure.units = TU_NONE;
    ctx.normal_pressure.resolution = 0;
    // ctx.tangential_pressure.max = 32767;
    ctx.tangential_pressure.max = 1023;
    ctx.tangential_pressure.units = TU_NONE;
    ctx.tangential_pressure.resolution = 0;
    ctx.orientation = [
        Axis {
            min: 0,
            max: 3600,
            units: TU_CIRCLE,
            resolution: 0x0e100000
        },
        Axis {
            min: -1000,
            max: 1000,
            units: TU_CIRCLE,
            resolution: 0x0e100000
        },
        Axis {
            min: 0,
            max: 3600,
            units: TU_CIRCLE,
            resolution: 0x0e100000
        },
    ];
    ctx.handle_info(index, lp_output)
}

pub fn handle_cursor(index: u32, lp_output: *mut c_void) -> u32 {
    let mut ctx = WtiCursors::psm_default();
    ctx.packet_data = PK_CONTEXT | PK_STATUS | PK_TIME | PK_CHANGED | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_Z | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION | PK_ROTATION;
    ctx.buttons = 3;
    ctx.button_bits = 3;
    ctx.physical_button = 0;
    ctx.tangential_button = 1;
    // ctx.buttons = 16;
    // ctx.button_bits = 16;
    // ctx.physical_button = 0;
    // ctx.tangential_button = 1;
    // ctx.capabilities = CRC_MULTIMODE;
    ctx.handle_info(index, lp_output)
}



