use std::{
    os::unix::io::RawFd,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use smithay::{
    delegate_compositor, delegate_data_device, delegate_layer_shell, delegate_output,
    delegate_primary_selection, delegate_shm, delegate_tablet_manager, delegate_viewporter,
    delegate_xdg_activation, delegate_xdg_decoration, delegate_xdg_shell,
    desktop::{PopupManager, Space, WindowSurfaceType},
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, Mode, PostAction},
        wayland_protocols::xdg::decoration::{
            self as xdg_decoration,
            zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{wl_data_source::WlDataSource, wl_surface::WlSurface},
            Display, DisplayHandle, Resource,
        },
    },
    utils::{Logical, Point},
    wayland::{
        compositor::CompositorState,
        data_device::{
            set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
            ServerDndGrabHandler,
        },
        output::{Output, OutputManagerState},
        primary_selection::{set_primary_focus, PrimarySelectionHandler, PrimarySelectionState},
        seat::{CursorImageStatus, Seat, SeatHandler, SeatState, XkbConfig},
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{
                decoration::{XdgDecorationHandler, XdgDecorationState},
                ToplevelSurface, XdgShellState,
            },
        },
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
        tablet_manager::TabletSeatTrait,
        viewporter::ViewporterState,
        xdg_activation::{
            XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
        },
    },
};

#[cfg(feature = "xwayland")]
use crate::xwayland::X11State;
#[cfg(feature = "xwayland")]
use smithay::xwayland::{XWayland, XWaylandEvent};

pub struct CalloopData<BackendData: 'static> {
    pub state: LimeWmState<BackendData>,
    pub display: Display<LimeWmState<BackendData>>,
}

#[derive(Debug, Default)]
pub struct ClientState;
impl ClientData for ClientState {
    /// Notification that a client was initialized
    fn initialized(&self, _client_id: ClientId) {}
    /// Notification that a client is disconnected
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

#[derive(Debug)]
pub struct LimeWmState<BackendData: 'static> {
    pub backend_data: BackendData,
    pub socket_name: Option<String>,
    pub running: Arc<AtomicBool>,
    pub handle: LoopHandle<'static, CalloopData<BackendData>>,

    // desktop
    pub space: Space,
    pub popups: PopupManager,

    // smithay state
    pub compositor_state: CompositorState,
    pub data_device_state: DataDeviceState,
    pub layer_shell_state: WlrLayerShellState,
    pub output_manager_state: OutputManagerState,
    pub primary_selection_state: PrimarySelectionState,
    pub seat_state: SeatState<Self>,
    pub shm_state: ShmState,
    pub viewporter_state: ViewporterState,
    pub xdg_activation_state: XdgActivationState,
    pub xdg_decoration_state: XdgDecorationState,
    pub xdg_shell_state: XdgShellState,

    pub dnd_icon: Option<WlSurface>,
    pub log: slog::Logger,

    // input-related fields
    pub suppressed_keys: Vec<u32>,
    pub pointer_location: Point<f64, Logical>,
    pub cursor_status: Arc<Mutex<CursorImageStatus>>,
    pub seat_name: String,
    pub seat: Seat<Self>,
    pub start_time: std::time::Instant,

    #[cfg(feature = "xwayland")]
    pub xwayland: XWayland,
    #[cfg(feature = "xwayland")]
    pub x11_state: Option<X11State>,
}

