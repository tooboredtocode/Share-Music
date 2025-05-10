/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */
use crate::context::metrics::metrics_handler;
use crate::context::{ClusterState, Context};
use crate::util::error::expect_err;
use crate::util::{EmptyResult, create_termination_future};
use axum::Router;
use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::routing::get;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

pub async fn ready_handler(AxumState(context): AxumState<Arc<Context>>) -> (StatusCode, String) {
    match context.state.get() {
        ClusterState::Starting => (StatusCode::SERVICE_UNAVAILABLE, "Starting".to_string()),
        ClusterState::Running => (StatusCode::OK, "ok".to_string()),
        ClusterState::Terminating => (StatusCode::SERVICE_UNAVAILABLE, "Terminating".to_string()),
        ClusterState::Crashing => (StatusCode::SERVICE_UNAVAILABLE, "Crashing".to_string()),
    }
}

impl Context {
    pub async fn start_status_server(self: &Arc<Self>, port: u16) -> EmptyResult<()> {
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/healthz", get(|| async { "ok" }))
            .route("/readyz", get(ready_handler))
            .with_state(self.clone());

        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(expect_err!("Failed to bind to status address"))?;

        let state = self.clone();
        let termination_future = create_termination_future(&self.state);

        info!("Starting Status Server");
        tokio::spawn(async move {
            let res = axum::serve(listener, app)
                .with_graceful_shutdown(termination_future)
                .await
                .map_err(expect_err!("Status server crashed"));

            if res.is_err() {
                state.state.set(ClusterState::Crashing)
            }
        });

        Ok(())
    }
}
