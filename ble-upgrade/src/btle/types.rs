use std::task::Poll;

use futures::{AsyncRead, AsyncWrite};
use libp2p::PeerId;

pub type PID = PeerId;

pub struct BtleStream {
    rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl AsyncRead for BtleStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.rx.try_recv() {
            Ok(data) => {
                let len = data.len().min(buf.len());
                buf[..len].copy_from_slice(&data[..len]);
                Poll::Ready(Ok(len))
            }
            Err(_) => Poll::Pending,
        }
    }
}

impl AsyncWrite for BtleStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.tx.try_send(buf.to_vec()) {
            Ok(_) => Poll::Ready(Ok(buf.len())),
            Err(_) => Poll::Pending,
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
