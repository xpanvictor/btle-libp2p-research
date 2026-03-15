mod btleplug;
mod types;
use core::fmt;
use std::task::{Context, Poll};

use futures::{AsyncRead, AsyncWrite, io};

pub enum BtleEvent<P, S> {
    PeerDiscovered { id: P },
    InviteReceived { id: P },
    InviteAccepted { id: P },
    SessionClosed { id: P },
    IncomingConnection { id: P, stream: S },
}

/// Minimal btle transport requirement
pub trait BtleTransport: Send + 'static {
    type BtlePeerData;
    type Stream: Send + Unpin + AsyncWrite + AsyncRead + fmt::Debug;
    type Error;

    fn start_broadcast(&mut self, data: Self::BtlePeerData) -> io::Result<()>;
    fn stop_broadcast(&mut self) -> io::Result<()>;
    fn scan_peers(&mut self) -> io::Result<()>;
    fn invite_peer(&mut self, peer: &Self::BtlePeerData) -> io::Result<()>;
    fn accept_peer(&mut self, peer: &Self::BtlePeerData) -> io::Result<()>;
    // todo
    fn negotiate_transport_upgrade();

    fn poll(
        _: Context<'_>,
    ) -> Poll<Result<BtleEvent<Self::BtlePeerData, Self::Stream>, Self::Error>>;
}
