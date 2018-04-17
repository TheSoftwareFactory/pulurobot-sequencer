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
        self.signal_ch.send(true).unwrap();
    }

    fn stop(&self) {
        self.signal_ch.send(false).unwrap();
    }
}

fn start_read_thread(signal_ch: Receiver<bool>, data_ch: Sender<u8>) {
    thread::spawn(move || {
        let mut stream: Option<TcpStream> = None;
        let mut read_mode = false;
        let mut buf: [u8; 1] = [0];

        loop {
            read_mode = signal_ch.try_recv().unwrap_or(read_mode);
            match (read_mode, stream.is_some()) {
                (true, false) => {
                    stream = Some(TcpStream::connect(ROBOT_IP).unwrap());
                    stream.as_mut().unwrap().read(&mut buf).unwrap();
                    data_ch.send(buf[0]).unwrap();
                }
                (true, true) => {
                    stream.as_mut().unwrap().read(&mut buf).unwrap();
                    data_ch.send(buf[0]).unwrap();
                }
                (false, false) => (),
                (false, true) => {
                    stream
                        .as_mut()
                        .unwrap()
                        .shutdown(::std::net::Shutdown::Both)
                        .unwrap();
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
        let mut write_mode = false;

        loop {
            write_mode = signal_ch.try_recv().unwrap_or(write_mode);
            match (write_mode, stream.is_some()) {
                (true, false) => {
                    stream = Some(TcpStream::connect(ROBOT_IP).unwrap());

                    let data: Vec<u8> = data_ch.recv().unwrap();
                    stream.as_mut().unwrap().write_all(&data[..]).unwrap();
                    completed_ch.send(true).unwrap();
                }
                (true, true) => {
                    let data: Vec<u8> = data_ch.recv().unwrap();
                    stream.as_mut().unwrap().write_all(&data[..]).unwrap();
                    completed_ch.send(true).unwrap();
                }
                (false, false) => (),
                (false, true) => {
                    stream
                        .as_mut()
                        .unwrap()
                        .shutdown(::std::net::Shutdown::Both)
                        .unwrap();
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
                println!("Got connection");
                stream.set_read_timeout(Some(::std::time::Duration::from_millis(10))).unwrap();
                stream.set_nodelay(true).unwrap();

                loop {
                    stream.set_nonblocking(true).unwrap();
                    let mut buf: Vec<u8> = Vec::new();

                    match stream.read_to_end(&mut buf).is_ok() && buf.len() > 0 {
                        true => {
                            read_conn.stop();
                            write_conn.start();

                            println!("buf {:?}", buf);
                            write_conn.data_ch.send(buf).unwrap();

                            let completed = write_conn.completed_ch.recv().expect("write conn");
                            println!("completed write: {:?}", completed);

                            if completed {
                                read_conn.start();
                                write_conn.stop();
                            }
                        }
                        false => {
                            read_conn.start();
                            write_conn.stop();

                            let data = read_conn.data_ch.recv().expect("data receive failed");

                            println!("data {:?}", data);

                            let slice = vec![data];
                            println!("slice {:?}", slice);

                            stream.set_nonblocking(false).unwrap();
                            stream.write(&slice[..]).expect("write slice failed");
                            stream.flush().expect("flush slice failed");
                        }
                    };
                }
            }
            Err(e) => {
                println!("connection failed");
            }
        }
    }
}
