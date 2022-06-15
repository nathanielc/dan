use anyhow::Result;
use std::sync::Arc;

use crate::vm::Engine;

use mqtt_async_client::client as mqtt;

#[derive(Debug)]
pub struct MQTTEngine {
    cli: mqtt::Client,
}

impl MQTTEngine {
    pub fn new() -> Result<Self> {
        // Create a client & define connect options
        let cli = mqtt::Client::builder()
            .set_url_string("mqtt://localhost")?
            .build()?;

        return Ok(Self { cli });
    }
    pub async fn connect(&mut self) -> Result<()> {
        Ok(self.cli.connect().await?)
    }
    pub async fn disconnect(&mut self) -> Result<()> {
        Ok(self.cli.disconnect().await?)
    }
}

impl<'a> Engine<'a> for Arc<MQTTEngine> {
    fn when(&self, _path: &str, _value: &str) -> futures::future::BoxFuture<'a, Option<String>> {
        todo!()
    }

    fn set(&self, path: &str, value: &str) -> futures::future::BoxFuture<'a, Option<String>> {
        let s = self.clone();
        let path = path.to_string();
        let value = value.as_bytes().to_vec();
        Box::pin(async move {
            log::debug!("set {}", &path);
            let msg = mqtt::Publish::new(path, value);
            s.cli.publish(&msg).await.unwrap();
            log::debug!("set done");
            None
        })
    }

    fn get(&self, _path: &str) -> futures::future::BoxFuture<'a, Option<String>> {
        todo!()
    }
}
