//!
//! Rerun Node
//!
//! This Node natively integrates with the rerun data visualization
//! framework by providing methods to enable the creation of NComm
//! types to perform rerun functionalities.
//!
//! To use this node, please ensure rerun-cli is installed on the
//! local system.  Currently, this is accomplishable via the command:
//! ```sh
//! cargo install rerun-cli --locked
//! ```
//!
//! If this changes in the future, I'll update these docs, however,
//! the main source of truth should be the [Rerun Documentation](https://rerun.io/)
//!

use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[cfg(feature = "rerun-web-viewer")]
use re_web_viewer_server::WebViewerServerPort;
#[cfg(feature = "rerun-web-viewer")]
use re_ws_comms::RerunServerPort;
#[cfg(feature = "rerun-web-viewer")]
use rerun::MemoryLimit;
use rerun::{
    ApplicationId, AsComponents, EntityPath, RecordingStream, RecordingStreamBuilder,
    RecordingStreamError,
};

use ncomm_core::Node;
use ncomm_publishers_and_subscribers::rerun::{RerunPublisher, RerunTimestampedPublisher};

/// The Rerun Node.
///
/// This Node provides a common interface for NComm applications
/// to utilize Rerun for data visualization
pub struct RerunNode<
    Id: PartialEq + Clone + Send + 'static,
    Path: Into<PathBuf> + Clone + Send + 'static,
> {
    /// The identifier for the RerunNode
    id: Id,
    /// The rerun recording stream
    stream: RecordingStream,
    /// The name of the file to save the collected data to
    output_path: Option<Path>,
}

impl<Id: PartialEq + Clone + Send + 'static, Path: Into<PathBuf> + Clone + Send + 'static>
    RerunNode<Id, Path>
{
    /// Create a new Rerun node that logs to a specified path on the system
    pub fn new(
        application_id: impl Into<ApplicationId>,
        output_path: Path,
        id: Id,
    ) -> Result<Self, RecordingStreamError> {
        let stream = RecordingStreamBuilder::new(application_id).save(output_path)?;

        Ok(Self {
            id,
            stream,
            output_path: None,
        })
    }

    /// Create a new Rerun node that logs to the default remote Rerun server
    pub fn new_remote_default(
        application_id: impl Into<ApplicationId>,
        output_path: Option<Path>,
        id: Id,
    ) -> Result<Self, RecordingStreamError> {
        let stream = RecordingStreamBuilder::new(application_id).connect()?;

        Ok(Self {
            id,
            stream,
            output_path,
        })
    }

    /// Create a new Rerun node that logs to a specified remote Rerun server
    pub fn new_remote(
        application_id: impl Into<ApplicationId>,
        address: SocketAddr,
        flush_timeout: Option<Duration>,
        output_path: Option<Path>,
        id: Id,
    ) -> Result<Self, RecordingStreamError> {
        let stream =
            RecordingStreamBuilder::new(application_id).connect_opts(address, flush_timeout)?;

        Ok(Self {
            id,
            stream,
            output_path,
        })
    }

    /// Create a new Rerun node that launches a new local Rerun viewer with default options when created
    pub fn new_rerun_spawn(
        application_id: impl Into<ApplicationId>,
        output_path: Option<Path>,
        id: Id,
    ) -> Result<Self, RecordingStreamError> {
        let stream = RecordingStreamBuilder::new(application_id).spawn()?;

        Ok(Self {
            id,
            stream,
            output_path,
        })
    }

    /// Create a new Rerun node that launches a new Rerun server over a given websocket configuration
    #[cfg(feature = "rerun-web-viewer")]
    pub fn new_rerun_server(
        application_id: impl Into<ApplicationId>,
        bind_ip: &str,
        web_port: WebViewerServerPort,
        ws_port: RerunServerPort,
        server_memory_limit: MemoryLimit,
        open_browser: bool,
        output_path: Option<Path>,
        id: Id,
    ) -> Result<Self, RecordingStreamError> {
        let stream = RecordingStreamBuilder::new(application_id).serve(
            bind_ip,
            web_port,
            ws_port,
            server_memory_limit,
            open_browser,
        )?;

        Ok(Self {
            id,
            stream,
            output_path,
        })
    }

    /// Create a publisher for the same Rerun stream referenced by this Node.  This publisher
    /// publishes a data and uses the publishing timestamp as the timestamp for data collection.
    pub fn create_rerun_publisher<LogPath: Into<EntityPath> + Clone, Arch: AsComponents>(
        &mut self,
        path: LogPath,
    ) -> RerunPublisher<LogPath, Arch> {
        RerunPublisher::new(self.stream.clone(), path)
    }

    /// Create a publisher for the same Rerun streamed referenced to by this Node.  This
    /// publisher publishes with a supplied timestamp so do not use if you want rerun to use its
    /// internal clock for published data
    pub fn create_rerun_timestamped_publisher<
        LogPath: Into<EntityPath> + Clone,
        Arch: AsComponents,
    >(
        &mut self,
        path: LogPath,
    ) -> RerunTimestampedPublisher<LogPath, Arch> {
        RerunTimestampedPublisher::new(self.stream.clone(), path)
    }
}

impl<Id: PartialEq + Clone + Send + 'static, Path: Into<PathBuf> + Clone + Send + 'static> Node<Id>
    for RerunNode<Id, Path>
{
    fn get_id(&self) -> Id {
        self.id.clone()
    }

    fn get_update_delay_us(&self) -> u128 {
        10_000_000
    }

    fn start(&mut self) {
        self.stream.connect();
    }

    fn shutdown(&mut self) {
        if let Some(path) = self.output_path.as_ref() {
            let _ = self.stream.save(path.clone());
        }
        self.stream.disconnect();
    }
}
