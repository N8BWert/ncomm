//!
//! NComm Publisher and Subscriber for publishing over Serial using the embedded-io traits.
//!
//! This publisher and subscriber send and receive data over the serial
//! peripherals of whatever system is being utilized.
//!

use core::marker::PhantomData;

use embedded_io::{Error, Read, ReadReady, Write};

use ncomm_core::publisher_subscriber::{Publisher, Subscriber};
use ncomm_utils::packing::{Packable, PackingError};

/// An Error regarding publishing serial data
#[derive(Debug)]
pub enum SerialPublishError<Err: Error> {
    /// Embedded-IO Error
    IOError(Err),
    /// An error occurred with packing the data
    PackingError(PackingError),
}

/// Publisher that publishes data via a serial device.
///
/// To make this publisher no_std compatible the publisher has an internal buffer
/// that is statically allocated, hence the reason for the const BUFFER_SIZE: usize
/// generic
pub struct SerialPublisher<
    Data: Packable,
    Serial: Write<Error = Err>,
    Err: Error,
    const BUFFER_SIZE: usize,
> {
    /// The serial peripheral device
    serial_device: Serial,
    /// The internal buffer for encoding data
    buffer: [u8; BUFFER_SIZE],
    /// A marker to bind the type of data published to the publisher
    _phantom: PhantomData<Data>,
}

impl<Data, Serial, Err, const BUFFER_SIZE: usize> SerialPublisher<Data, Serial, Err, BUFFER_SIZE>
where
    Data: Packable,
    Serial: Write<Error = Err>,
    Err: Error,
{
    /// Create a new SerialPublisher from the peripheral
    pub fn new(serial_device: Serial, buffer: [u8; BUFFER_SIZE]) -> Self {
        assert!(
            BUFFER_SIZE >= Data::len(),
            "The buffer must be large enough to fit encoded data"
        );
        Self {
            serial_device,
            buffer,
            _phantom: PhantomData,
        }
    }

    /// Destroy the SerialPublisher returning the serial peripheral
    pub fn destroy(self) -> Serial {
        self.serial_device
    }
}

impl<Data, Serial, Err, const BUFFER_SIZE: usize> Publisher
    for SerialPublisher<Data, Serial, Err, BUFFER_SIZE>
where
    Data: Packable,
    Serial: Write<Error = Err>,
    Err: Error,
{
    type Data = Data;
    type Error = SerialPublishError<Err>;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        self.buffer.iter_mut().for_each(|v| *v = 0);
        data.pack(&mut self.buffer)
            .map_err(SerialPublishError::PackingError)?;

        self.serial_device
            .write_all(&self.buffer)
            .map_err(SerialPublishError::IOError)?;

        Ok(())
    }
}

/// Serial Subscriber that reads data from a serial line
///
/// Note: To make this subscriber no_std compatible the subscriber
/// has an internal buffer that is statically allocated, hence the reason
/// for the const BUFFER_SIZE: usize generic
pub struct SerialSubscriber<
    Data: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err>,
    Err: Error,
    const BUFFER_SIZE: usize,
> {
    /// The serial peripheral device
    serial_device: Serial,
    /// The internal buffer for decoding data
    buffer: [u8; BUFFER_SIZE],
    /// The current data stored in the subscriber
    data: Option<Data>,
}

impl<Data, Serial, Err, const BUFFER_SIZE: usize> SerialSubscriber<Data, Serial, Err, BUFFER_SIZE>
where
    Data: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err>,
    Err: Error,
{
    /// Create a new SerialSubscriber from the peripheral
    pub fn new(serial_device: Serial, buffer: [u8; BUFFER_SIZE]) -> Self {
        assert!(
            BUFFER_SIZE >= Data::len(),
            "The buffer must be large enough to fit encoded data"
        );
        Self {
            serial_device,
            buffer,
            data: None,
        }
    }

    /// Destroy the SerialSubscriber returning the serial peripheral
    pub fn destroy(self) -> Serial {
        self.serial_device
    }
}

impl<Data, Serial, Err, const BUFFER_SIZE: usize> Subscriber
    for SerialSubscriber<Data, Serial, Err, BUFFER_SIZE>
where
    Data: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err>,
    Err: Error,
{
    type Target = Option<Data>;

    fn get(&mut self) -> &Self::Target {
        let mut new_data = None;

        while let Ok(ready) = self.serial_device.read_ready() {
            if !ready {
                break;
            }

            self.buffer.iter_mut().for_each(|v| *v = 0);
            if self.serial_device.read(&mut self.buffer).is_ok() {
                if let Ok(data) = Data::unpack(&self.buffer) {
                    new_data = Some(data);
                }
            }
        }

        if let Some(data) = new_data {
            self.data = Some(data);
        }

        &self.data
    }
}

/// A serial publisher/subscriber capable of both publishing and subscribing
/// a specific data type.
///
/// Note: To make this subscriber no_std compatible the subscriber
/// has an internal buffer that is statically allocated, hence the reason
/// for the const BUFFER_SIZE: usize generic
pub struct SerialPublisherSubscriber<
    Data: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
    const BUFFER_SIZE: usize,
> {
    /// The serial peripheral device
    serial_device: Serial,
    /// The internal buffer for sending and receiving data
    buffer: [u8; BUFFER_SIZE],
    /// The most recent data received from the subscription
    data: Option<Data>,
}

impl<Data, Serial, Err, const BUFFER_SIZE: usize>
    SerialPublisherSubscriber<Data, Serial, Err, BUFFER_SIZE>
where
    Data: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
{
    /// Create a new SerialPublisherSubscriber from the peripheral
    pub fn new(serial_device: Serial, buffer: [u8; BUFFER_SIZE]) -> Self {
        assert!(
            BUFFER_SIZE >= Data::len(),
            "The buffer must be large enough to fit encoded data"
        );
        Self {
            serial_device,
            buffer,
            data: None,
        }
    }

    /// Destroy the SerialPublisherSubscriber returning the serial
    /// peripheral
    pub fn destroy(self) -> Serial {
        self.serial_device
    }
}

impl<Data, Serial, Err, const BUFFER_SIZE: usize> Publisher
    for SerialPublisherSubscriber<Data, Serial, Err, BUFFER_SIZE>
where
    Data: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
{
    type Data = Data;
    type Error = SerialPublishError<Err>;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        self.buffer.iter_mut().for_each(|v| *v = 0);
        data.pack(&mut self.buffer)
            .map_err(SerialPublishError::PackingError)?;

        self.serial_device
            .write_all(&self.buffer)
            .map_err(SerialPublishError::IOError)?;

        Ok(())
    }
}

impl<Data, Serial, Err, const BUFFER_SIZE: usize> Subscriber
    for SerialPublisherSubscriber<Data, Serial, Err, BUFFER_SIZE>
where
    Data: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
{
    type Target = Option<Data>;

    fn get(&mut self) -> &Self::Target {
        let mut new_data = None;

        while let Ok(ready) = self.serial_device.read_ready() {
            if !ready {
                break;
            }

            self.buffer.iter_mut().for_each(|v| *v = 0);
            if self.serial_device.read(&mut self.buffer).is_ok() {
                if let Ok(data) = Data::unpack(&self.buffer) {
                    new_data = Some(data);
                }
            }
        }

        if let Some(data) = new_data {
            self.data = Some(data);
        }

        &self.data
    }
}
