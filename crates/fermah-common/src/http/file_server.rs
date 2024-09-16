use std::{net::SocketAddr, path::PathBuf};

use tracing::info;
use warp::Filter;

/// A simple file server that serves a single file over HTTP.
pub struct FileServer {
    pub addr: SocketAddr,
}

impl FileServer {
    pub fn new(port: u16) -> Self {
        let addr = SocketAddr::new([0, 0, 0, 0].into(), port);
        Self { addr }
    }

    pub async fn serve_dir(&self, path: String, dir: PathBuf) {
        let route = warp::get().and(warp::path(path)).and(warp::fs::dir(dir));

        info!("starting file server on {}", self.addr);
        warp::serve(route).run(self.addr).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_server() {
        tokio::spawn(async {
            FileServer::new(3000)
                .serve_dir("files".to_string(), "./".into())
                .await;
        });

        let res = reqwest::get("http://localhost:3000/files/Cargo.toml")
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
    }
}
