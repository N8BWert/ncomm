//!
//! Serial Client and Server for communication using embedded-io
//! traits.
//!

use core::marker::PhantomData;

use embedded_io::{Error, Read, ReadReady, Write};

use ncomm_core::client_server::{Client, Server};
use ncomm_utils::packing::{Packable, PackingError};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

/// An Error regarding sending and receiving data via serial
#[derive(Debug)]
pub enum SerialClientServerError<Err: Error> {
    /// Embedded-IO Error
    IOError(Err),
    /// An error occurred with pacing the data
    PackingError(PackingError),
}

/// Client that sends requests and receives responses via a serial device.
///
/// Note: To make this client no_std compatible the client has an internal buffer
/// that is statically allocated, hence the reason for the const BUFFER_SIZE: usize
/// generic
pub struct SerialClient<
    Req: Packable,
    Res: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
    const BUFFER_SIZE: usize,
> {
    /// The serial peripheral device
    serial_device: Serial,
    /// The internal buffer for encoding data
    buffer: [u8; BUFFER_SIZE],
    /// A marker t bind the type of data
    _phantom: PhantomData<(Req, Res)>,
}

impl<Req, Res, Serial, Err, const BUFFER_SIZE: usize>
    SerialClient<Req, Res, Serial, Err, BUFFER_SIZE>
where
    Req: Packable,
    Res: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
{
    /// Construct a new SerialClient from a serial peripheral
    pub fn new(serial_device: Serial, buffer: [u8; BUFFER_SIZE]) -> Self {
        assert!(
            BUFFER_SIZE >= Req::len() + Res::len(),
            "The buffer must be large enough to fit a request and response"
        );
        Self {
            serial_device,
            buffer,
            _phantom: PhantomData,
        }
    }

    /// Destroy the SerialClient, returning the serial peripheral
    pub fn destroy(self) -> Serial {
        self.serial_device
    }
}

impl<Req, Res, Serial, Err, const BUFFER_SIZE: usize> Client
    for SerialClient<Req, Res, Serial, Err, BUFFER_SIZE>
where
    Req: Packable,
    Res: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
{
    type Request = Req;
    type Response = Res;
    type Error = SerialClientServerError<Err>;

    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error> {
        self.buffer.iter_mut().for_each(|v| *v = 0);
        request
            .pack(&mut self.buffer)
            .map_err(SerialClientServerError::PackingError)?;

        self.serial_device
            .write_all(&self.buffer[..Req::len()])
            .map_err(SerialClientServerError::IOError)?;

        Ok(())
    }

    fn poll_for_response(
        &mut self,
    ) -> Result<Option<(Self::Request, Self::Response)>, Self::Error> {
        if let Ok(ready) = self.serial_device.read_ready() {
            if !ready {
                return Ok(None);
            }

            self.buffer.iter_mut().for_each(|v| *v = 0);
            if self.serial_device.read(&mut self.buffer).is_ok() {
                if let (Ok(request), Ok(response)) = (
                    Req::unpack(&self.buffer[..Req::len()]),
                    Res::unpack(&self.buffer[Req::len()..]),
                ) {
                    return Ok(Some((request, response)));
                }
            }
        }
        Ok(None)
    }

    #[cfg(any(feature = "alloc", feature = "std"))]
    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>> {
        let mut responses = Vec::new();

        while let Ok(ready) = self.serial_device.read_ready() {
            if !ready {
                break;
            }

            self.buffer.iter_mut().for_each(|v| *v = 0);
            if self.serial_device.read(&mut self.buffer).is_ok() {
                if let (Ok(request), Ok(response)) = (
                    Req::unpack(&self.buffer[..Req::len()]),
                    Res::unpack(&self.buffer[Req::len()..]),
                ) {
                    responses.push(Ok((request, response)));
                }
            }
        }

        responses
    }
}

/// A serial server capable of receiving requests and sending responses.
///
/// Note: To make this server no_std compatible the server has an internal buffer
/// that is statically allocated, hence the reason for the const BUFFER_SIZE: usize
/// generic
pub struct SerialServer<
    Req: Packable,
    Res: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
    const BUFFER_SIZE: usize,
> {
    /// The serial peripheral device
    serial_device: Serial,
    /// The internal buffer for sending and receiving data
    buffer: [u8; BUFFER_SIZE],
    /// A holder for the request and response data type
    _phantom: PhantomData<(Req, Res)>,
}

impl<Req, Res, Serial, Err, const BUFFER_SIZE: usize>
    SerialServer<Req, Res, Serial, Err, BUFFER_SIZE>
where
    Req: Packable,
    Res: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
{
    /// Create a new SerialServer from a serial device peripheral
    pub fn new(serial_device: Serial, buffer: [u8; BUFFER_SIZE]) -> Self {
        assert!(
            buffer.len() >= Req::len() + Res::len(),
            "The buffer must be large enough to accommodate a request and response"
        );
        Self {
            serial_device,
            buffer,
            _phantom: PhantomData,
        }
    }

    /// Destroy the SerialServer returning the serial peripheral
    pub fn destroy(self) -> Serial {
        self.serial_device
    }
}

impl<Req, Res, Serial, Err, const BUFFER_SIZE: usize> Server
    for SerialServer<Req, Res, Serial, Err, BUFFER_SIZE>
where
    Req: Packable,
    Res: Packable,
    Serial: ReadReady<Error = Err> + Read<Error = Err> + Write<Error = Err>,
    Err: Error,
{
    type Request = Req;
    type Response = Res;
    type Key = bool;
    type Error = SerialClientServerError<Err>;

    fn poll_for_request(&mut self) -> Result<Option<(Self::Key, Self::Request)>, Self::Error> {
        if let Ok(ready) = self.serial_device.read_ready() {
            if !ready {
                return Ok(None);
            }

            self.buffer.iter_mut().for_each(|v| *v = 0);
            if self.serial_device.read(&mut self.buffer).is_ok() {
                if let Ok(request) = Req::unpack(&self.buffer[..Req::len()]) {
                    return Ok(Some((true, request)));
                }
            }
        }
        Ok(None)
    }

    #[cfg(any(feature = "alloc", feature = "std"))]
    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>> {
        let mut requests = Vec::new();

        while let Ok(ready) = self.serial_device.read_ready() {
            if !ready {
                break;
            }

            self.buffer.iter_mut().for_each(|v| *v = 0);
            if self.serial_device.read(&mut self.buffer).is_ok() {
                if let Ok(request) = Req::unpack(&self.buffer[..Req::len()]) {
                    requests.push(Ok((true, request)));
                }
            }
        }

        requests
    }

    fn send_response(
        &mut self,
        _client_key: Self::Key,
        request: Self::Request,
        response: Self::Response,
    ) -> Result<(), Self::Error> {
        self.buffer.iter_mut().for_each(|v| *v = 0);
        request
            .pack(&mut self.buffer[..Req::len()])
            .map_err(SerialClientServerError::PackingError)?;
        response
            .pack(&mut self.buffer[Req::len()..])
            .map_err(SerialClientServerError::PackingError)?;

        self.serial_device
            .write_all(&self.buffer[..(Req::len() + Res::len())])
            .map_err(SerialClientServerError::IOError)?;

        Ok(())
    }
}
