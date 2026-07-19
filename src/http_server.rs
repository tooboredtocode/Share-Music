/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use std::net::SocketAddr;

use axum::Router;
use axum::routing::{MethodRouter, get};
use metronomos::lifecycle::{Lifecycle, LifecycleContext};
use metronomos_pulse::value::{ArcValue, GroupValues, PulseValue};
use tracing::{error, info};

use crate::args::Args;

#[derive(Clone, PulseValue)]
pub struct HttpServeRoute {
    pub path: &'static str,
    pub router: MethodRouter<()>,
}

async fn start_http_server(ctx: LifecycleContext, router: Router, port: u16) {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind to port {}: {}", port, e);
            ctx.notify_error();
            return;
        }
    };

    info!("Starting Status Server");

    if let Err(e) = axum::serve(listener, router)
        .with_graceful_shutdown(ctx.wait_for_shutdown_owned())
        .await
    {
        error!("Error while running axum server, shutting down: {}", e);
        ctx.notify_error();
    }
}

pub fn provide_http_server(
    lifecycle: Lifecycle,
    routes: GroupValues<HttpServeRoute>,
    args: ArcValue<Args>,
) {
    let mut app = Router::new().route("/healthz", get(|| async { "ok" }));

    for route in routes {
        app = app.route(route.path, route.router);
    }

    lifecycle
        .hook(move |ctx| start_http_server(ctx, app.clone(), args.metrics_port))
        .disable_timeout();
}