delegate_compositor!(@<BackendData: Backend + 'static> LimeWmState<BackendData>);

impl<BackendData> DataDeviceHandler for LimeWmState<BackendData> {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
    fn send_selection(&mut self, _dh: &DisplayHandle, _mime_type: String, _fd: RawFd) {
        unreachable!("Lime_wm doesn't do server-side selections");
    }
}
impl<BackendData> ClientDndGrabHandler for LimeWmState<BackendData> {
    fn started(
        &mut self,
        _source: Option<WlDataSource>,
        icon: Option<WlSurface>,
        _seat: Seat<Self>,
    ) {
        self.dnd_icon = icon;
    }
    fn dropped(&mut self, _seat: Seat<Self>) {
        self.dnd_icon = None;
    }
}
impl<BackendData> ServerDndGrabHandler for LimeWmState<BackendData> {
    fn send(&mut self, _mime_type: String, _fd: RawFd) {
        unreachable!("Lime_wm doesn't do server-side grabs");
    }
}
delegate_data_device!(@<BackendData: 'static> LimeWmState<BackendData>);

delegate_output!(@<BackendData: 'static> LimeWmState<BackendData>);

impl<BackendData> PrimarySelectionHandler for LimeWmState<BackendData> {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}
delegate_primary_selection!(@<BackendData: 'static> LimeWmState<BackendData>);

impl<BackendData> ShmHandler for LimeWmState<BackendData> {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(@<BackendData: 'static> LimeWmState<BackendData>);

impl<BackendData> SeatHandler for LimeWmState<BackendData> {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

// This is a false positive
// Needs to be in a mod for allow to be used on macro
#[allow(clippy::use_self)]
mod x {
    smithay::delegate_seat!(@<BackendData: 'static> crate::LimeWmState<BackendData>);
}
pub use x::*;

delegate_tablet_manager!(@<BackendData: 'static> LimeWmState<BackendData>);

delegate_viewporter!(@<BackendData: 'static> LimeWmState<BackendData>);

impl<BackendData> XdgActivationHandler for LimeWmState<BackendData> {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn request_activation(
        &mut self,
        _dh: &DisplayHandle,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        if token_data.timestamp.elapsed().as_secs() < 10 {
            // Just grant the wish
            let w = self
                .space
                .window_for_surface(&surface, WindowSurfaceType::TOPLEVEL)
                .cloned();
            if let Some(window) = w {
                self.space.raise_window(&window, true);
            }
        } else {
            // Discard the request
            self.xdg_activation_state.remove_request(&token);
        }
    }

    fn destroy_activation(
        &mut self,
        _token: XdgActivationToken,
        _token_data: XdgActivationTokenData,
        _surface: WlSurface,
    ) {
        // The request is cancelled
    }
}
delegate_xdg_activation!(@<BackendData: 'static> LimeWmState<BackendData>);

impl<BackendData> XdgDecorationHandler for LimeWmState<BackendData> {
    fn new_decoration(&mut self, _dh: &DisplayHandle, toplevel: ToplevelSurface) {
        use xdg_decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
    }
    fn request_mode(
        &mut self,
        _dh: &DisplayHandle,
        _toplevel: ToplevelSurface,
        _mode: DecorationMode,
    ) {
    }
    fn unset_mode(&mut self, _dh: &DisplayHandle, _toplevel: ToplevelSurface) {}
}
delegate_xdg_decoration!(@<BackendData: Backend + 'static> LimeWmState<BackendData>);

delegate_xdg_shell!(@<BackendData: Backend + 'static> LimeWmState<BackendData>);
delegate_layer_shell!(@<BackendData: 'static> LimeWmState<BackendData>);

impl<BackendData: Backend + 'static> LimeWmState<BackendData> {
    pub fn init(
        display: &mut Display<Self>,
        handle: LoopHandle<'static, CalloopData<BackendData>>,
        backend_data: BackendData,
        log: slog::Logger,
        listen_on_socket: bool,
    ) -> Self {
        // init wayland clients
        let socket_name = if listen_on_socket {
            let source = ListeningSocketSource::new_auto(log.clone())
                .expect("Failed to create listening socket");
            let socket_name = source.socket_name().to_string_lossy().into_owned();
            handle
                .insert_source(source, |client_stream, _, data| {
                    if let Err(err) = data
                        .display
                        .handle()
                        .insert_client(client_stream, Arc::new(ClientState))
                    {
                        slog::warn!(data.state.log, "Error adding wayland client: {}", err);
                    };
                })
                .expect("Failed to init wayland socket source");
            info!(log, "Listening on wayland socket"; "name" => socket_name.clone());
            ::std::env::set_var("WAYLAND_DISPLAY", &socket_name);
            Some(socket_name)
        } else {
            None
        };
        handle
            .insert_source(
                Generic::new(display.backend().poll_fd(), Interest::READ, Mode::Level),
                |_, _, data| {
                    data.display
                        .dispatch_clients(&mut data.state)
                        .expect("Failed to dispatch requests");
                    Ok(PostAction::Continue)
                },
            )
            .expect("Failed to init wayland server source");

        // init globals
        let dh = display.handle();
        let compositor_state = CompositorState::new::<Self, _>(&dh, log.clone());
        let data_device_state = DataDeviceState::new::<Self, _>(&dh, log.clone());
        let layer_shell_state = WlrLayerShellState::new::<Self, _>(&dh, log.clone());
        let output_manager_state = OutputManagerState::new();
        let primary_selection_state = PrimarySelectionState::new::<Self, _>(&dh, log.clone());
        let seat_state = SeatState::new();
        let shm_state = ShmState::new::<Self, _>(&dh, vec![], log.clone());
        let viewporter_state = ViewporterState::new::<Self, _>(&dh, log.clone());
        let xdg_activation_state = XdgActivationState::new::<Self, _>(&dh, log.clone());
        let xdg_decoration_state = XdgDecorationState::new::<Self, _>(&dh, log.clone());
        let xdg_shell_state = XdgShellState::new::<Self, _>(&dh, log.clone());

        // init input
        let seat_name = backend_data.seat_name();
        let mut seat = Seat::new(&dh, seat_name.clone(), log.clone());

        let cursor_status = Arc::new(Mutex::new(CursorImageStatus::Default));
        let cursor_status2 = cursor_status.clone();
        seat.add_pointer(move |new_status| {
            *cursor_status2.lock().expect("Cursor status lock poisoned") = new_status;
        });

        seat.add_keyboard(XkbConfig::default(), 200, 25, move |seat, surface| {
            let focus = surface.and_then(|s| dh.get_client(s.id()).ok());
            let focus2 = surface.and_then(|s| dh.get_client(s.id()).ok());
            set_data_device_focus(&dh, seat, focus);
            set_primary_focus(&dh, seat, focus2);
        })
        .expect("Failed to initialize the keyboard");

        let cursor_status3 = cursor_status.clone();
        seat.tablet_seat()
            .on_cursor_surface(move |_tool, new_status| {
                // TODO: tablet tools should have their own cursors
                *cursor_status3.lock().expect("Cursor status lock poisoned") = new_status;
            });

        #[cfg(feature = "xwayland")]
        let xwayland = {
            let (xwayland, channel) = XWayland::new(log.clone(), &display.handle());
            let ret = handle.insert_source(channel, |event, _, data| match event {
                XWaylandEvent::Ready {
                    connection, client, ..
                } => data.state.xwayland_ready(connection, client),
                XWaylandEvent::Exited => data.state.xwayland_exited(),
            });
            if let Err(e) = ret {
                error!(
                    log,
                    "Failed to insert the XWaylandSource into the event loop: {}", e
                );
            }
            xwayland
        };

        Self {
            backend_data,
            socket_name,
            running: Arc::new(AtomicBool::new(true)),
            handle,
            space: Space::new(log.clone()),
            popups: PopupManager::new(log.clone()),
            compositor_state,
            data_device_state,
            layer_shell_state,
            output_manager_state,
            primary_selection_state,
            seat_state,
            shm_state,
            viewporter_state,
            xdg_activation_state,
            xdg_decoration_state,
            xdg_shell_state,
            dnd_icon: None,
            log,
            suppressed_keys: Vec::new(),
            pointer_location: (0.0, 0.0).into(),
            cursor_status,
            seat_name,
            seat,
            start_time: std::time::Instant::now(),
            #[cfg(feature = "xwayland")]
            xwayland,
            #[cfg(feature = "xwayland")]
            x11_state: None,
        }
    }
}

pub trait Backend {
    fn seat_name(&self) -> String;
    fn reset_buffers(&mut self, output: &Output);
    fn early_import(&mut self, surface: &WlSurface);
}
