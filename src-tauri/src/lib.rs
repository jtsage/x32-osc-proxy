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
use std::{net::Ipv4Addr, sync::{Arc, Mutex, RwLock}, time::{Duration, SystemTime}};
use tauri::{Emitter, Manager};
use x32_osc_state as x32;
use tokio::net::UdpSocket;

/// Persistent settings
mod settings;
/// UDP Networking
mod net;

/// webView2 retrieve current settings
#[tauri::command]
fn view_settings(handle: tauri::AppHandle) -> settings::AppSettings {
    let settings = handle.state::<RwLock<settings::AppSettings>>();
    let settings = settings.read().expect("lock failed");
    settings.clone()
}

/// Set X32 IP
#[tauri::command]
fn set_x32(handle: tauri::AppHandle, ip: &str) {
    if ip.parse::<Ipv4Addr>().is_ok() {
        let settings = handle.state::<RwLock<settings::AppSettings>>();
        let mut settings = settings.write().expect("lock failed");
        ip.clone_into(&mut settings.x32_ip);
        let _ = settings.save();

        let x32 = handle.state::<RwLock<x32::X32Console>>();
        let mut x32 = x32.write().expect("failed to acquire lock");
        x32.reset();

        let force_up = handle.state::<Mutex<bool>>();
        *force_up.lock().expect("lock failed") = true;
    }
}

/// Set Vor IP and port
#[tauri::command]
fn set_vor(handle: tauri::AppHandle, ip: &str, port: &str) {
    if let Ok(port_u) = port.parse::<usize>() {
        if (1024..=65535).contains(&port_u) {
            let settings = handle.state::<RwLock<settings::AppSettings>>();
            let mut settings = settings.write().expect("lock failed");
            port.clone_into(&mut settings.vor_port);
            let _ = settings.save();
        }
    }

    if ip.parse::<Ipv4Addr>().is_ok() {
        let settings = handle.state::<RwLock<settings::AppSettings>>();
        let mut settings = settings.write().expect("lock failed");
        ip.clone_into(&mut settings.vor_ip);
        let _ = settings.save();
    }
}

/// Set Vor send flag
#[tauri::command]
fn set_flag(handle: tauri::AppHandle, flag: &str) {
    let settings = handle.state::<RwLock<settings::AppSettings>>();
    let mut settings = settings.write().expect("lock failed");

    match flag {
        "aux"     => settings.send_aux     = settings.send_aux.switch(),
        "bus"     => settings.send_bus     = settings.send_bus.switch(),
        "channel" => settings.send_channel = settings.send_channel.switch(),
        "cue"     => settings.send_cue     = settings.send_cue.switch(),
        "dca"     => settings.send_dca     = settings.send_dca.switch(),
        "main"    => settings.send_main    = settings.send_main.switch(),
        "matrix"  => settings.send_matrix  = settings.send_matrix.switch(),
        _ => ()
    };
    let _ = settings.save();
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(RwLock::new(x32::X32Console::default()));
            app.manage(Mutex::new(SystemTime::UNIX_EPOCH));
            app.manage(RwLock::new(settings::AppSettings::load()));
            app.manage(Mutex::new(false));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    {
                        let state = handle.state::<RwLock<x32::X32Console>>();
                        let x32 = state.read().unwrap().clone();
                        handle.emit("all-faders", &x32.faders).unwrap();
                        handle.emit("cue-count", x32.cue_list_size()).unwrap();
                        handle.emit("cue-current", x32.active_cue()).unwrap();
                    }
                    {
                        let time = handle.state::<Mutex<SystemTime>>();
                        let elapsed = time.lock().expect("lock failed").elapsed().expect("bad clock");
                        handle.emit("packet_status", match elapsed.as_secs() {
                            0..7 => 2_usize,
                            7..=30 => 1_usize,
                            _ => 0_usize
                        }).unwrap();
                    }
                    std::thread::sleep(Duration::from_millis(1000));
                }
            });

            let handle_p = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let sock_listen = Arc::new(UdpSocket::bind("0.0.0.0:10023").await.expect("unable to open socket"));

                let socket = sock_listen.clone();
                let handle = handle_p.clone();
                tauri::async_runtime::spawn(async move {
                    net::x32_listener(socket, handle).await;
                });

                let socket = sock_listen.clone();
                let handle = handle_p.clone();
                tauri::async_runtime::spawn(async move {
                    net::x32_update_loop(socket, handle).await;
                });

                let handle = handle_p.clone();
                tauri::async_runtime::spawn(async move {
                    net::vor_update_loop(handle).await;
                });
            });
            
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![view_settings, set_x32, set_vor, set_flag])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
