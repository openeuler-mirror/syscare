use std::{sync::Arc, any::Any};

use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Sender};

#[derive(Debug, Clone)]
pub struct Event<'a, T> {
    pub topic: &'a str,
    pub payload: T,
}

#[derive(Debug, Clone)]
pub enum TrxEvent {
    Data(Arc<dyn Any>),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct TxChannel<'a> {
    topic: &'a str,
    inner: Sender<TrxEvent>,
}

impl<'a> TxChannel<'a> {
    // pub fn send<T: Any + 'a>(&'a self, payload: T) -> Result<()> {
    //     self.inner.send(TrxEvent::Data(Arc::new(Event {
    //         topic: &self.topic,
    //         payload,
    //     }))).map_err(|e| anyhow!(e.to_string()))
    // }
}

#[derive(Debug, Clone)]
pub struct RxChannel {
    topic: String,
    inner: Receiver<TrxEvent>,
}

impl RxChannel {
    pub fn recv(&self) -> Result<TrxEvent> {
        self.inner.recv().map_err(|e| anyhow!(e.to_string()))
    }
}
