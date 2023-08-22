use std::sync::Arc;
use tokio::sync::RwLock;

use kube::Client;

use super::{Context, Diagnostics};

pub struct State {
    /// Diagnostics populated by the reconciler
    diagnostics: Arc<RwLock<Diagnostics>>,
    image: String,
}

impl State {
    pub fn new(image: String) -> Self {
        Self {
            diagnostics: Arc::new(RwLock::new(Diagnostics::default())),
            image,
        }
    }

    pub(crate) fn to_context(&self, client: Client) -> Arc<Context> {
        Arc::new(Context {
            client,
            // metrics: Metrics::default().register(&self.registry).unwrap(),
            diagnostics: self.diagnostics.clone(),
            image: self.image.clone(),
        })
    }
}
