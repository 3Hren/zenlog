use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::str::FromStr;

use mio;
use mio::{EventLoop, Handler, Token, EventSet, PollOpt};
use mio::udp::UdpSocket;

use serde_json;

use source::{Source, SourceFactory};
use {Config, Record};

struct UdpHandler {
    socket: UdpSocket,
    tx: Sender<Record>,
    buf: [u8; 16 * 1024], // TODO: Replace with Vec. Reason is - kernel recv buffers can be tuned.
}

impl UdpHandler {
    pub fn new(tx: Sender<Record>, socket: UdpSocket) -> UdpHandler {
        UdpHandler {
            socket: socket,
            tx: tx,
            buf: [0; 16 * 1024],
        }
    }
}

impl Handler for UdpHandler {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, ev: &mut EventLoop<UdpHandler>, token: Token, _events: EventSet) {
        assert_eq!(Token(0), token);

        loop {
            // Read until EWOULDBLOCK, because we're using edge triggering.
            match self.socket.recv_from(&mut self.buf) {
                Ok(Some((nread, endpoint))) => {
                    debug!("read {} bytes datagram from {}", nread, endpoint);

                    match serde_json::from_slice::<Record>(&self.buf[..nread]) {
                        Ok(record) => {
                            self.tx.send(record).expect("pipeline must outlive all attached inputs");
                        }
                        Err(err) => {
                            warn!("unable to decode datagram - {}", err);
                        }
                    }
                }
                Ok(None) => {
                    debug!("operation would block - waiting for more events");
                    break;
                }
                Err(err) => {
                    error!("failed to read datagram: {:?}", err);
                    ev.shutdown();
                }
            }
        }
    }

    fn notify(&mut self, ev: &mut EventLoop<UdpHandler>, _: ()) {
        ev.shutdown();
    }
}

pub struct UdpSource {
    stop: mio::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Source for UdpSource {}

impl SourceFactory for UdpSource {
    type Error = ::std::io::Error;

    fn ty() -> &'static str {
        "udp"
    }

    fn run(cfg: &Config, tx: Sender<Record>) -> Result<Box<Source>, Self::Error> {
        let endpoint = cfg.find("endpoint")
            .expect("field 'endpoint' is required")
            .as_string()
            .expect("field 'endpoint' must be a string");

        let listener = try!(UdpSocket::bound(&FromStr::from_str(endpoint).unwrap()));
        info!(target: "UDP input", "exposed UDP input on {}", endpoint);

        let mut ev = try!(EventLoop::new());

        let stop = ev.channel();
        let thread = thread::spawn(move || {
            ev.register(&listener, Token(0), EventSet::readable(), PollOpt::edge()).unwrap();
            ev.run(&mut UdpHandler::new(tx, listener)).unwrap();
        });

        let src = UdpSource {
            stop: stop,
            thread: Some(thread),
        };

        Ok(Box::new(src))
    }
}

impl Drop for UdpSource {
    fn drop(&mut self) {
        self.stop.send(()).unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}
