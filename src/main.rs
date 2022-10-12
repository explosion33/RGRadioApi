use crate::api::start_api;
mod api;

use ArmlabRadio::radio_serial::{Radio, get_radio_ports};

use std::slice::from_raw_parts;
use std::{thread, usize};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

const DATA_STREAM_SIZE: usize = 30;

struct rocket_data {
    time: u32,
    altitude: f32,
    orx: f32,
    ory: f32,
    orz: f32,
    lat: f32,
    long: f32,
    fix: u8,
    quality: u8,
    cont1: bool,
    cont2: bool,
}

fn encode_stream(data: rocket_data) -> Result<[u8; DATA_STREAM_SIZE], String> {
    let mut buf: Vec<u8> = vec![];

    buf.extend_from_slice(&data.time.to_le_bytes());
    buf.extend_from_slice(&data.altitude.to_le_bytes());
    buf.extend_from_slice(&data.orx.to_le_bytes());
    buf.extend_from_slice(&data.ory.to_le_bytes());
    buf.extend_from_slice(&data.orz.to_le_bytes());
    buf.extend_from_slice(&data.lat.to_le_bytes());
    buf.extend_from_slice(&data.long.to_le_bytes());


    let mut fix_qual: u8 = data.quality << 4;
    fix_qual += data.fix & 0b00001111;
    buf.push(fix_qual);

    let mut conts: u8 = 0;
    if data.cont1 {
        conts += 1;
    }
    if data.cont2 {
        conts += 2;
    }

    buf.push(conts);

    match buf.as_slice().try_into() {
        Ok(n) => Ok(n),
        Err(_) => Err("Error converting vec to slice".to_string()),
    }
}

fn decode_stream(buf: [u8; DATA_STREAM_SIZE]) -> Result<rocket_data, String> {
    let time: u32 = u32::from_le_bytes(match buf[0..4].try_into(){
        Ok(n) => n,
        Err(_) => {return Err("error converting time to u32".to_string())},
    });

    let altitude: f32 = f32::from_le_bytes(match buf[4..8].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting altitude to f32".to_string())},
    });
    let orx: f32 = f32::from_le_bytes(match buf[8..12].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting orx to f32".to_string())},
    });
    let ory: f32 = f32::from_le_bytes(match buf[12..16].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting ory to f32".to_string())},
    });
    let orz: f32 = f32::from_le_bytes(match buf[16..20].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting orz to f32".to_string())},
    });
    let lat: f32 = f32::from_le_bytes(match buf[20..24].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting lat to f32".to_string())},
    });
    let long: f32 = f32::from_le_bytes(match buf[24..28].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting long to f32".to_string())},
    });

    // 28: 0000 0000
    //     qual fix
    let fix: u8 = buf[28] | 0b00001111;// first 4 (least significant) bits of 29
    let quality: u8 = buf[28] >> 4;// last 4 bits of 29

    // 00000   0   0
    //         2   1
    let cont1: bool = buf[29] & 1 == 1; // first (lsb) of 30
    let cont2: bool = buf[29] & 2 == 1; // second (lsb) of 30
    


    Ok(rocket_data {time, altitude, orx, ory, orz, lat, long, fix, quality, cont1, cont2})
}

fn get_user_port() -> String{
    let radios = get_radio_ports().expect("error getting devices");
    if radios.len() == 0 {
        panic!("no radios found");
    }
    return radios[0].clone();

    /*let port: String = match radios.len() {
        1 => {
            println!("Found one radio on {}", radios[0]);
            radios[0].clone()
        }
        0 | _ => {
            if radios.len() == 0 {
                println!("Radio could not be automatically detected");
                radios = get_open_ports().unwrap();
            }
            else {
                println!("Multiple radios detected");
            }

            println!("Please select a port: ");
            let mut i: usize = 0;
            for port in &radios {
                println!("\t{}. {}", i, port);
                i += 1;
            }

            loop {
                let res = input!("> ");
                
                let val: usize = match res.parse::<usize>() {
                    Ok(n) => n,
                    Err(_) => {
                        println!("Error \"{}\" is not a valid selection", res);
                        continue;
                    }
                };

                if val >= radios.len() {
                    println!("Error \"{}\" is not a valid selection", res);
                    continue;
                }
                break radios[val].clone();
            }
        }

    };
    */

}




