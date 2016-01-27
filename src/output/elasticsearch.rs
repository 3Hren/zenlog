use std::collections::VecDeque;
use std::sync::mpsc;
use std::io::Read;
use std::time::Duration;
use std::thread::{self, JoinHandle};

use hyper::client::Client;
use elastic_hyper::bulk::post_index_type;

use serde_json::ser::to_string;
use serde_json::value::Value;

use super::super::Output;
use super::super::Record;

enum Event {
    Data(Record),
    Timeout,
    Terminate,
}

pub struct ElasticSearch {
    tx: mpsc::Sender<Event>,
    threads: Option<(JoinHandle<()>, JoinHandle<()>)>
}

impl ElasticSearch {
    pub fn new(host: &str, port: u16) -> ElasticSearch {
        let mut client = Client::new();

        let (tx, rx) = mpsc::channel();

        let url = format!("{}:{}", host, port);

        let worker = thread::spawn(move || {
            let mut queue = VecDeque::new();

            for event in rx {
                match event {
                    Event::Data(mut record) => {
                        record.as_object_mut()
                            .unwrap()
                            .insert("hostname".to_owned(), Value::String("pidor".to_owned()));
                        queue.push_back(record);
                    }
                    Event::Timeout => {
                        debug!(target: "O.ES.W", "timed out");

                        if queue.is_empty() {
                            continue;
                        }

                        debug!(target: "O.ES.W", "ready for sending {} actions from the queue", queue.len());

                        let mut body = String::new();
                        for record in queue.drain(..) {
                            body.push_str("{\"index\": {}}");
                            body.push('\n');
                            body.push_str(&to_string(&record).unwrap());
                            body.push('\n');
                        }

                        debug!(target: "O.ES.W", "sending...\n{}", body);

                        match post_index_type(&mut client, &url, "cocaine-v0.12-2016.03.10-16", "cocaine", body) {
                            Ok(mut rs) => {
                                info!("{:?}", rs);
                                let mut body = String::new();
                                rs.read_to_string(&mut body).unwrap();
                                info!("{:?}", body);
                            }
                            Err(err) => {
                                error!(target: "O.ES.W", "failed to process bulk request: {}", err);
                            }
                        }
                    }
                    Event::Terminate => {
                        break;
                    }
                }
            }

            // TODO: Reenable when fix threads destruction order.
            // debug!(target: "O::ES::W", "stopped");
        });

        let timer = {
            let tx = tx.clone();

            thread::spawn(move || {
                let dur = Duration::from_millis(3000);

                loop {
                    thread::sleep(dur);
                    debug!(target: "O::ES::T", "slept for {:?}", dur);

                    if tx.send(Event::Timeout).is_err() {
                        break;
                    }
                }

                // TODO: Reenable when fix threads destruction order.
                // debug!(target: "O::ES::T", "stopped");
            })
        };

        ElasticSearch {
            tx: tx,
            threads: Some((worker, timer)),
        }
    }
}

impl Output for ElasticSearch {
    fn feed(&mut self, payload: &Record) {
        self.tx.send(Event::Data(payload.clone())).unwrap();
    }
}

impl Drop for ElasticSearch {
    fn drop(&mut self) {
        self.tx.send(Event::Terminate).unwrap();

        let threads = self.threads.take().unwrap();
        threads.0.join().unwrap();
        threads.1.join().unwrap();
    }
}
