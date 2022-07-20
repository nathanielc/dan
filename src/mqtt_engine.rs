use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::vm::Engine;

use mqtt_async_client::client::{Client, Publish, QoS, ReadResult, Subscribe, SubscribeTopic};

#[derive(Debug)]
pub struct MQTTEngine {
    requests_tx: mpsc::Sender<Request>,
    join_handle: JoinHandle<Result<()>>,
}

#[derive(Debug)]
enum Request {
    Publish(Publish),
    Subscribe(Subscribe),
    Watch(Watch),
}
#[derive(Debug)]
struct Watch {
    path: String,
    value: Vec<u8>,
    tx: oneshot::Sender<()>,
}

enum SelectResult {
    Request(Option<Request>),
    Data(ReadResult),
}

impl MQTTEngine {
    pub fn new(url: &str) -> Result<Arc<Self>> {
        // Create a client & define connect options
        let cli = Client::builder().set_url_string(url)?.build()?;

        let (requests_tx, requests_rx) = mpsc::channel(100);
        let join_handle = tokio::spawn(async move { Self::run(cli, requests_rx).await });
        Ok(Arc::new(Self {
            requests_tx,
            join_handle,
        }))
    }
    async fn run(mut cli: Client, mut requests_rx: mpsc::Receiver<Request>) -> Result<()> {
        cli.connect().await?;
        let mut watches: Vec<Watch> = Vec::new();
        loop {
            let s = select! {
                req = requests_rx.recv() =>  SelectResult::Request(req),
                data = cli.read_subscriptions() =>  SelectResult::Data(data?),
            };
            match s {
                SelectResult::Request(req) => match req {
                    Some(Request::Watch(watch)) => watches.push(watch),
                    Some(Request::Publish(p)) => {
                        cli.publish(&p).await?;
                    }
                    Some(Request::Subscribe(s)) => {
                        cli.subscribe(s).await?;
                    }
                    None => break,
                },
                SelectResult::Data(data) => {
                    let mut i = 0 as usize;
                    while i < watches.len() {
                        if data.topic() == watches[i].path && data.payload() == watches[i].value {
                            let w = watches.remove(i);
                            w.tx.send(()).unwrap();
                            continue;
                        }
                        i = i + 1;
                    }
                }
            }
        }
        let r = cli.disconnect().await;
        Ok(r?)
    }
    pub async fn shutdown(self) -> Result<()> {
        // Explicitly drop request_tx so that the run loop
        // knows its done
        drop(self.requests_tx);
        self.join_handle.await??;
        Ok(())
    }
}

#[async_trait]
impl Engine for Arc<MQTTEngine> {
    async fn when(&self, path: &str, value: Vec<u8>) -> Result<()> {
        let s = Subscribe::new(vec![SubscribeTopic {
            topic_path: path.to_string(),
            qos: QoS::AtLeastOnce,
        }]);
        self.requests_tx.send(Request::Subscribe(s)).await?;

        let (tx, rx) = oneshot::channel();
        self.requests_tx
            .send(Request::Watch(Watch {
                path: path.to_string(),
                value,
                tx,
            }))
            .await?;
        Ok(rx.await?)
    }

    async fn set(&self, path: &str, value: Vec<u8>) -> Result<()> {
        let msg = Publish::new(path.to_string(), value);
        self.requests_tx.send(Request::Publish(msg)).await?;
        Ok(())
    }
}
