#![allow(dead_code)]

use serialport::{SerialPort, available_ports};

use std::{time::Duration, vec};

const IDENT_MSG: &str = "ArmLabCC1200";

pub enum ModulationFormat {
    FSK2 = 0x0,
    GFSK2 = 0x1,
    ASK = 0x3,
    FSK4 = 0x4,
    GFSK4 = 0x5,
}

#[derive(Debug)]
pub enum RadioError {
    PortOpenError,
    DevciceDetectError,
    InvalidArgument,
    ReadLenError,
    ReadError,
    WriteError,
    WriteLenError,
    SyncTimeoutError,
    PortDetectError,
}

/// Radio (Serial) is a driver for interfacing with STM32 based radios over serial
/// This library supports serial communication over all platforms
/// 
/// `Radio::new("PORT_NAME")` to get started
/// 
/// `get_open_ports` or `get_radio_ports` to see options for `PORT_NAME`
pub struct Radio {
    port: Box<dyn SerialPort>,
    port_path: String,
}

// init
impl Radio {
    /// creates a new Radio object on the given port
    /// 
    /// runs existence checks, and attempts to synchronize radio
    /// 
    /// see `Radio::new_bare` for a constructor without overhead
    pub fn new(path: &str) -> Result<Radio, RadioError> {
        let mut port = match serialport::new(path, 115200)
            .timeout(Duration::from_millis(100))
            .open() {
                Ok(n) => n,
                Err(_) => {return Err(RadioError::PortOpenError)}
        };

        match port.write_data_terminal_ready(true) {
            Ok(_) => {},
            Err(_) => {return Err(RadioError::WriteError);},
        };


        match Radio::sync_serial(&mut port, 6) {
            Ok(n) => {println!("synced radio, after {} bytes | device was {} steps ahead", n, 6-n);},
            Err(n) => {return Err(n);},
        };

        if !Radio::check_for_device(&mut port) {
            return Err(RadioError::DevciceDetectError);
        }
        
        
        Ok(Radio {port, port_path: path.to_string()})
    }

    /// creates a new Radio object on the given port
    /// without checking for existence or attempting to synchronize
    /// just simply opens the port and returns an object
    /// 
    /// not recommended for the general use case
    /// see `Radio::new_bare` for implemented existence and synchronization
    pub fn new_bare(path: &str) -> Result<Radio, RadioError> {
        let mut port = match serialport::new(path, 115200)
            .timeout(Duration::from_millis(100))
            .open() {
                Ok(n) => n,
                Err(_) => {return Err(RadioError::PortOpenError)}
        };

        match port.write_data_terminal_ready(true) {
            Ok(_) => {},
            Err(_) => {return Err(RadioError::WriteError)},
        }


        Ok(Radio {port, port_path: path.to_string()})
    }


    /// queries the radio and checks if it is available
    /// 
    /// ## Returns
    /// wheather or not a device was found
    pub fn is_device_available(&mut self) -> bool {
        return Radio::check_for_device(&mut self.port);
    }

    /// queries the port to check if the radio is available
    /// 
    /// useful for in-constructor checks
    fn check_for_device(port: &mut Box<dyn SerialPort>) -> bool {
        let cmd: [u8; 6] = [0,0,0,0,0, '\n' as u8];
        match port.write_all(&cmd) {
            Ok(_) => {},
            Err(_) => {
                return false;
            }
        };

        let mut buf: [u8; IDENT_MSG.len()] = [0u8; IDENT_MSG.len()];
        match port.read_exact(&mut buf) {
            Ok(_) => {},
            Err(_) => {
                return false
            },
        };

        
        if buf != IDENT_MSG.as_bytes() {
            for i in 0..IDENT_MSG.len() {
                println!("{}, {}", buf[i], IDENT_MSG.as_bytes()[i]);
            }
            return false;
        }

        return IDENT_MSG.as_bytes() == buf;
    }

