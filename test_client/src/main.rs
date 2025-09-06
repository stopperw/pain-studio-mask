use std::{io::Write, net::TcpStream};

use clap::Parser;
use color_eyre::eyre::Context;
use log::error;
use psm_common::netcode::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    status: u32,
    buttons: u32,
    x: u32,
    y: u32,
    z: u32,
    normal_pressure: u32,
    tangential_pressure: u32,
}

fn main() {
    colog::init();
    color_eyre::install().ok();
    match fmain() {
        Ok(_) => {}
        Err(err) => error!("{:?}", err),
    }
}

fn fmain() -> color_eyre::Result<()> {
    let args = Args::parse();
    let mut stream = TcpStream::connect("127.0.0.1:40302").wrap_err("client connection failed")?;
    send_packet(
        &mut stream,
        &PSMPacketC2S::Hi {
            name: "test_client 0.1.0".to_string(),
        },
    )?;
    // send_packet(
    //     &mut stream,
    //     &PSMPacketC2S::ConfigureDevice {
    //         hardware: 8 | 4,
    //         packet_rate: 100,
    //         packet_mode: 0,
    //         x_margin: 0,
    //         y_margin: 0,
    //         z_margin: 0,
    //         device_x: Axis {
    //             min: 0,
    //             max: 15199,
    //             units: 2,
    //             resolution: 0x03e8_0000,
    //         },
    //         device_y: Axis {
    //             min: 0,
    //             max: 9499,
    //             units: 2,
    //             resolution: 0x03e8_0000,
    //         },
    //         device_z: Axis {
    //             min: -1023,
    //             max: 1023,
    //             units: 2,
    //             resolution: 0x03e8_0000,
    //         },
    //         normal_pressure: Axis {
    //             min: 0,
    //             max: 32767,
    //             units: 0,
    //             resolution: 0,
    //         },
    //         tangential_pressure: Axis {
    //             min: 0,
    //             max: 1023,
    //             units: 0,
    //             resolution: 0,
    //         },
    //         // normal_pressure: Axis::default(),
    //         // tangential_pressure: Axis::default(),
    //         orientation: [Axis::default(), Axis::default(), Axis::default()],
    //         rotation: [Axis::default(), Axis::default(), Axis::default()],
    //     },
    // )?;

    // send_packet(
    //     &mut stream,
    //     &PSMPacketC2S::ConfigureContext {
    //         status: 0,
    //         packet_rate: 100,
    //         packet_mode: 0,
    //         move_mask: 0xFFFFFFFF,
    //         in_org_x: 0,
    //         in_org_y: 0,
    //         in_org_z: -1023,
    //         in_ext_x: 15200,
    //         in_ext_y: 9500,
    //         in_ext_z: 2047,
    //         out_org_x: 0,
    //         out_org_y: 0,
    //         out_org_z: -1023,
    //         out_ext_x: 15200,
    //         out_ext_y: 9500,
    //         out_ext_z: 2047,
    //         sys_org_x: 0,
    //         sys_org_y: 0,
    //         sys_ext_x: 1920,
    //         sys_ext_y: 1080,
    //     },
    // )?;
    // for x in 0..(2048 / 128) {
    // for y in 0..(2048 / 128) {
    //     send_packet(&mut stream, &PSMPacketC2S::Proximity { value: true })?;
    //     send_packet(&mut stream, &PSMPacketC2S::RawTabletPacket {
    //         status: args.status,
    //         buttons: args.buttons,
    //         x: x * 128,
    //         y: y * 128,
    //         z: args.z,
    //         normal_pressure: args.normal_pressure,
    //         tangential_pressure: args.tangential_pressure,
    //     })?;
    //     std::thread::sleep(std::time::Duration::from_millis(50));
    //     send_packet(&mut stream, &PSMPacketC2S::RawTabletPacket {
    //         status: args.status,
    //         buttons: 0,
    //         x: args.x,
    //         y: args.y,
    //         z: 1024,
    //         normal_pressure: 0,
    //         tangential_pressure: args.tangential_pressure,
    //     })?;
    //     send_packet(&mut stream, &PSMPacketC2S::Proximity { value: false })?;
    //     std::thread::sleep(std::time::Duration::from_millis(50));
    // }
    // }
    send_packet(&mut stream, &PSMPacketC2S::Proximity { value: true })?;
    send_packet(
        &mut stream,
        &PSMPacketC2S::TabletEvent {
            status: args.status,
            buttons: args.buttons,
            x: args.x,
            y: args.y,
            z: args.z,
            normal_pressure: args.normal_pressure,
            tangential_pressure: args.tangential_pressure,
        },
    )?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    for i in 0..4 {
        send_packet(
            &mut stream,
            &PSMPacketC2S::TabletEvent {
                status: args.status,
                buttons: args.buttons,
                x: args.x + i * 50,
                y: args.y,
                z: args.z,
                normal_pressure: args.normal_pressure,
                tangential_pressure: args.tangential_pressure,
            },
        )?;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    send_packet(
        &mut stream,
        &PSMPacketC2S::TabletEvent {
            status: args.status,
            buttons: 0,
            x: args.x,
            y: args.y,
            z: args.z,
            normal_pressure: 0,
            tangential_pressure: args.tangential_pressure,
        },
    )?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_packet(
        &mut stream,
        &PSMPacketC2S::TabletEvent {
            status: args.status,
            buttons: 0,
            x: args.x,
            y: args.y,
            z: 1020,
            normal_pressure: 0,
            tangential_pressure: args.tangential_pressure,
        },
    )?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_packet(&mut stream, &PSMPacketC2S::Proximity { value: false })?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    Ok(())
}

pub fn send_packet(stream: &mut impl Write, packet: &PSMPacketC2S) -> color_eyre::Result<()> {
    let data = serde_json::to_vec(packet)?;
    let bytes: [u8; 4] = (data.len() as u32).to_be_bytes();
    stream.write_all(&bytes)?;
    stream.write_all(&data)?;
    Ok(())
}
