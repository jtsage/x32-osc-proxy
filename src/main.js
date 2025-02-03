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
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let show_meters = true;

window.addEventListener("DOMContentLoaded", () => {
    populate_settings()

    document.getElementById('x32').addEventListener('submit', (e) => {
        e.preventDefault(); set_x32()
    })
    document.getElementById('vor').addEventListener('submit', (e) => {
        e.preventDefault(); set_vor()
    })

    document.getElementById('send-cue').addEventListener('change', (e) => { set_flag('cue') })
    document.getElementById('send-dca').addEventListener('change', (e) => { set_flag('dca') })
    document.getElementById('send-main').addEventListener('change', (e) => { set_flag('main') })
    document.getElementById('send-matrix').addEventListener('change', (e) => { set_flag('matrix') })
    document.getElementById('send-bus').addEventListener('change', (e) => { set_flag('bus') })
    document.getElementById('send-aux').addEventListener('change', (e) => { set_flag('aux') })
    document.getElementById('send-channel').addEventListener('change', (e) => { set_flag('channel') })

    document.getElementById('show-audio').addEventListener('change', (e) => {
        show_meters = e.target.checked;
    })
});

async function set_flag(flag) {
    await invoke("set_flag", { flag : flag })
    populate_settings()
}

async function set_x32() {
    let ip = document.getElementById("x32-address").value
    await invoke("set_x32", { ip : ip })
    populate_settings()
}

async function set_vor() {
    let ip = document.getElementById("vor-address").value
    let port = document.getElementById("vor-port").value
    await invoke("set_vor", { ip : ip, port : port })
    populate_settings()
}

async function populate_settings() {
    let result = await invoke("view_settings");
    document.getElementById('x32-address').value = result.x32_ip;
    document.getElementById('vor-address').value = result.vor_ip;
    document.getElementById('vor-port').value = result.vor_port;
    document.getElementById('send-cue').checked = result.send_cue;
    document.getElementById('send-channel').checked = result.send_channel;
    document.getElementById('send-main').checked = result.send_main;
    document.getElementById('send-dca').checked = result.send_dca;
    document.getElementById('send-bus').checked = result.send_bus;
    document.getElementById('send-matrix').checked = result.send_matrix;
    document.getElementById('send-aux').checked = result.send_aux;
}

let last_meter_0 = Array(71).fill(0);
let last_meter_5 = Array(28).fill(0);

const meters_0 = await listen('meter-update-0', (event) => {
    // 0 - garbage
    // 1-32 - channel
    // 33-40 - aux
    // 41-48 - garbage (fx)
    // 49-64 - bus
    // 65-70 - mtx
    if ( Array.isArray(event.payload) ) {
        let this_data = show_meters ? event.payload : Array(71).fill(1)

        for (const [i, v] of this_data.entries()) {
            if ( i == 0 || ( i >= 41 && i <= 48 )) { continue }

            // check against last.
            if ( last_meter_0[i] === v ) { continue }

            last_meter_0[i] = v

            // compute percentage
            let percent = Math.max(Math.min(Math.floor(100 - ((v ** 0.35) * 100)), 100), 0)

            if (i >= 1 && i <= 32 ) {
                do_fader_meter(`chan-${i}`, percent)
            } else if (i >= 33 && i <= 40 ) {
                do_fader_meter(`aux-${i-32}`, percent)
            } else if (i >= 49 && i <= 64 ) {
                do_fader_meter(`bus-${i-48}`, percent)
            } else if (i >= 65 && i <= 70 ) {
                do_fader_meter(`mtx-${i-64}`, percent)
            }
        }
    }
})


const meters_5 = await listen('meter-update-5', (event) => {
    // 0 - garbage
    // 1-16 - channel
    // 17-24 - aux
    // 25-26 - main l/r
    // 27 - mono
    if ( Array.isArray(event.payload) ) {
        let this_data = show_meters ? event.payload : Array(28).fill(1)

        for (const [i, v] of this_data.entries()) {
            if ( i <= 16 || i == 26 ) { continue }

            // check against last.
            if ( last_meter_5[i] === v ) { continue }

            last_meter_5[i] = v

            // compute percentage
            let percent = Math.max(Math.min(Math.floor(100 - ((v ** 0.35) * 100)), 100), 0)

            if (i >= 17 && i <= 24 ) {
                do_fader_meter(`dca-${i-16}`, percent)
            } else if (i == 25 ) {
                do_fader_meter('main-1', percent)
            } else if (i == 27 ) {
                do_fader_meter('main-2', percent)
            }
        }
    }
})

function do_fader_meter(id, level) {
    const element = document.querySelector(`#${id} .fader-level-mask`)
    if (element !== null) {
        element.style.height = `${level}%`
    }
}

const last_packet = await listen('packet_status', (event) => {
    document.getElementById("status").setAttribute('data-last', event.payload);
})

const cue_current = await listen('cue-current', (event) => {
    document.getElementById("current-cue").textContent = event.payload
})

const cue_count = await listen('cue-count', (event) => {
    document.getElementById("cue-count").textContent = event.payload[0]
    document.getElementById("scene-count").textContent = event.payload[1]
    document.getElementById("snippet-count").textContent = event.payload[2]
})

const one_fader_listen = await listen('single-fader', (event) => {
    let id = `${event.payload.source.type}-${event.payload.source.index}`
    update_fader_by_id(id, event.payload);
});

const fader_listen = await listen('all-faders', (event) => {
    let bank = event.payload;

    update_fader('aux', bank.aux)
    update_fader('bus', bank.bus)
    update_fader('main', bank.main)
    update_fader('mtx', bank.matrix)
    update_fader('dca', bank.dca)
    update_fader('chan', bank.channel)
});



function update_fader(name, data) {
    if (!Array.isArray(data)) { return }
    for (let i = 0; i < data.length; i++) {
        update_fader_by_id(`${name}-${i + 1}`, data[i])
    }
}

function update_fader_by_id(id, data) {
    const element = document.getElementById(id);
    if (element !== null) {
        element.querySelector('.fader-label').innerHTML = data.label === "" ? "&nbsp;" : data.label
        element.querySelector('.fader-level').textContent = data.level
        element.setAttribute("data-is-on", data.is_on ? "true" : "false")
        element.setAttribute("data-color", data.color)
    }
}