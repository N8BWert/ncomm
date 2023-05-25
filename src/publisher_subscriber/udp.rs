use std::net::UdpSocket;
use std::marker::PhantomData;

use crate::publisher_subscriber::{Publish, SubscribeRemote, Receive};

pub struct Publisher<'a, Data: Send + Clone, const DATA_SIZE: usize> {
    tx: UdpSocket,
    addresses: Vec<&'a str>,
    phantom: PhantomData<Data>,
}

pub struct Subscriber<Data: Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Option<Data>,
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Publisher<'a, Data, DATA_SIZE> {
    pub fn new(bind_address: &'a str, addresses: Vec<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        
        Self { tx: socket, addresses: addresses, phantom: PhantomData }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Subscriber<Data, DATA_SIZE> {
    pub fn new(bind_address: &'a str, from_address: &'a str) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        socket.connect(from_address).expect("couldn't connect to the given address");

        Self { rx: socket, data: None }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> Publish<Data> for Publisher<'a, Data, DATA_SIZE> {
    fn send(&self, data: Data) {
        let buf: [u8; DATA_SIZE] = unsafe { std::mem::transmute_copy(&data) };
        for address in self.addresses.iter() {
            if let Err(err) = self.tx.send_to(&buf, address) {
                println!("Received Error on Send: {}", err)
            }
        }
    }
}

impl<'a, Data: Send + Clone, const DATA_SIZE: usize> SubscribeRemote<'a> for Publisher<'a, Data, DATA_SIZE> {
    fn add_subscriber(&mut self, address: &'a str) {
        self.addresses.push(address);
    }
}

impl<Data: Send + Clone, const DATA_SIZE: usize> Receive for Subscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        let mut data: Option<Data> = None;
        loop {
            let mut buf = [0u8; DATA_SIZE];
            match self.rx.recv(&mut buf) {
                Ok(_received) => unsafe { data = Some(std::mem::transmute_copy(&buf)); },
                Err(_) => break,
            }
        }
        self.data = data;
    }
}