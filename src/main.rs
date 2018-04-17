use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::net::{TcpListener, TcpStream};

static ROBOT_IP: &'static str = "192.168.43.23:22222";

struct ReadConnection {
    signal_ch: Sender<bool>,
    data_ch: Receiver<u8>,
}

impl ReadConnection {
    fn spawn() -> Self {
        let signal_ch: (Sender<bool>, Receiver<bool>) = channel();
        let data_ch: (Sender<u8>, Receiver<u8>) = channel();

        start_read_thread(signal_ch.1, data_ch.0);

        ReadConnection {
            signal_ch: signal_ch.0,
            data_ch: data_ch.1,
        }
    }

    fn start(&self) {
        self.signal_ch.send(true);
    }

    fn stop(&self) {
        self.signal_ch.send(false);
    }
}

fn start_read_thread(signal_ch: Receiver<bool>, data_ch: Sender<u8>) {
    thread::spawn(move || {
        let mut stream: Option<TcpStream> = None;
        let mut buf: [u8; 1] = [0];
        loop {
            let read_mode = signal_ch.try_recv().unwrap_or(true);
            match (read_mode, stream.is_some()) {
                (true, false) => {
                    stream = Some(TcpStream::connect(ROBOT_IP).unwrap());
                    stream.as_mut().unwrap().read(&mut buf);
                    data_ch.send(buf[0]);
                }
                (true, true) => {
                    stream.as_mut().unwrap().read(&mut buf);
                    data_ch.send(buf[0]);
                }
                (false, false) => (),
                (false, true) => {
                    stream
                        .as_mut()
                        .unwrap()
                        .shutdown(::std::net::Shutdown::Both);
                    stream = None;
                }
            }
        }
    });
}

struct WriteConnection {
    signal_ch: Sender<bool>,
    data_ch: Sender<Vec<u8>>,
    completed_ch: Receiver<bool>,
}

impl WriteConnection {
    fn spawn() -> Self {
        let signal_ch: (Sender<bool>, Receiver<bool>) = channel();
        let data_ch: (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
        let completed_ch: (Sender<bool>, Receiver<bool>) = channel();

        start_write_thread(signal_ch.1, data_ch.1, completed_ch.0);

        WriteConnection {
            signal_ch: signal_ch.0,
            data_ch: data_ch.0,
            completed_ch: completed_ch.1,
        }
    }

    fn start(&self) {
        self.signal_ch.send(true);
    }

    fn stop(&self) {
        self.signal_ch.send(false);
    }
}

fn start_write_thread(
    signal_ch: Receiver<bool>,
    data_ch: Receiver<Vec<u8>>,
    completed_ch: Sender<bool>,
) {
    thread::spawn(move || {
        let mut stream: Option<TcpStream> = None;
        loop {
            let write_mode = signal_ch.try_recv().unwrap_or(false);
            match (write_mode, stream.is_some()) {
                (true, false) => {
                    stream = Some(TcpStream::connect(ROBOT_IP).unwrap());

                    let data: Vec<u8> = data_ch.recv().unwrap();
                    stream.as_mut().unwrap().write_all(&data[..]);
                    completed_ch.send(true);
                }
                (true, true) => {
                    let data: Vec<u8> = data_ch.recv().unwrap();
                    stream.as_mut().unwrap().write_all(&data[..]);
                    completed_ch.send(true);
                }
                (false, false) => (),
                (false, true) => {
                    stream
                        .as_mut()
                        .unwrap()
                        .shutdown(::std::net::Shutdown::Both);
                    stream = None;
                }
            };
        }
    });
}

fn main() {
    let read_conn = ReadConnection::spawn();
    let write_conn = WriteConnection::spawn();

    let listener = TcpListener::bind("192.168.43.97:22222").unwrap();
    println!("Listening on 192.168.43.97:22222");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                stream.set_read_timeout(Some(::std::time::Duration::from_millis(10)));
                println!("Got connection");

                loop {
                    let mut buf: Vec<u8> = Vec::new();
                    match stream.read_to_end(&mut buf).is_ok() && buf.len() > 0 {
                        true => {
                            read_conn.stop();
                            write_conn.start();

                            println!("buf {:?}", buf);
                            write_conn.data_ch.send(buf);
                        }
                        false => {
                            read_conn.start();
                            write_conn.stop();

                            let data = read_conn.data_ch.recv().unwrap();

                            println!("data {:?}", data);

                            let slice = vec![data];
                            println!("slice {:?}", slice);

                            stream.write(&slice[..]).unwrap();
                            stream.flush();
                        }
                    }
                }
            }
            Err(e) => {
                println!("connection failed");
            }
        }
    }

    // let mut stream = TcpStream::connect("192.168.43.97:22222").unwrap();
    // let x = 1000;
    // let y = 1000;

    // let mut buf: [u8; 11] = [0; 11];
    // buf[0] = 56;
    // buf[1] = 0;
    // buf[2] = 9;

    // buf[3] = (x << 24) as u8;
    // buf[4] = (x << 16) as u8;
    // buf[5] = (x << 8) as u8;
    // buf[6] = (x << 0) as u8;

    // buf[7] = (y << 24) as u8;
    // buf[8] = (y << 16) as u8;
    // buf[9] = (y << 8) as u8;
    // buf[10] = (y << 0) as u8;

    // stream.write(&buf);

    // let listener = TcpListener::bind("192.168.43.97:22222").unwrap();
    // for stream in listener.incoming() {
    //     match stream {
    //         Ok(mut stream) => {
    //             let mut buf: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    //             stream.write(&buf[..]);
    //         }
    //         Err(e) => {
    //             println!("connection failed");
    //          }
    //     }
    // }
}
