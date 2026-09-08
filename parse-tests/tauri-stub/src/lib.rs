//! The slice of Tauri the provider layer actually touches.
//!
//! Everything under `src-tauri/src/providers/` reaches for exactly one
//! Tauri facility — `async_runtime` — plus `WebviewWindow` as an opaque
//! handle. Backing those with a plain Tokio runtime lets the harness
//! compile the real provider sources without linking Tauri itself.

pub mod async_runtime {
    use std::future::Future;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    /// Tauri keeps one process-wide runtime; so does this, so tasks spawned
    /// by provider code outlive the `block_on` that started them exactly as
    /// they do in the app.
    fn rt() -> &'static Runtime {
        static RT: OnceLock<Runtime> = OnceLock::new();
        RT.get_or_init(|| Runtime::new().expect("tokio runtime"))
    }

    pub fn block_on<F: Future>(fut: F) -> F::Output {
        rt().block_on(fut)
    }

    pub fn spawn<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        rt().spawn(fut)
    }

    pub fn spawn_blocking<F, R>(f: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        rt().spawn_blocking(f)
    }
}

/// Opaque in the harness: no provider code inspects it, it is only passed
/// through to the platform layer's WebView memory hint.
pub struct WebviewWindow;
