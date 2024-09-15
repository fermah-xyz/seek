use std::{collections::HashMap, sync::Arc};

use thiserror::Error;
use tokio::sync::{
    oneshot::{error::TryRecvError, Receiver},
    Mutex,
};
use tracing::{debug, error};

use crate::hash::blake3::Blake3Hash;

#[derive(Error, Debug)]
pub enum ReleaseError {
    #[error("Releasing hash {0}, but it is not registered")]
    NotRegistered(Blake3Hash),

    #[error("Releasing hash {0}, but channel closed")]
    ChannelClosed(Blake3Hash),
}

#[derive(Clone, Default)]
pub struct Releaser {
    inner: Arc<Mutex<HashMap<Blake3Hash, tokio::sync::oneshot::Sender<()>>>>,
}

impl Releaser {
    pub async fn release(&self, hash: &Blake3Hash) -> Result<(), ReleaseError> {
        let mut inner = self.inner.lock().await;

        // Releasing multiple times, if definitely possible, because we want to insure calling release eventually, but
        // it will most likely be released in a timely manner. It is less likely to be called once than twice. So,
        // we should expect that there will be no desired releaser present, but if it is, and failes - that is a problem.
        if let Some(releaser) = inner.remove(hash) {
            if releaser.send(()).is_err() {
                error!(?hash, "Failed to send release signal");
                return Err(ReleaseError::ChannelClosed(*hash));
            }
        } else {
            // Not found, but most likely was just removed
            debug!(?hash, "Releaser no found, which is ok")
            // return Err(ReleaseError::NotRegistered(hash.clone()))
        }

        Ok(())
    }

    pub async fn register(&self, hash: Blake3Hash) -> ReleaserReceiver {
        let mut inner = self.inner.lock().await;
        let (s, r) = tokio::sync::oneshot::channel();
        inner.insert(hash, s);
        ReleaserReceiver::new(r)
    }
}

pub struct ReleaserReceiver {
    receiver: Receiver<()>,
}

impl ReleaserReceiver {
    fn new(receiver: Receiver<()>) -> Self {
        Self { receiver }
    }

    pub fn is_released(&mut self) -> bool {
        match self.receiver.try_recv() {
            Ok(_) => true,
            Err(TryRecvError::Closed) => true,
            Err(TryRecvError::Empty) => false,
        }
    }
}
