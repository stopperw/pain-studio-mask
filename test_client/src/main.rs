use std::{io::Write, net::TcpStream};

use clap::Parser;
use color_eyre::eyre::Context;
use log::{error, info};
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
    for i in 0..8 {
        for j in 0..8 {
            send_packet(
                &mut stream,
                &PSMPacketC2S::TabletEvent {
                    status: args.status,
                    buttons: args.buttons,
                    x: args.x + i * 50,
                    y: args.y + j * 50,
                    z: args.z,
                    normal_pressure: args.normal_pressure,
                    tangential_pressure: args.tangential_pressure,
                },
            )?;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
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
    let datas = serde_json::to_string(packet)?;
    info!("{}", datas);
    let data = serde_json::to_vec(packet)?;
    let bytes: [u8; 4] = (data.len() as u32).to_be_bytes();
    stream.write_all(&bytes)?;
    stream.write_all(&data)?;
    Ok(())
}
