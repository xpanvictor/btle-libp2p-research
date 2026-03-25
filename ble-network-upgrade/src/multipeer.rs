#[cfg(target_os = "macos")]
mod macos_impl {
    use block2::DynBlock;
    use objc2::rc::autoreleasepool;
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{Bool, ProtocolObject};
    use objc2::{define_class, msg_send, AnyThread, ClassType, DefinedClass};
    use objc2_foundation::{
        NSDate, NSDefaultRunLoopMode, NSData, NSDictionary, NSError, NSObject, NSObjectProtocol,
        NSRunLoop, NSString,
    };
    use objc2_multipeer_connectivity::{
        MCEncryptionPreference, MCNearbyServiceAdvertiser, MCNearbyServiceAdvertiserDelegate,
        MCNearbyServiceBrowser, MCNearbyServiceBrowserDelegate, MCPeerID, MCSession,
        MCSessionDelegate, MCSessionSendDataMode, MCSessionState,
    };
    use std::cell::{Cell, RefCell};
    use std::time::{Duration, Instant};

    fn peer_display_name(peer_id: &MCPeerID) -> String {
        autoreleasepool(|pool| {
            let name = unsafe { peer_id.displayName() };
            unsafe { name.to_str(pool) }.to_string()
        })
    }

    fn pump_runloop_slice(seconds: f64) {
        let run_loop = NSRunLoop::currentRunLoop();
        let limit = NSDate::dateWithTimeIntervalSinceNow(seconds);
        let mode = unsafe { &*NSDefaultRunLoopMode };
        let _ = run_loop.runMode_beforeDate(mode, &limit);
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MultiPeerRole {
        Initiator,
        Responder,
        Both,
    }

    impl MultiPeerRole {
        pub fn from_env() -> Self {
            match std::env::var("MP_ROLE")
                .unwrap_or_else(|_| "both".to_string())
                .to_lowercase()
                .as_str()
            {
                "initiator" => Self::Initiator,
                "responder" => Self::Responder,
                _ => Self::Both,
            }
        }

        fn is_initiator(self) -> bool {
            matches!(self, Self::Initiator | Self::Both)
        }

        fn is_responder(self) -> bool {
            matches!(self, Self::Responder | Self::Both)
        }
    }

    #[derive(Default)]
    struct DelegateState {
        role_is_initiator: Cell<bool>,
        session: RefCell<Option<Retained<MCSession>>>,
        received: RefCell<Vec<Vec<u8>>>,
        connected: Cell<bool>,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = objc2::MainThreadOnly]
        #[name = "BtleLibp2pMpDelegate"]
        #[ivars = DelegateState]
        struct MpDelegate;

        impl MpDelegate {
            #[unsafe(method_id(init))]
            fn init(this: Allocated<Self>) -> Retained<Self> {
                let this = this.set_ivars(DelegateState::default());
                unsafe { msg_send![super(this), init] }
            }
        }

        unsafe impl NSObjectProtocol for MpDelegate {}

        unsafe impl MCNearbyServiceBrowserDelegate for MpDelegate {
            #[unsafe(method(browser:foundPeer:withDiscoveryInfo:))]
            fn browser_found_peer(
                &self,
                browser: &MCNearbyServiceBrowser,
                peer_id: &MCPeerID,
                _info: Option<&NSDictionary<NSString, NSString>>,
            ) {
                println!(
                    "[MultiPeer] Browser discovered peer '{}'",
                    peer_display_name(peer_id)
                );

                if !self.ivars().role_is_initiator.get() {
                    return;
                }

                if let Some(session) = self.ivars().session.borrow().as_ref() {
                    println!("[MultiPeer] Sending invitation to discovered peer");
                    unsafe {
                        browser.invitePeer_toSession_withContext_timeout(peer_id, session, None, 10.0);
                    }
                }
            }

            #[unsafe(method(browser:lostPeer:))]
            fn browser_lost_peer(&self, _browser: &MCNearbyServiceBrowser, peer_id: &MCPeerID) {
                println!(
                    "[MultiPeer] Browser lost peer '{}'",
                    peer_display_name(peer_id)
                );
            }

            #[unsafe(method(browser:didNotStartBrowsingForPeers:))]
            fn browser_did_not_start(&self, _browser: &MCNearbyServiceBrowser, error: &NSError) {
                println!("[MultiPeer] Browser failed to start: {:?}", error);
            }
        }

        unsafe impl MCNearbyServiceAdvertiserDelegate for MpDelegate {
            #[unsafe(method(advertiser:didReceiveInvitationFromPeer:withContext:invitationHandler:))]
            fn advertiser_did_receive_invitation(
                &self,
                _advertiser: &MCNearbyServiceAdvertiser,
                peer_id: &MCPeerID,
                _context: Option<&NSData>,
                invitation_handler: &DynBlock<dyn Fn(Bool, *mut MCSession)>,
            ) {
                println!(
                    "[MultiPeer] Received invitation from '{}' (accepting)",
                    peer_display_name(peer_id)
                );

                let session_ptr = self
                    .ivars()
                    .session
                    .borrow()
                    .as_ref()
                    .map(|s| Retained::as_ptr(s) as *mut MCSession)
                    .unwrap_or(std::ptr::null_mut());

                invitation_handler.call((Bool::YES, session_ptr));
            }

            #[unsafe(method(advertiser:didNotStartAdvertisingPeer:))]
            fn advertiser_did_not_start(
                &self,
                _advertiser: &MCNearbyServiceAdvertiser,
                error: &NSError,
            ) {
                println!("[MultiPeer] Advertiser failed to start: {:?}", error);
            }
        }

        unsafe impl MCSessionDelegate for MpDelegate {
            #[unsafe(method(session:peer:didChangeState:))]
            fn session_peer_state(
                &self,
                _session: &MCSession,
                peer_id: &MCPeerID,
                state: MCSessionState,
            ) {
                self.ivars().connected.set(state == MCSessionState::Connected);
                println!(
                    "[MultiPeer] Peer '{}' state changed to {:?}",
                    peer_display_name(peer_id),
                    state
                );
            }

            #[unsafe(method(session:didReceiveData:fromPeer:))]
            fn session_did_receive_data(
                &self,
                _session: &MCSession,
                data: &NSData,
                peer_id: &MCPeerID,
            ) {
                println!(
                    "[MultiPeer] Received {} bytes from '{}'",
                    data.len(),
                    peer_display_name(peer_id)
                );
                self.ivars().received.borrow_mut().push(data.to_vec());
            }

            #[unsafe(method(session:didReceiveStream:withName:fromPeer:))]
            fn session_did_receive_stream(
                &self,
                _session: &MCSession,
                _stream: &objc2_foundation::NSInputStream,
                _stream_name: &NSString,
                _peer_id: &MCPeerID,
            ) {
            }

            #[unsafe(method(session:didStartReceivingResourceWithName:fromPeer:withProgress:))]
            fn session_did_start_resource(
                &self,
                _session: &MCSession,
                _resource_name: &NSString,
                _peer_id: &MCPeerID,
                _progress: &objc2_foundation::NSProgress,
            ) {
            }

            #[unsafe(method(session:didFinishReceivingResourceWithName:fromPeer:atURL:withError:))]
            fn session_did_finish_resource(
                &self,
                _session: &MCSession,
                _resource_name: &NSString,
                _peer_id: &MCPeerID,
                _local_url: Option<&objc2_foundation::NSURL>,
                _error: Option<&NSError>,
            ) {
            }
        }
    );

