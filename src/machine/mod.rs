/*
    Struct to represent a Gemini PR machine to read stroke input from.
*/

use super::engine::steno_to_id;
use serialport::SerialPort;
use std::{error::Error, io::ErrorKind, thread, time::Duration};

pub struct Machine {
    port: Box<dyn SerialPort>,
    path: String,
    keymap: [char; 42],
}
impl Machine {
    pub fn new(path: String, keymap: [char; 42]) -> Self {
        let port = serialport::new(&path, 9600)
            .timeout(Duration::from_millis(10))
            .open()
            .expect("Failed to open port");

        Self {
            port: port,
            path: path,
            keymap: keymap,
        }
    }

    // Read a stroke from the serial buffer, and return the processed steno version of it
    pub fn read(&mut self) -> Result<u32, Box<dyn Error>> {
        let timeout = Duration::from_millis(50);
        let mut buffer: Vec<u8> = vec![0; 6];
        loop {
            match self.port.read_exact(buffer.as_mut_slice()) {
                Ok(()) => {
                    if buffer[0] & 0b1000_0000 > 0 {
                        return Ok(self.bytes_to_stroke(buffer));
                    }
                }
                Err(e) => match e.kind() {
                    ErrorKind::TimedOut => {}
                    ErrorKind::BrokenPipe => self.reconnect(),
                    _ => return Err(Box::new(e)),
                },
            };
            thread::sleep(timeout);
        }
    }

    // Try every timeout seconds to re-open the machine connection
    fn reconnect(&mut self) {
        let timeout = Duration::from_millis(1000);
        loop {
            let port = serialport::new(&self.path, 9600)
                .timeout(Duration::from_millis(10))
                .open();
            match port {
                Ok(x) => {
                    self.port = x;
                    break;
                }
                Err(_) => thread::sleep(timeout),
            }
        }
    }

    // Convert Gemini PR protocol bytes to a steno stroke
    fn bytes_to_stroke(&self, bytes: Vec<u8>) -> u32 {
        let mut stroke = String::with_capacity(30);
        for (byte, e) in bytes.iter().enumerate() {
            for i in (0..7).rev() {
                let mask = 1 << i;
                let index = 7 * byte + 6 - i;
                if e & mask != 0 {
                    let key = self.keymap[index];
                    if !stroke.contains(key) {
                        stroke.push(key);
                    }
                }
            }
        }

        steno_to_id(&stroke)
    }
}
