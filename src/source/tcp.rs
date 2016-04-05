use std::io::{BufReader, Read};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

use serde_json;
use serde::de::Deserialize;

use super::{Source, SourceFrom};
use super::super::{Record};

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    endpoint: (String, u16),
}

pub struct TcpSource {
    abort: Arc<AtomicBool>,
    listener: TcpListener,
}

impl TcpSource {
    fn new(endpoint: &SocketAddr, tx: mpsc::Sender<Record>) -> Result<TcpSource, ::std::io::Error> {
        let listener = TcpListener::bind(endpoint)?;
        info!("exposed TCP input on {}", endpoint);

        let socket = listener.try_clone()?;
        let abort = Arc::new(AtomicBool::new(false));
        let aborted = abort.clone();

        thread::spawn(move || {
            for stream in socket.incoming() {
                if aborted.load(Ordering::SeqCst) {
                    break;
                }

                match stream {
                    Ok(stream) => {
                        let peer = match stream.peer_addr() {
                            Ok(peer) => peer,
                            Err(..) => {
                                continue;
                            }
                        };

                        debug!("accepted TCP connection from {}", peer);
                        let tx = tx.clone();
                        let rd = BufReader::new(stream);
                        thread::spawn(move || {
                            let mut de = serde_json::Deserializer::new(rd.bytes());

                            loop {
                                match Record::deserialize(&mut de) {
                                    Ok(record) => {
                                        tx.send(record).expect("pipeline must outlive all attached inputs");
                                    }
                                    Err(err) => {
                                        warn!("unable to decode payload - {}", err);
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(err) => {
                        error!("unable to accept TCP connection: {}", err);
                    }
                }
            }
        });

        let input = TcpSource {
            abort: abort,
            listener: listener,
        };

        Ok(input)
    }
}

impl Drop for TcpSource {
    fn drop(&mut self) {
        self.abort.store(true, Ordering::SeqCst);
        let endpoint = match self.listener.local_addr() {
            Ok(val) => val,
            Err(..) => return,
        };

        let _ = TcpStream::connect(endpoint);
    }
}

impl Source for TcpSource {
    fn ty() -> &'static str {
        "tcp"
    }
}

impl SourceFrom for TcpSource {
    type Config = Config;

    fn run(config: Config, tx: mpsc::Sender<Record>) -> Result<TcpSource, ()> {
        let (host, port) = config.endpoint;
        debug!("performing blocking DNS request ...");

        for endpoint in (host.as_str(), port).to_socket_addrs().unwrap() {
            if let Ok(source) = TcpSource::new(&endpoint, tx.clone()) {
                return Ok(source);
            }
        }

        Err(())
    }
}
