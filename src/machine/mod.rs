/*
    Struct to represent a Gemini PR machine to read stroke input from.
*/

use super::engine::steno_to_id;
use super::Ui;
use serialport::SerialPort;
use std::{error::Error, io::ErrorKind, thread, time::Duration};

const READ_RATE: Duration = Duration::from_millis(50);
const RECONNECT_RATE: Duration = Duration::from_millis(1000);

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
    pub fn read(&mut self, tx: std::sync::mpsc::Sender<Ui>) -> Result<u32, Box<dyn Error>> {
        let mut buffer: Vec<u8> = vec![0; 6];
        loop {
            match self.port.read_exact(buffer.as_mut_slice()) {
                Ok(()) => {
                    if buffer[0] & 0b1000_0000 > 0 {
                        let mut stroke = String::with_capacity(super::engine::STENO_ORDER.len());
                        for (byte, e) in buffer.iter().enumerate() {
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
                        return Ok(steno_to_id(&stroke));
                    }
                }
                Err(e) => match e.kind() {
                    ErrorKind::TimedOut => {}
                    ErrorKind::BrokenPipe => loop {
                        tx.send(Ui::Machine(String::new())).unwrap();
                        let port = serialport::new(&self.path, 9600)
                            .timeout(Duration::from_millis(10))
                            .open();
                        match port {
                            Ok(x) => {
                                self.port = x;
                                tx.send(Ui::Machine(self.path.clone())).unwrap();
                                break;
                            }
                            Err(_) => thread::sleep(RECONNECT_RATE),
                        }
                    },
                    _ => return Err(Box::new(e)),
                },
            }
            thread::sleep(READ_RATE);
        }
    }
}