fn radio(arc_data: api::TData) {
    let port = get_user_port();

    println!("found radio on port {}", port);

    let mut radio = Radio::new(&port).expect("Error Creating Radio");
    radio.set_power(14f32).expect("error setting power");

    let abs_start = Instant::now();
    let mut altitude: f32 = 0f32;
    let mut start_time = Instant::now();
    let mut good_connection = true;
    let mut iter: usize = 0;

    loop {
        let mut data = arc_data.lock().unwrap();

        // handle thread quit
        if !data.is_alive {
            return ();
        }

        // handle commands
        for command in data.cmds.iter() {
            let (cmd, arg) = command;

            println!("got cmd {}, with args {}", cmd, arg);

            let mut buf: [u8; 5] = [0u8; 5];
            match cmd.as_str() {
                "test" => {
                    buf[0] = 2;
                    
                    let f = arg.to_le_bytes();
                    buf[1] = f[0];
                    buf[2] = f[1];
                    buf[3] = f[2];
                    buf[4] = f[3];

                }
                _ => {
                    println!("unknown command");
                }
            }
            
            radio.transmit(&buf).expect("error transmitting");
        }
        data.cmds.clear();

        // downlink
        {
            // get data stream
            let buf = match radio.get_packet() {
                Ok(n) => {
                    if n.len() == 0 {
                        return;
                    }
                    n
                },
                Err(_) => {return;}
            };

            let buf: [u8; DATA_STREAM_SIZE] = match buf.try_into() {
                Ok(n) => n,
                Err(n) => {
                    println!("Error | expected length {} got {} ", DATA_STREAM_SIZE, n.len());
                    return;
                }
            };

            let rec_data = match decode_stream(buf) {
                Ok(n) => n,
                Err(n) => {
                    println!("Error decoding stream | {}", n);
                    return;
                }
            };

            let time: f32 = rec_data.time as f32 / 1000f32;

            data.altitude.push((time, rec_data.altitude)); 
            data.orx.push((time, rec_data.orx));
            data.ory.push((time, rec_data.ory));
            data.orz.push((time, rec_data.orz));
            data.lat.push((time, rec_data.lat));
            data.long.push((time, rec_data.long));
            data.fix.push((time, rec_data.fix as f32));
            data.quality.push((time, rec_data.quality as f32));
            data.cont_droug.push((time, if rec_data.cont1 {1f32} else {0f32}));
            data.cont_main.push((time, if rec_data.cont2 {1f32} else {0f32}));

        }

        drop(data);

        // if we are unable to parse a data stream we continue; this skips the heartbeat section alltogether
        if start_time.elapsed() >= Duration::from_millis(2000) {
            iter += 1;
            // transmit heartbeat
            println!("sending heartbeat {}", iter);
            
            start_time = Instant::now();
            radio.transmit(&[1, 1, 1, 1, 1]).expect("transmit error");
        }


        
        // give api a chance to aquire mutex lock
        thread::sleep(Duration::from_millis(50));
    } 

}

fn main() {
    println!("Hello, world!");

    let data = api::Data::new();
    let thread_data: api::TData = Arc::new(Mutex::new(data));
    let collect = Arc::clone(&thread_data);


    // move serial radio handler to thread
    // write recieved data to TData
    // write commands from TData to rocket
    // Radio Comm Layer / Protocol needs to be established
    
    let handle = thread::spawn(move || {
        println!("setting up thread");
        radio(collect);
    });
    

    println!("starting api");
    start_api(thread_data);
    println!("api closed");
    let _ = handle.join();
    println!("thread closed");

    //loop {}

}
