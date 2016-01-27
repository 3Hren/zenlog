struct TcpInput {
    _listener: TcpListener,
}

impl TcpInput {
    fn new(endpoint: &SocketAddr, tx: Sender<Record>) -> Result<Self, std::io::Error> {
        let listener = try!(TcpListener::bind(endpoint));
        info!(target: "TCP input", "exposed TCP input on {}", endpoint);

        let socket = try!(listener.try_clone());

        thread::spawn(move || {
            for stream in socket.incoming() {
                match stream {
                    Ok(stream) => {
                        let peer = match stream.peer_addr() {
                            Ok(peer) => peer,
                            Err(..) => {
                                continue;
                            }
                        };

                        debug!(target: "TCP input", "accepted TCP connection from {}", peer);
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
                                        warn!(target: "TCP input", "unable to decode payload - {}", err);
                                        break;
                                    }
                                }
                            }
                        });
                    }
                    Err(err) => {
                        error!(target: "TCP input", "unable to accept TCP connection: {}", err);
                    }
                }
            }
        });

        let input = TcpInput {
            _listener: listener,
        };

        Ok(input)
    }
}

impl Input for TcpInput {}
