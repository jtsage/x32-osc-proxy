// X32 OSC Proxy
// MIT License
//
// Copyright (c) 2025 J.T.Sage
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
use std::net::SocketAddr;
use std::sync::{Mutex, Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::net::UdpSocket;
use tauri::{Emitter, Manager};
use x32_osc_state::{X32Console, X32ProcessResult, x32::ConsoleRequest, osc::{Packet, Buffer, Bundle, Message}, enums};

// MARK: X32 -> MAIN
/// X32 UPD Listener
pub async fn x32_listener(socket : Arc<UdpSocket>, handle: tauri::AppHandle) -> ! {
    let mut buf = [0; 1024];
    loop {
        let (_len, _addr) = socket.recv_from(&mut buf).await.expect("broken socket");

        {
            let buffer = Buffer::from(buf.clone().to_vec());

            let time = handle.state::<Mutex<SystemTime>>();
            *time.lock().expect("lock failed") = SystemTime::now();

            let x32_result:X32ProcessResult;
            {
                let x32 = handle.state::<RwLock<X32Console>>();
                let mut x32_state = x32.write().expect("lock failed");
                x32_result = x32_state.process(buffer);
            }
            match x32_result {
                X32ProcessResult::NoOperation => (),
                X32ProcessResult::Meters(v) => {
                    match v {
                        (0, _) => {
                            handle.emit("meter-update-0", v.1).unwrap();
                        },
                        (5, _) => {
                            handle.emit("meter-update-5", v.1).unwrap();
                        }
                        _ => ()
                    }
                }
                X32ProcessResult::Fader(fader) => {
                    handle.emit("single-fader", fader).unwrap();
                },
                X32ProcessResult::CurrentCue(s) => {
                    handle.emit("cue-current", s).unwrap();
                },
            }
        }
    }
}

// MARK: MAIN -> X32
/// X32 Update loop
pub async fn x32_update_loop(socket : Arc<UdpSocket>, handle: tauri::AppHandle) {
    let update_all_messages = ConsoleRequest::full_update();
    let mut update_counter = 0_usize;

    loop {
        {
            if update_counter >= 300 { update_counter = 0; }

            let ip: String;
            {
                let settings = handle.state::<RwLock<super::settings::AppSettings>>();
                ip = settings.read().expect("lock failed").x32_ip.clone();
                let force_up = handle.state::<Mutex<bool>>();
                let mut force_up_l = force_up.lock().expect("lock failed");
                if *force_up_l {
                    update_counter = 0;
                    *force_up_l = false;
                }
            }
            if !ip.is_empty() && update_counter % 5 == 0 {
                if let Ok(target ) = format!("{ip}:10023").parse::<SocketAddr>() {
                    socket.send_to(enums::X32_XREMOTE.as_slice(), target).await.expect("broken socket");
                    socket.send_to(enums::X32_METER_0.as_slice(), target).await.expect("broken socket");
                    socket.send_to(enums::X32_METER_5.as_slice(), target).await.expect("broken socket");

                    // Meters already serve this purpose, but leaving it here anyway.
                    // socket.send_to(enums::X32_KEEP_ALIVE.as_slice(), target).await.expect("broken socket");
                
                    if update_counter == 0 {
                        for item in update_all_messages.clone() {
                            socket.send_to(item.as_slice(), target).await.expect("broken socket");
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                    }
                }
            }

            update_counter += 1;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// MARK: MAIN -> VOR
/// VOR Update Loop
pub async fn vor_update_loop(handle: tauri::AppHandle) {
    let socket = UdpSocket::bind("0.0.0.0:0").await.expect("unable to open socket");

    loop {
        {
            let settings:super::settings::AppSettings;
            {
                let settings_h = handle.state::<RwLock<super::settings::AppSettings>>();
                settings = settings_h.read().expect("lock failed").clone();
            }
            if !settings.vor_ip.is_empty() && !settings.vor_port.is_empty() {
                if let Ok(target) = format!("{}:{}", settings.vor_ip, settings.vor_port).parse::<SocketAddr>() {
                    let mut packets:Vec<Packet> = vec![];
                    let fader_bank:enums::FaderBank;
                    let current_cue:String;
                    {
                        let x32 = handle.state::<RwLock<X32Console>>();
                        let x32_state = x32.read().expect("failed to acquire lock");
                        fader_bank = x32_state.faders.clone();
                        current_cue = x32_state.active_cue();
                    }

                    if settings.send_cue.into() {
                        packets.push(Packet::Message((Message::new("/currentCue").add_item(current_cue)).clone()));
                    }

                    if settings.send_dca.into() {
                        packets.push(fader_bundle(&enums::FaderBankKey::Dca, &fader_bank));
                    }

                    if settings.send_main.into() {
                        packets.push(fader_bundle(&enums::FaderBankKey::Main, &fader_bank));
                    }

                    if settings.send_matrix.into() {
                        packets.push(fader_bundle(&enums::FaderBankKey::Matrix, &fader_bank));
                    }

                    if settings.send_aux.into() {
                        packets.push(fader_bundle(&enums::FaderBankKey::Aux, &fader_bank));
                    }

                    if settings.send_bus.into() {
                        packets.push(fader_bundle(&enums::FaderBankKey::Bus, &fader_bank));
                    }

                    if settings.send_channel.into() {
                        packets.push(fader_bundle(&enums::FaderBankKey::Channel, &fader_bank));
                    }

                    for pack in packets {
                        let buffer:Buffer = pack.try_into().unwrap_or_default();
                        socket.send_to(buffer.as_slice(), target).await.expect("broken socket");
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}


/// get OSC bundle for fader type
fn fader_bundle(key : &enums::FaderBankKey, x32_fader_bank : &enums::FaderBank) -> Packet {
    let messages = x32_fader_bank.vor_bundle(key);
    let bundle = Bundle::new_with_messages(messages);
    Packet::Bundle(bundle)
}