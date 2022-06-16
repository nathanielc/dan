use anyhow::Result;
use async_trait::async_trait;
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

#[async_trait]
impl Engine for Arc<MQTTEngine> {
    async fn when(&self, _path: &str, _value: &str) -> Result<()> {
        todo!()
    }

    async fn set(&self, path: &str, value: &str) -> Result<()> {
        let msg = mqtt::Publish::new(path.to_string(), value.as_bytes().to_vec());
        self.cli.publish(&msg).await?;
        Ok(())
    }

    async fn get(&self, _path: &str) -> Result<String> {
        todo!()
    }
}