    /// ensures the command queue is in sync with the given port
    /// 
    /// sometimes on intialization Windows will send 1-4 extra bytes causing
    /// the command queue to be out of sync
    /// 
    /// this sends a byte at a time, then checks for output, until it reaches timeout
    /// or it detects output
    fn sync_serial(port: &mut Box<dyn SerialPort>, timeout_iter: usize) -> Result<usize, RadioError> {
        let mut i: usize = 0;

        loop {
            i += 1;
            match port.write_all(&[0]) {
                Ok(_) => {},
                Err(_) => {return Err(RadioError::WriteError);},
            };
            
            let mut buf: [u8; IDENT_MSG.len()] = [0; IDENT_MSG.len()];
            match port.read_exact(&mut buf) {
                Ok(_) => {return Ok(i)},
                Err(_) => {}
            }

            if i == timeout_iter {
                return Err(RadioError::SyncTimeoutError);
            }
    }

    }

    /// ensures the Radio command queue is in sync
    /// 
    /// ## Returns
    /// number of bytes sent in order to sync Radio
    pub fn sync(&mut self, timeout_iter: usize) -> Result<usize, RadioError>  {
        return Radio::sync_serial(&mut self.port, timeout_iter);
    }

}

// transmit / recieve
impl Radio {
    /// helper function to write bytes to Serial
    /// 
    /// ensures the bytes were all flushed 
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), RadioError> {
        // create temp vector to hold both message and newline
        let mut middle: Vec<u8> = vec![0; 0];
        middle.extend_from_slice(data);
        middle.push('\n' as u8);

        // code should always be safe, we are trimming to the data that was added
        // plus we check for the condition where it would fail
        unsafe {
            if data.len() + 1 <= middle.capacity() {
                middle.set_len(data.len() + 1);
            }
        }

        let buf: Box<[u8]> = middle.into_boxed_slice();

        match self.port.write_all(&buf) {
            Ok(_) => Ok(()),
            Err(_) => Err(RadioError::WriteError),
        }
    }

    /// transmits the given message
    pub fn transmit(&mut self, msg: &[u8]) -> Result<(), RadioError> {
        if msg.len() > u8::MAX as usize {
            return Err(RadioError::InvalidArgument);
        }

        let cmd:[u8; 5] = [0x1, msg.len() as u8, 0x0, 0x0, 0x0];
        match self.write_bytes(&cmd) {
            Ok(_) => {},
            Err(_) => {return Err(RadioError::WriteError)},
        };

        
        match self.write_bytes(&msg) {
            Ok(_) => {},
            Err(_) => {return Err(RadioError::WriteError)},
        };

        /*
        let mut buf: [u8; 100] = [0; 100];

        self.port.read(&mut buf);

        for val in buf {
            if val == 0 {
                break;
            }
            print!("{} ", val);
        }
        println!();
        */

        Ok(())
    }

    /// gets the most recent packet stored on the Radio
    /// 
    /// ## Returns
    /// Vec/<u8/> with the byte data of the packet
    /// 
    /// empty Vec/<u8/> if no available packet was found
    pub fn get_packet(&mut self) -> Result<Vec<u8>, RadioError> {
        let cmd: [u8; 5] = [0x2, 0x0, 0x0, 0x0, 0x0];
        match self.write_bytes(&cmd) {
            Ok(_) => {},
            Err(_) => {return Err(RadioError::WriteError)},
        };

        
        let mut buf: [u8; 1] = [0x0];
        match self.port.read_exact(&mut buf) {
            Ok(_) => {},
            Err(_) => {return Err(RadioError::ReadLenError)}
        };
        let msg_size = buf[0] as usize;

        if msg_size == 0 {
            return Ok(Vec::new());
        }
        

        let mut buf2: Box<[u8]> = vec![0; msg_size].into_boxed_slice();
        match self.port.read_exact(&mut buf2) {
            Ok(_) => {},
            Err(_) => {
                return Err(RadioError::ReadError)},
        };

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&buf2);

        Ok(out)
    }

    /// sends a reset command to reset the onboard Radio chip
    pub fn radio_reset(&mut self) -> Result<(), RadioError> {
        return self.write_bytes(&[9,0,0,0,0]);
    }

    /// sends a reset command to perform a soft reset on the entire board
    pub fn soft_reset(&mut self) -> Result<(), RadioError> {
        return self.write_bytes(&[10,0,0,0,0]);
    }

}

