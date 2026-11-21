// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Entrypoint binary for the Rust Pub/Sub conformance proxy.

use std::io::Write;
use std::net::SocketAddr;

use pubsub_conformance_proxy::server::{
    ProxyService, ShutdownController, serve_with_grace_deadline,
};
use tokio::net::TcpListener;

fn parse_port_args<I>(args: I) -> Result<u16, String>
where
    I: IntoIterator<Item = String>,
{
    let mut port: u16 = 0;
    let mut iter = args.into_iter();
    let _prog = iter.next();
    while let Some(arg) = iter.next() {
        if let Some(val) = arg.strip_prefix("--port=") {
            port = val
                .parse::<u16>()
                .map_err(|e| format!("invalid --port value '{val}': {e}"))?;
        } else if arg == "--port" {
            let val = iter
                .next()
                .ok_or_else(|| "missing value after --port".to_string())?;
            port = val
                .parse::<u16>()
                .map_err(|e| format!("invalid --port value '{val}': {e}"))?;
        } else {
            return Err(format!("unrecognized argument: {arg}"));
        }
    }
    Ok(port)
}

async fn os_shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let port = parse_port_args(std::env::args())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let addr: SocketAddr = format!("127.0.0.1:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    let shutdown_ctrl = ShutdownController::new();
    let service = ProxyService::new(shutdown_ctrl);

    // Contract: emit READY line on stdout and flush immediately before serving.
    {
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "READY {local_addr}")?;
        stdout.flush()?;
    }

    serve_with_grace_deadline(listener, service, os_shutdown_signal()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_port_variants() {
        assert_eq!(parse_port_args(["proxy".into()]).unwrap(), 0);
        assert_eq!(
            parse_port_args(["proxy".into(), "--port=50051".into()]).unwrap(),
            50051
        );
        assert_eq!(
            parse_port_args(["proxy".into(), "--port".into(), "4321".into()]).unwrap(),
            4321
        );
        assert!(parse_port_args(["proxy".into(), "--port".into()]).is_err());
        assert!(parse_port_args(["proxy".into(), "--port=abc".into()]).is_err());
        assert!(parse_port_args(["proxy".into(), "--unknown".into()]).is_err());
    }
}
