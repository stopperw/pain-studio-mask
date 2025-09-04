use std::{io::Write, net::TcpStream};

use color_eyre::eyre::Context;
use log::error;
use psm_common::netcode::*;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    status: u32,
    buttons: u32,
    x: u32,
    y: u32,
    z: u32,
    normal_pressure: u32,
    tangential_pressure: u32
}

fn main() {
    colog::init();
    color_eyre::install().ok();
    match fmain() {
        Ok(_) => {},
        Err(err) => error!("{:?}", err)
    }
}

fn fmain() -> color_eyre::Result<()> {
    let args = Args::parse();
    let mut stream = TcpStream::connect("127.0.0.1:40302").wrap_err("client connection failed")?;
    send_packet(&mut stream, &PSMPacketC2S::Hi { name: "test_client 0.1.0".to_string() })?;
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
    send_packet(&mut stream, &PSMPacketC2S::RawTabletPacket {
        status: args.status,
        buttons: args.buttons,
        x: args.x,
        y: args.y,
        z: args.z,
        normal_pressure: args.normal_pressure,
        tangential_pressure: args.tangential_pressure,
    })?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    for i in 0..4 {
        send_packet(&mut stream, &PSMPacketC2S::RawTabletPacket {
            status: args.status,
            buttons: args.buttons,
            x: args.x + i * 50,
            y: args.y,
            z: args.z,
            normal_pressure: args.normal_pressure,
            tangential_pressure: args.tangential_pressure,
        })?;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    send_packet(&mut stream, &PSMPacketC2S::RawTabletPacket {
        status: args.status,
        buttons: 0,
        x: args.x,
        y: args.y,
        z: args.z,
        normal_pressure: 0,
        tangential_pressure: args.tangential_pressure,
    })?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_packet(&mut stream, &PSMPacketC2S::RawTabletPacket {
        status: args.status,
        buttons: 0,
        x: args.x,
        y: args.y,
        z: 1020,
        normal_pressure: 0,
        tangential_pressure: args.tangential_pressure,
    })?;
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