    impl MpDelegate {
        fn configure(&self, role: MultiPeerRole, session: Retained<MCSession>) {
            self.ivars().role_is_initiator.set(role.is_initiator());
            *self.ivars().session.borrow_mut() = Some(session);
        }

        fn is_connected(&self) -> bool {
            self.ivars().connected.get()
        }

        fn take_received(&self) -> Vec<Vec<u8>> {
            let mut out = Vec::new();
            let mut guard = self.ivars().received.borrow_mut();
            out.append(&mut *guard);
            out
        }
    }

    pub struct MultiPeerBackend {
        session: Retained<MCSession>,
        _peer_id: Retained<MCPeerID>,
        delegate: Retained<MpDelegate>,
        _browser: Option<Retained<MCNearbyServiceBrowser>>,
        _advertiser: Option<Retained<MCNearbyServiceAdvertiser>>,
    }

    impl MultiPeerBackend {
        pub fn start(
            display_name: &str,
            service_type: &str,
            role: MultiPeerRole,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let peer_name = NSString::from_str(display_name);
            let service = NSString::from_str(service_type);

            let peer_id = unsafe { MCPeerID::initWithDisplayName(MCPeerID::alloc(), &peer_name) };
            let session = unsafe {
                MCSession::initWithPeer_securityIdentity_encryptionPreference(
                    MCSession::alloc(),
                    &peer_id,
                    None,
                    MCEncryptionPreference::Required,
                )
            };

            let delegate: Retained<MpDelegate> = unsafe { msg_send![MpDelegate::class(), new] };
            delegate.configure(role, session.clone());
            unsafe {
                session.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            }

            let advertiser = if role.is_responder() {
                let adv = unsafe {
                    MCNearbyServiceAdvertiser::initWithPeer_discoveryInfo_serviceType(
                        MCNearbyServiceAdvertiser::alloc(),
                        &peer_id,
                        None,
                        &service,
                    )
                };
                unsafe {
                    adv.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
                    adv.startAdvertisingPeer();
                }
                Some(adv)
            } else {
                None
            };

            let browser = if role.is_initiator() {
                let br = unsafe {
                    MCNearbyServiceBrowser::initWithPeer_serviceType(
                        MCNearbyServiceBrowser::alloc(),
                        &peer_id,
                        &service,
                    )
                };
                unsafe {
                    br.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
                    br.startBrowsingForPeers();
                }
                Some(br)
            } else {
                None
            };

            Ok(Self {
                session,
                _peer_id: peer_id,
                delegate,
                _browser: browser,
                _advertiser: advertiser,
            })
        }

