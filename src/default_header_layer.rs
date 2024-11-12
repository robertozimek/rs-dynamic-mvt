use axum::http::HeaderMap;
use axum::{
    extract::Request
    ,
    response::Response,
};
use futures_util::future::BoxFuture;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct DefaultHeaderLayer {
    default_headers: HeaderMap,
}

impl DefaultHeaderLayer {
    pub fn new(default_headers: HeaderMap) -> Self {
        Self { default_headers }
    }
}

impl<S> Layer<S> for DefaultHeaderLayer
{
    type Service = DefaultHeaderMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefaultHeaderMiddleware { inner, default_headers: self.default_headers.clone() }
    }
}

#[derive(Clone)]
pub struct DefaultHeaderMiddleware<S> {
    default_headers: HeaderMap,
    inner: S,
}

impl<S> Service<Request> for DefaultHeaderMiddleware<S>
where
    S: Service<Request, Response=Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let pinned_headers = Box::new(self.default_headers.clone());
        let future = self.inner.call(request);
        Box::pin(async move {
            let mut response: Response = future.await?;
            let headers = response.headers_mut();
            for (name, value) in pinned_headers.iter() {
                if !headers.contains_key(name) {
                    headers.insert(name, value.clone());
                }
            }

            Ok(response)
        })
    }
}