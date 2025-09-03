use std::{collections::{HashMap, VecDeque}, ffi::c_void, sync::{atomic::AtomicBool, LazyLock, Mutex}, thread::JoinHandle};

use color_eyre::eyre::bail;
use log::{debug, error, info};
use static_init::{constructor, destructor};
use windows::{
    core::*,
    Win32::{Foundation::{HWND, LPARAM, WPARAM}, UI::WindowsAndMessaging::*},
};

use crate::info_write::{info_write, info_write_array};
use crate::ffi::*;

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
    info!("PSM is now listening on 127.0.0.1:40302");

    let ptthread = std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            // if WE_SHOULD_DIE.load(std::sync::atomic::Ordering::Relaxed) {
            //     break;
            // }
            let mut state = STATE.lock().unwrap();
            for (_, ctx) in state.contexts.iter_mut().filter(|(_, x)| x.enabled) {
                ctx.send_packet().ok();
            }
        }
    });
    // THREADS.lock().unwrap().push(ptthread);

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
            serial: 0
        }
    }

    pub fn send_packet(&mut self) -> color_eyre::Result<()> {
        if !self.enabled {
            bail!("packet sent when context is disabled");
        }
        if self.window.0.0 == std::ptr::null_mut() {
            bail!("packet sent without a valid window");
        }
        self.serial += 1;
        let packet = Packet {
            context: self.handle,
            status: 0,
            time: 1,
            changed: 0,
            serial: self.serial as u32,
            cursor: 2,
            buttons: 3,
            x: 4,
            y: 5,
            z: 6,
            normal_pressure: 0,
            tangential_pressure: 0,
            orientation: Orientation::default(),
            rotation: Rotation::default(),
        };
        self.packets.push_back(packet);
        unsafe { PostMessageW(Some(self.window.0), self.logical_context.msg_base, WPARAM(self.serial), LPARAM(self.handle as isize))? };
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ThreadHWND(pub HWND);
unsafe impl Send for ThreadHWND {}
unsafe impl Sync for ThreadHWND {}

pub struct PSMPacket {}

#[unsafe(no_mangle)]
pub extern "C" fn WTOpenW(hwnd: HWND, lp_log_ctx: *mut WtiLogicalContext, f_enable: bool) -> usize {
    debug!("WTOpenW({:#?}, {:#?}, {})", hwnd, lp_log_ctx, f_enable);
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
    let mut state = STATE.lock().unwrap();
    let ctx = match state.contexts.get_mut(&ctx_id) {
        Some(ctx) => ctx,
        None => return 0
    };
    let mut count = 0;
    for i in 0..max_packets {
        let packet = match ctx.packets.pop_front() {
            Some(x) => x,
            None => break
        };
        // FIXME: ooooh scary pointer arithmetics.
        let packet_size = size_of::<Packet>();
        unsafe { info_write(&packet, ptr.wrapping_add(packet_size * (i as usize))); }
        count += 1;
    }

    // unsafe {
    //     std::ptr::copy(slice, ptr, count);
    // }
    // info_write_array(slice, ptr, count);

    count as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn WTPacket(ctx_id: usize, serial: u32, ptr: *mut c_void) -> bool {
    debug!("!STUB! WTPacket({:#?}, {:#?}, {:#?})", ctx_id, serial, ptr);
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn WTInfoW(w_category: u32, n_index: u32, lp_output: *mut c_void) -> u32 {
    debug!("WTInfoW({}, {}, {:#?});", w_category, n_index, lp_output);


    match w_category {
        // If the wCategory argument is zero, the function copies no data to the output buffer,
        // but returns the size in bytes of the buffer necessary to hold the largest complete category. 
        0 => size_of::<WtiLogicalContext>() as u32,
        WTI_INTERFACE => WtiInterface::psm_default().handle_info(n_index, lp_output),

        // WTI_DEFCONTEXT => WtiLogicalContext::psm_default().handle_info(n_index, lp_output),
        // WTI_DEFSYSCTX => WtiLogicalContext::psm_default().handle_info(n_index, lp_output),
        WTI_DEFCONTEXT => handle_logctx(n_index, lp_output),
        WTI_DEFSYSCTX => handle_logctx(n_index, lp_output),
        // WTI_STATUS => todo!(),
        // WTI_DEFCONTEXT => todo!(),
        // WTI_DEFSYSCTX => todo!(),
        // WTI_DEVICES => WtiDevices::psm_default().handle_info(n_index, lp_output),
        WTI_DEVICES => handle_device(n_index, lp_output),
        // WTI_CURSORS => WtiCursors::psm_default().handle_info(n_index, lp_output),
        WTI_CURSORS => handle_cursor(n_index, lp_output),
        // WTI_DEVICES => todo!(),
        // WTI_CURSORS => todo!(),
        // WTI_EXTENSIONS => todo!(),
        // WTI_DDCTXS => todo!(),
        // WTI_DSCTXS => todo!(),
        // _ => unreachable!()
        _ => 0
    }
}

pub fn handle_logctx(index: u32, lp_output: *mut c_void) -> u32 {
    let mut ctx = WtiLogicalContext::psm_default();
    ctx.status = CXS_ONTOP;
    ctx.options = CXO_SYSTEM;
    ctx.pkt_data = PK_CONTEXT | PK_STATUS | PK_TIME | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_NORMAL_PRESSURE | PK_ORIENTATION;
    // ctx.pkt_data = PK_CONTEXT | PK_CHANGED | PK_STATUS | PK_TIME | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION;
    ctx.move_mask = PK_BUTTONS | PK_X | PK_Y | PK_NORMAL_PRESSURE | PK_ORIENTATION;
    ctx.btn_dn_mask = 0xFFFFFFFF;
    ctx.btn_up_mask = 0xFFFFFFFF;
    ctx.in_ext_x = 262143;
    ctx.in_ext_y = 262143;
    ctx.out_ext_x = 262143;
    ctx.out_ext_y = 262143;
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
    ctx.packet_data = PK_CONTEXT | PK_STATUS | PK_TIME | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_NORMAL_PRESSURE | PK_ORIENTATION;
    // ctx.packet_data = PK_CONTEXT | PK_CHANGED | PK_STATUS | PK_TIME | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION;
    ctx.device_x.max = 262143;
    ctx.device_x.resolution = 0x00002710;
    ctx.device_y.max = 262143;
    ctx.device_y.resolution = 0x00002710;
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
    ctx.packet_data = PK_TIME | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION;
    // ctx.packet_data = PK_CONTEXT | PK_CHANGED | PK_STATUS | PK_TIME | PK_SERIAL_NUMBER | PK_CURSOR | PK_BUTTONS | PK_X | PK_Y | PK_NORMAL_PRESSURE | PK_TANGENT_PRESSURE | PK_ORIENTATION;
    ctx.buttons = 9;
    ctx.physical_button = 1;
    ctx.tangential_button = 1;
    ctx.capabilities = CRC_MULTIMODE;
    ctx.handle_info(index, lp_output)
}



