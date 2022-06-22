use anyhow::Result;
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, oneshot},
};

use crate::vm::Engine;

use mqtt_async_client::client::{Client, Publish, QoS, ReadResult, Subscribe, SubscribeTopic};

#[derive(Debug)]
pub struct MQTTEngine {
    requests_tx: mpsc::Sender<Request>,
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
    value: String,
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
        tokio::spawn(async move {
            Self::run(cli, requests_rx).await.unwrap();
        });
        Ok(Arc::new(Self { requests_tx }))
    }
    async fn run(mut cli: Client, mut requests_rx: mpsc::Receiver<Request>) -> Result<()> {
        cli.connect().await?;
        let mut watches: Vec<Watch> = Vec::new();
        loop {
            let req_fut = Box::pin(requests_rx.recv().fuse());
            let data_fut = Box::pin(async { cli.read_subscriptions().await }.fuse());
            let s = select! {
                req = req_fut =>  SelectResult::Request(req),
                data = data_fut =>  SelectResult::Data(data.unwrap()),
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
                        if data.topic() == watches[i].path
                            && data.payload() == watches[i].value.as_bytes()
                        {
                            let w = watches.remove(i);
                            w.tx.send(()).unwrap();
                            continue;
                        }
                        i = i + 1;
                    }
                }
            }
        }
        Ok(cli.disconnect().await?)
    }
}

#[async_trait]
impl Engine for Arc<MQTTEngine> {
    async fn when(&self, path: &str, value: &str) -> Result<()> {
        let s = Subscribe::new(vec![SubscribeTopic {
            topic_path: path.to_string(),
            qos: QoS::AtLeastOnce,
        }]);
        self.requests_tx.send(Request::Subscribe(s)).await?;

        let (tx, rx) = oneshot::channel();
        self.requests_tx
            .send(Request::Watch(Watch {
                path: path.to_string(),
                value: value.to_string(),
                tx,
            }))
            .await?;
        Ok(rx.await?)
    }

    async fn set(&self, path: &str, value: &str) -> Result<()> {
        let msg = Publish::new(path.to_string(), value.as_bytes().to_vec());
        self.requests_tx.send(Request::Publish(msg)).await?;
        Ok(())
    }

    async fn get(&self, _path: &str) -> Result<String> {
        todo!()
    }
}
