use std::future::Future;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

/// A shared Tokio runtime for all Kubernetes API calls.
/// GPUI has its own async executor that doesn't include Tokio,
/// but kube-rs requires a Tokio runtime (for hyper/tower).
static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static Runtime {
    TOKIO_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

/// Spawn a future on the Tokio runtime and return a std::future
/// that can be awaited from GPUI's executor.
pub fn spawn_on_tokio<F, T>(future: F) -> impl Future<Output = T> + Send
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let handle = runtime().handle().clone();
    let join_handle = handle.spawn(future);
    async { join_handle.await.expect("Tokio task panicked") }
}
