use std::error::Error;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};

use mio;
use mio::{EventLoop, Handler, Token, EventSet, PollOpt};
use mio::udp::UdpSocket;

use serde_json;

use source::{Source, SourceFactory};
use {Config, Record};

struct UdpHandler {
    socket: UdpSocket,
    tx: Sender<Arc<Record>>,
    buf: Vec<u8>,
}

impl UdpHandler {
    pub fn new(tx: Sender<Arc<Record>>, socket: UdpSocket) -> UdpHandler {
        UdpHandler {
            socket: socket,
            tx: tx,
            buf: Vec::with_capacity(16 * 1024),
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
            match self.socket.recv_from(&mut self.buf[..]) {
                Ok(Some((nread, endpoint))) => {
                    debug!("read {} bytes datagram from {}", nread, endpoint);

                    match serde_json::from_slice::<Record>(&self.buf[..nread]) {
                        Ok(record) => {
                            self.tx.send(Arc::new(record))
                                .expect("pipeline must outlive all attached inputs");
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

impl UdpSource {
    fn new(endpoint: &SocketAddr, tx: Sender<Arc<Record>>) -> Result<UdpSource, Box<Error>> {
        let listener = try!(UdpSocket::bound(endpoint));
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

        Ok(src)
    }
}

impl Source for UdpSource {}

impl SourceFactory for UdpSource {
    type Error = Box<Error>;

    fn ty() -> &'static str {
        "udp"
    }

    fn run(cfg: &Config, tx: Sender<Arc<Record>>) -> Result<Box<Source>, Box<Error>> {
        let endpoint = cfg.find("endpoint")
            .expect("field 'endpoint' is required")
            .as_string()
            .expect("field 'endpoint' must be a string");

        UdpSource::new(&FromStr::from_str(endpoint).unwrap(), tx)
            .map(|v| Box::new(v) as Box<Source>)
    }
}

impl Drop for UdpSource {
    fn drop(&mut self) {
        self.stop.send(()).unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}
