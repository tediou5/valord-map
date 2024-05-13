use std::sync::Arc;
use tokio::sync::watch;

pub struct Watcher<V> {
    rx: watch::Receiver<Option<Arc<V>>>,
}

impl<V> Watcher<V> {
    pub async fn head_changed(&mut self) -> Result<Option<Arc<V>>, watch::error::RecvError> {
        self.rx.changed().await?;
        Ok(self.rx.borrow().clone())
    }
}

impl<V> From<watch::Receiver<Option<Arc<V>>>> for Watcher<V> {
    fn from(rx: watch::Receiver<Option<Arc<V>>>) -> Self {
        Self { rx }
    }
}
