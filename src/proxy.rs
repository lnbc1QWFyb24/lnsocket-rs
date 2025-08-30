//! TOR support for onion address connections

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

/// Default TOR proxy configuration (localhost:9050)
pub struct TorConfig {
    /// SOCKS5 proxy host (default "127.0.0.1" for local TOR)
    pub host: String,
    /// SOCKS5 proxy port (default 9050 for TOR)
    pub port: u16,
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 9050,
        }
    }
}

impl TorConfig {
    /// Create a new TorConfig with custom host and port
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    /// Get the proxy address as a string
    pub fn proxy_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// A unified stream type that can be either direct TCP or TOR SOCKS5
pub enum LnStream {
    /// Direct TCP stream
    Direct(TcpStream),
    /// TOR SOCKS5 proxied stream
    Tor(Socks5Stream<TcpStream>),
}

impl AsyncRead for LnStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            LnStream::Direct(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            LnStream::Tor(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for LnStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            LnStream::Direct(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            LnStream::Tor(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            LnStream::Direct(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            LnStream::Tor(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            LnStream::Direct(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            LnStream::Tor(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
        }
    }
}