impl Radio {
    /// helper function to write the byte data of a f32 to the serial port
    fn set_float_val(&mut self, cmd: u8, val: f32) -> Result<(), RadioError> {
        let bytes = val.to_ne_bytes();
        let mut buf: [u8; 5] = [cmd, 0x00, 0x00, 0x00, 0x00];
        buf[1] = bytes[0];
        buf[2] = bytes[1];
        buf[3] = bytes[2];
        buf[4] = bytes[3];

        match self.write_bytes(&buf) {
            Ok(_) => {},
            Err(_) => {
                return Err(RadioError::WriteError);
            },
        };

        return Ok(());
    }

    /// attempts to set the frequency of the radio
    /// 
    /// there is a chance the radio rejects the value if it is invalid
    pub fn set_frequency(&mut self, frequency: f32) -> Result<(), RadioError> {
        return self.set_float_val(0x03, frequency);
    }

    /// attempts to set the tx gain of the radio
    /// 
    /// there is a chance the radio rejects the value if it is invalid
    pub fn set_power(&mut self, power: f32) -> Result<(), RadioError> {
        return self.set_float_val(0x04, power);
    }

    /// attempts to set the FSK bandwith of the radio
    /// 
    /// there is a chance the radio rejects the value if it is invalid
    pub fn set_deviation(&mut self, deviation: f32) -> Result<(), RadioError> {
        return self.set_float_val(0x05, deviation);
    }

    /// attempts to set the symbol rate of the radio
    /// 
    /// there is a chance the radio rejects the value if it is invalid
    pub fn set_symbol_rate(&mut self, symbol_rate: f32) -> Result<(), RadioError> {
        return self.set_float_val(0x06, symbol_rate);
    }

    /// attempts to set the recieve filter of the radio
    /// 
    /// there is a chance the radio rejects the value if it is invalid
    pub fn set_rx_filter(&mut self, rx_filter: f32) -> Result<(), RadioError> {
        return self.set_float_val(0x07, rx_filter);
    }

    /// sets the modulation mode of the radio
    pub fn set_modulation(&mut self, mode: ModulationFormat) -> Result<(), RadioError> {
        let buf: [u8; 5] = [0x08, mode as u8, 0x00, 0x00, 0x00];

        match self.write_bytes(&buf) {
            Ok(_) => {},
            Err(_) => {
                return Err(RadioError::WriteError);
            },
        };

        return Ok(());
    }
}

/// gets a Vector containing the names of all connected serial ports on the system
pub fn get_open_ports() -> Result<Vec<String>, RadioError> {
    let res = match available_ports() {
        Ok(n) => n,
        Err(_) => return Err(RadioError::PortDetectError),
    };

    let mut out:Vec<String> = vec![];

    for val in res {
        out.push(val.port_name);
    }
    
    return Ok(out);
}

/// gets a Vector containing the names of all connected Radio objects
pub fn get_radio_ports() -> Result<Vec<String>, RadioError> {
    let mut out: Vec<String> = vec![];
    let res = match available_ports() {
        Ok(n) => n,
        Err(_) => {return Err(RadioError::PortDetectError);},
    };

    for val in res {
        let port = match val.port_type {
            serialport::SerialPortType::UsbPort(p) => p,
            _ => {continue;}
        };

        if port.vid == 0x3A3A && port.pid == 0x1 {
            out.push(val.port_name);
        }
    }
    return Ok(out);
}