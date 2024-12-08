use anyhow::Result;
use async_trait::async_trait;
use std::{collections::HashSet, sync::Arc};
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
    Get(Get),
}
#[derive(Debug)]
struct Get {
    path: String,
    tx: oneshot::Sender<Vec<u8>>,
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
        let mut watches: Vec<Get> = Vec::new();
        // Deduplicate subscriptions so we do not get busy loops getting messages of after
        // re-subscribing.
        let mut subscriptions: HashSet<String, _> = HashSet::new();
        loop {
            let s = select! {
                req = requests_rx.recv() =>  SelectResult::Request(req),
                data = cli.read_subscriptions() =>  SelectResult::Data(data?),
            };
            match s {
                SelectResult::Request(req) => match req {
                    Some(Request::Get(watch)) => watches.push(watch),
                    Some(Request::Publish(p)) => {
                        cli.publish(&p).await?;
                    }
                    Some(Request::Subscribe(s)) => {
                        let topic_paths: Vec<_> =
                            s.topics().iter().map(|t| t.topic_path.clone()).collect();
                        if topic_paths.iter().any(|t| !subscriptions.contains(t)) {
                            cli.subscribe(s).await?;
                        }
                        subscriptions.extend(topic_paths.into_iter());
                    }
                    None => break,
                },
                SelectResult::Data(data) => {
                    log::debug!(
                        "data receieved for topic {} {}",
                        data.topic(),
                        std::str::from_utf8(data.payload()).unwrap()
                    );
                    let mut i = 0_usize;
                    while i < watches.len() {
                        if data.topic() == watches[i].path {
                            let w = watches.remove(i);
                            w.tx.send(data.payload().to_vec()).unwrap();
                            continue;
                        }
                        i += 1;
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
    async fn get(&self, path: &str) -> Result<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        self.requests_tx
            .send(Request::Get(Get {
                path: path.to_string(),
                tx,
            }))
            .await?;
        // Subscribe after sending get so we are listening before we recieve the response
        let s = Subscribe::new(vec![SubscribeTopic {
            topic_path: path.to_string(),
            qos: QoS::AtLeastOnce,
        }]);
        self.requests_tx.send(Request::Subscribe(s)).await?;
        Ok(rx.await?)
    }

    async fn set(&self, path: &str, value: Vec<u8>) -> Result<()> {
        let msg = Publish::new(path.to_string(), value);
        self.requests_tx.send(Request::Publish(msg)).await?;
        Ok(())
    }
}
