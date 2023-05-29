use std::{
    fmt,
    net::SocketAddr,
    task::{Context, Poll},
    time::Instant,
};

use axum::extract::{ConnectInfo, MatchedPath};
use futures::Future;
use http::{Method, Request, Response};
use http_body::Body;
use prometheus::{register_histogram_vec, HistogramVec};
use tower_layer::Layer;
use tower_service::Service;

#[derive(Clone)]
pub struct LoggerLayer;

impl<S> Layer<S> for LoggerLayer {
    type Service = Logger<S>;

    fn layer(&self, service: S) -> Self::Service {
        Logger::new(service)
    }
}

#[derive(Clone)]
pub struct Logger<S> {
    inner: S,
}

impl<S> Logger<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

lazy_static::lazy_static! {
    static ref API_ELAPSED_METRIC: HistogramVec = register_histogram_vec!("command_api_elapsed_ms", "elapsed time & count of api responses", &["route", "status"]).unwrap();
}

#[pin_project::pin_project]
pub struct LoggerFuture<S, ReqBody, ResBody>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Error: fmt::Display + 'static,
{
    remote_addr: String,
    path: String,
    matched_path: String,
    level: log::Level,
    method: Method,
    start: Instant,
    #[pin]
    inner: S::Future,
}

impl<S, ReqBody, ResBody> Future for LoggerFuture<S, ReqBody, ResBody>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Error: fmt::Display + 'static,
{
    type Output = <S::Future as Future>::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(response)) => {
                //TODO: include a filtered query parameter list
                let elapsed = this.start.elapsed().as_secs_f64() * 1000.0;
                if !matches!(this.level, log::Level::Debug) {
                    API_ELAPSED_METRIC
                        .with_label_values(&[&*this.matched_path, response.status().as_str()])
                        .observe(elapsed);
                }
                log!(
                    *this.level,
                    "[{}] {} {} -> {} [{:.02} ms]",
                    this.remote_addr,
                    this.method,
                    this.path,
                    response.status(),
                    elapsed
                );
                Poll::Ready(Ok(response))
            }
            Poll::Ready(Err(e)) => {
                let elapsed = this.start.elapsed().as_secs_f64() * 1000.0;
                if !matches!(this.level, log::Level::Debug) {
                    API_ELAPSED_METRIC
                        .with_label_values(&[&*this.matched_path, "INTERNAL"])
                        .observe(elapsed);
                }

                log!(
                    *this.level,
                    "[{}] {} {} -> FAIL {} [{:.02} ms]",
                    this.remote_addr,
                    this.method,
                    this.path,
                    e,
                    elapsed
                );
                Poll::Ready(Err(e))
            }
        }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for Logger<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    ReqBody: Body,
    ResBody: Body,
    ResBody::Error: fmt::Display + 'static,
    S::Error: fmt::Display + 'static,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = LoggerFuture<S, ReqBody, ResBody>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let start = Instant::now();

        let path = req
            .uri()
            .path_and_query()
            .map(|x| x.to_string())
            .unwrap_or_default();
        let remote_addr = if let Some(forwarded) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|x| x.to_str().ok())
        {
            forwarded.to_string()
        } else {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .expect("missing ConnectInfo")
                .0
                .to_string()
        };
        let matched_path = req
            .extensions()
            .get::<MatchedPath>()
            .map(|x| x.as_str().to_string())
            .unwrap_or_default();

        let method = req.method().clone();
        let future = self.inner.call(req);
        let level = if path == "/api/v1/health" {
            log::Level::Debug
        } else {
            log::Level::Info
        };

        LoggerFuture {
            start,
            level,
            method,
            remote_addr,
            path,
            matched_path,
            inner: future,
        }
    }
}
