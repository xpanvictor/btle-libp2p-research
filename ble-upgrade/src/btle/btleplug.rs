//! @author: xpanvictor
//! BtlePlug Implementation of the minimal btle spec

use std::collections::{HashMap, VecDeque};

use btleplug::{
    api::Manager,
    platform::{Adapter, Manager as PManager, Peripheral},
};

use crate::btle::types::{BtleStream, PID};

pub struct Peer {
    pub id: PID,
    pub peripheral: Peripheral,
}

pub struct BtlePlugTransport {
    adapter: Adapter,

    discovered: VecDeque<Peer>,
    incoming: VecDeque<(Peer, BtleStream)>,

    sessions: HashMap<String, BtleStream>,
}

impl BtlePlugTransport {
    pub async fn new() -> anyhow::Result<Self> {
        let manager = PManager::new().await?;
        let adapters = manager.adapters().await?;
        let adapter = adapters.into_iter().next().unwrap();

        Ok(Self {
            adapter,
            discovered: VecDeque::new(),
            incoming: VecDeque::new(),
            sessions: HashMap::new(),
        })
    }
}
