//!
//! Publishers for integration with the Rerun data visualizer
//!
//! In general, I would advise instantiating the Rerun Node from the
//! ncomm-nodes crate and creating these publishers off of that Node.
//! However, this isn't technically necessary and each of these publishers
//! can be created without the Rerun Node.
//!
//! To actually use rerun it is usually necessary to install the rerun-cli
//! which can be installed via the command:
//! ```sh
//! cargo install rerun-cli --locked
//! ```
//!
//! If this changes in the future, I'll update these docs, however,
//! the main source of truth for all things Rerun should always be the
//! [Rerun Documentation](https://rerun.io/)
//!

use std::marker::PhantomData;

use quanta::{Clock, Instant};

use rerun::{
    external::re_types::blueprint::components::TimelineName, AsComponents, EntityPath,
    RecordingStream, RecordingStreamError,
};

use ncomm_core::Publisher;

/// Rerun Publisher that publishes a specific type of Archetype to a given path
/// logging the current Rerun timestamp as the current timestamp of the Archetype.
pub struct RerunPublisher<Path: Into<EntityPath> + Clone, Arch: AsComponents> {
    /// The quanta reference clock for publishing
    clock: Clock,
    /// The start timestamp as a reference for publishing
    start_instant: Instant,
    /// The Rerun stream to log to
    stream: RecordingStream,
    /// The path to log data to
    path: Path,
    /// Marker to denote the datatype that can be published via this publisher
    _phantom: PhantomData<Arch>,
}

impl<Path: Into<EntityPath> + Clone, Arch: AsComponents> RerunPublisher<Path, Arch> {
    /// Create a new RerunPublisher.
    ///
    /// Note: I would advise using the RerunNode in the ncomm-nodes crate
    /// to create this publisher, however I can't stop you from creating
    /// the publisher this way and it'll work the same way.
    pub fn new(stream: RecordingStream, path: Path) -> Self {
        let clock = Clock::new();
        let start_instant = clock.now();

        Self {
            clock,
            start_instant,
            stream,
            path,
            _phantom: PhantomData,
        }
    }

    /// Reset the start instant for the reference clock
    pub fn start(&mut self) {
        self.start_instant = self.clock.now();
    }
}

impl<Path: Into<EntityPath> + Clone, Arch: AsComponents> Publisher for RerunPublisher<Path, Arch> {
    type Data = Arch;
    type Error = RecordingStreamError;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        self.stream.set_time_nanos(
            "time (ns)",
            self.clock
                .now()
                .duration_since(self.start_instant)
                .as_nanos() as i64,
        );
        self.stream.log(self.path.clone(), &data)
    }
}

/// Rerun Publisher that allows users to specify the time to log a specific piece of published data.
pub struct RerunTimestampedPublisher<Path: Into<EntityPath> + Clone, Arch: AsComponents> {
    /// The Rerun stream to log to
    stream: RecordingStream,
    /// The path to log data to
    path: Path,
    /// Marker to denote the datatype that can be published via this publisher
    _phantom: PhantomData<Arch>,
}

impl<Path: Into<EntityPath> + Clone, Arch: AsComponents> RerunTimestampedPublisher<Path, Arch> {
    /// Create a new RerunTimestampedPublisher.
    ///
    /// Note: I would advise towards using the RerunNoe in ncomm-nodes to create
    /// this publisher, but instantiating this publisher on its own will work
    /// perfectly fine so do whichever works for you I guess.
    pub fn new(stream: RecordingStream, path: Path) -> Self {
        Self {
            stream,
            path,
            _phantom: PhantomData,
        }
    }
}

impl<Path: Into<EntityPath> + Clone, Arch: AsComponents> Publisher
    for RerunTimestampedPublisher<Path, Arch>
{
    type Data = (Arch, TimelineName, i64);
    type Error = RecordingStreamError;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        self.stream.set_time_sequence(data.1, data.2);
        self.stream.log(self.path.clone(), &data.0)
    }
}