        pub fn wait_for_connection(
            &self,
            timeout: Duration,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let start = Instant::now();
            while start.elapsed() < timeout {
                pump_runloop_slice(0.03);

                let peers = unsafe { self.session.connectedPeers() };
                if !peers.is_empty() || self.delegate.is_connected() {
                    println!("[MultiPeer] Connected peers: {}", peers.len());
                    return Ok(());
                }

                std::thread::sleep(Duration::from_millis(20));
            }

            Err("Timed out waiting for Multipeer connection".into())
        }

        pub fn send(&self, data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
            let peers = unsafe { self.session.connectedPeers() };
            if peers.is_empty() {
                return Err("No connected Multipeer peers".into());
            }

            let payload = NSData::from_vec(data.to_vec());
            unsafe {
                self.session
                    .sendData_toPeers_withMode_error(
                        &payload,
                        &peers,
                        MCSessionSendDataMode::Reliable,
                    )
                    .map_err(|e| format!("sendData failed: {:?}", e))?;
            }
            Ok(data.len())
        }

        pub fn connected_peer_count(&self) -> usize {
            let peers = unsafe { self.session.connectedPeers() };
            peers.len()
        }

        pub fn drain_received(&self) -> Vec<Vec<u8>> {
            self.delegate.take_received()
        }

        pub fn stop(&self) {
            unsafe {
                self.session.disconnect();
            }
        }
    }

    pub use MultiPeerRole as Role;
}

#[cfg(target_os = "macos")]
pub use macos_impl::{MultiPeerBackend, Role as MultiPeerRole};

#[cfg(not(target_os = "macos"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiPeerRole {
    Initiator,
    Responder,
    Both,
}

#[cfg(not(target_os = "macos"))]
impl MultiPeerRole {
    pub fn from_env() -> Self {
        Self::Both
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MultiPeerBackend;

#[cfg(not(target_os = "macos"))]
impl MultiPeerBackend {
    pub fn start(
        _display_name: &str,
        _service_type: &str,
        _role: MultiPeerRole,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Err("Multipeer backend is only available on macOS".into())
    }

    pub fn wait_for_connection(
        &self,
        _timeout: std::time::Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Multipeer backend is only available on macOS".into())
    }

    pub fn send(&self, _data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        Err("Multipeer backend is only available on macOS".into())
    }

    pub fn connected_peer_count(&self) -> usize {
        0
    }

    pub fn drain_received(&self) -> Vec<Vec<u8>> {
        Vec::new()
    }

    pub fn stop(&self) {}
}
