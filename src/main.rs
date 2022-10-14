use crate::api::start_api;
mod api;

use crate::protocol::{RocketData, decode_stream, DATA_STREAM_SIZE};
mod protocol;

use ArmlabRadio::radio_serial::{Radio, get_radio_ports};

use std::{thread, usize};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

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

    let mut start_time = Instant::now();
    let mut iter: usize = 0;

    loop {
        let mut data = match arc_data.lock() {
            Ok(n) => n,
            Err(_) => {
                println!("could not lock mutex");
                continue;
            } 
        };

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
            
            match radio.transmit(&buf) {
                Ok(_) => {},
                Err(n) => {
                    println!("transmit error: {:?} | skipping command: {}", n, cmd);
                }
            };
        }
        data.cmds.clear();

        // downlink
        let mut handle_packet = || -> () {
            // get data stream
            let buf = match radio.get_packet() {
                Ok(n) => {
                    if n.len() == 0 {
                        return;
                    }
                    n
                },
                Err(n) => {
                    println!("Error getting packet: {:?}", n);
                    return;
                }
            };

            let buf: [u8; DATA_STREAM_SIZE] = match buf.try_into() {
                Ok(n) => n,
                Err(n) => {
                    println!("Error | expected length {} got {} ", DATA_STREAM_SIZE, n.len());
                    return;
                }
            };

            let rec_data: RocketData = match decode_stream(buf) {
                Ok(n) => n,
                Err(n) => {
                    println!("Error decoding stream | {}", n);
                    return;
                }
            };

            println!("got packet, alt={}", rec_data.altitude);

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

        };

        handle_packet();

        drop(data);

        // if we are unable to parse a data stream we continue; this skips the heartbeat section alltogether
        if start_time.elapsed() >= Duration::from_millis(2000) {
            iter += 1;
            // transmit heartbeat
            println!("sending heartbeat {}", iter);
            
            start_time = Instant::now();
            match radio.transmit(&[1, 1, 1, 1, 1]) {
                Ok(_) => {},
                Err(n) => {
                    println!("transmit error: {:?} | skipping heartbeat", n);
                }
            }
        }


        
        // give api a chance to aquire mutex lock
        thread::sleep(Duration::from_millis(50));
    } 

}

fn main() {
    let data = api::Data::new();
    let thread_data: api::TData = Arc::new(Mutex::new(data));
    let collect = Arc::clone(&thread_data);


    
    // move serial radio handler to thread with shared data struct
    let handle = thread::spawn(move || {
        println!("setting up thread");
        radio(collect);
    });
    
    // move api to thread with same shared data struct
    println!("starting api");
    let handle2 = thread::spawn(move || {
        start_api(thread_data);
    });

    // check if either thread quits and terminate the program if they do
    // radio will panic with a radio error
    // api will close once a quit command is sent
    loop {
        if handle.is_finished() || handle2.is_finished() {
            println!("one of the threads closed, terminating");
            return;
        }
    }

}
