//! Request completion hooks for embedding applications.

use std::{
    net::SocketAddr,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, RwLock},
    time::Duration,
};

use hyper::{HeaderMap, Method, StatusCode, Uri};
use log::error;

/// The protocol used to serve a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestProtocol {
    Http,
    Https,
}

/// An owned summary emitted after Chimney has produced a response.
#[derive(Debug, Clone)]
pub struct RequestEvent {
    pub method: Method,
    pub uri: Uri,
    pub request_headers: HeaderMap,
    pub host: Option<String>,
    pub site: Option<String>,
    pub status: StatusCode,
    pub response_headers: HeaderMap,
    pub duration: Duration,
    pub remote_addr: Option<SocketAddr>,
    pub protocol: RequestProtocol,
}

type RequestHook = Arc<dyn Fn(&RequestEvent) + Send + Sync + 'static>;

#[derive(Clone, Default)]
pub(crate) struct RequestHooks(Arc<RwLock<Vec<RequestHook>>>);

impl RequestHooks {
    pub(crate) fn add<F>(&self, hook: F)
    where
        F: Fn(&RequestEvent) + Send + Sync + 'static,
    {
        self.0
            .write()
            .expect("request hook lock poisoned")
            .push(Arc::new(hook));
    }

    pub(crate) fn notify(&self, event: &RequestEvent) {
        let hooks = self.0.read().expect("request hook lock poisoned").clone();
        for hook in hooks {
            if catch_unwind(AssertUnwindSafe(|| hook(event))).is_err() {
                error!("A request completion hook panicked");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn event() -> RequestEvent {
        RequestEvent {
            method: Method::GET,
            uri: "/health".parse().unwrap(),
            request_headers: HeaderMap::new(),
            host: Some("example.com".to_string()),
            site: Some("example".to_string()),
            status: StatusCode::OK,
            response_headers: HeaderMap::new(),
            duration: Duration::from_millis(2),
            remote_addr: None,
            protocol: RequestProtocol::Http,
        }
    }

    #[test]
    fn notifies_all_hooks_and_isolates_panics() {
        let hooks = RequestHooks::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let first = calls.clone();
        hooks.add(move |_| {
            first.fetch_add(1, Ordering::Relaxed);
        });
        hooks.add(|_| panic!("hook failure"));
        let last = calls.clone();
        hooks.add(move |_| {
            last.fetch_add(1, Ordering::Relaxed);
        });

        hooks.notify(&event());
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }
}
