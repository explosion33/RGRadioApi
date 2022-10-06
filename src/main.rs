use crate::api::start_api;
mod api;

use ArmlabRadio::radio_serial::{Radio, get_radio_ports};

use std::thread;
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
    let mut radio = Radio::new(&get_user_port()).expect("Error Creating Radio");
    radio.set_power(14f32).expect("error setting power");

    let mut start_time = Instant::now();
    let mut good_connection = true;

    loop {
        let mut data = arc_data.lock().unwrap();

        // handle thread quit
        if !data.is_alive {
            return ();
        }

        // handle commands
        for command in data.cmds.iter() {
            let (cmd, arg) = command;
            match cmd.as_str() {
                _ => {
                    println!("unknown command");
                }
            }
        }
        data.cmds.clear();

        if good_connection && start_time.elapsed() >= Duration::from_millis(2000) {
            // transmit heartbeat
            
            start_time = Instant::now();
        }


        //handle data downlink
        //radio.transmit()
        //radio.get_packet()

        drop(data);
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
