#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

use alloy::primitives::{Address, B256};

use rusty_safe_signing_adapters::{
    AbiAdapter, Eip1193Adapter, HashingAdapter, QueueAdapter, SafeServiceAdapter,
    WalletConnectAdapter,
};
use rusty_safe_signing_core::{ClockPort, Orchestrator, PortError, SigningCommand};

#[derive(Debug, Default)]
pub struct TestClock {
    now: AtomicU64,
}

impl ClockPort for TestClock {
    fn now_ms(&self) -> Result<u64, PortError> {
        Ok(self.now.fetch_add(1, Ordering::SeqCst) + 1_739_750_400_000)
    }
}

pub type TestOrchestrator = Orchestrator<
    Eip1193Adapter,
    SafeServiceAdapter,
    WalletConnectAdapter,
    QueueAdapter,
    AbiAdapter,
    HashingAdapter,
    TestClock,
>;

pub fn new_orchestrator() -> TestOrchestrator {
    Orchestrator::new(
        Eip1193Adapter::default(),
        SafeServiceAdapter::in_memory(),
        WalletConnectAdapter::in_memory(),
        QueueAdapter::default(),
        AbiAdapter,
        HashingAdapter,
        TestClock::default(),
    )
}

pub fn acquire_lock(orch: &TestOrchestrator) {
    orch.handle(SigningCommand::AcquireWriterLock {
        tab_id: "test-tab".to_owned(),
        tab_nonce: B256::ZERO,
        ttl_ms: 60_000,
    })
    .expect("acquire writer lock");
}

pub fn safe_address() -> Address {
    "0x000000000000000000000000000000000000BEEF"
        .parse()
        .expect("valid safe address")
}

pub fn owner_address() -> Address {
    "0x1000000000000000000000000000000000000001"
        .parse()
        .expect("valid owner address")
}

pub fn signature_bytes(seed: u8) -> alloy::primitives::Bytes {
    let mut v = vec![seed; 65];
    v[64] = 27;
    alloy::primitives::Bytes::from(v)
}
