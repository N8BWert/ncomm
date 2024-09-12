//!
//! Local Publishers and Subscribers
//!
//! Local Publishers and Subscribers utilize some combination of
//! primitives from the standard library to enable the sharing of
//! data between publishers and subscribers
//!

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crossbeam::channel::{self, Receiver, SendError, Sender};

use ncomm_core::{Publisher, Subscriber};

/// Local Subscriber that utilizes a crossbeam multi subscriber channel
/// to receive data from a local publisher
pub struct LocalSubscriber<Data> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Arc<Option<Data>>>,
    /// The current data stored in the local subscriber
    data: Arc<Option<Data>>,
}

impl<Data> Subscriber for LocalSubscriber<Data> {
    type Target = Option<Data>;

    fn get(&mut self) -> &Self::Target {
        if let Some(data) = self.rx.try_iter().last() {
            self.data = data;
        }

        self.data.as_ref()
    }
}

/// Local Subscriber that stores incoming data into a buffer for processing all at once
pub struct LocalBufferedSubscriber<Data> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Arc<Option<Data>>>,
    /// The buffer of data stored in the subscriber
    buffer: Vec<Arc<Option<Data>>>,
}

impl<Data> LocalBufferedSubscriber<Data> {
    /// Clear the data buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl<Data> Subscriber for LocalBufferedSubscriber<Data> {
    type Target = Vec<Arc<Option<Data>>>;

    fn get(&mut self) -> &Self::Target {
        for data in self.rx.try_iter() {
            self.buffer.push(data)
        }

        &self.buffer
    }
}

/// Local subscriber where data has a specific time-to-live and will decay
/// after the lifetime has passed
pub struct LocalTTLSubscriber<Data> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Arc<Option<Data>>>,
    /// The current data stored in the local subscriber
    data: Option<(Arc<Option<Data>>, Instant)>,
    /// The time-to-live of a piece of data
    ttl: Duration,
}

impl<Data> Subscriber for LocalTTLSubscriber<Data> {
    type Target = Option<(Arc<Option<Data>>, Instant)>;

    fn get(&mut self) -> &Self::Target {
        if let Some(data) = self.rx.try_iter().last() {
            self.data = Some((data, Instant::now()));
        }

        if self.data.as_ref().is_some()
            && Instant::now().duration_since(self.data.as_ref().unwrap().1) > self.ttl
        {
            self.data = None;
        }

        &self.data
    }
}

/// Local subscriber that maps incoming data to into a location in a hashmap
/// allowing the subscriber to maintain a number of pieces of data at once.
pub struct LocalMappedSubscriber<Data, K: Eq + Hash, F: Fn(&Option<Data>) -> K> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Arc<Option<Data>>>,
    /// The current data stored in the local hashmap
    data: HashMap<K, Arc<Option<Data>>>,
    /// The hash function used to map incoming data into the hashmap
    hash: F,
}

impl<Data, K: Eq + Hash, F: Fn(&Option<Data>) -> K> Subscriber
    for LocalMappedSubscriber<Data, K, F>
{
    type Target = HashMap<K, Arc<Option<Data>>>;

    fn get(&mut self) -> &Self::Target {
        for data in self.rx.try_iter() {
            let label = (self.hash)(&data);
            self.data.insert(label, data);
        }

        &self.data
    }
}

/// Local subscriber that maps incoming data to into a location in a hashmap
/// while specifying a time-to-live for pieces of data contained in the
/// hashmap
pub struct LocalMappedTTLSubscriber<Data, K: Eq + Hash, F: Fn(&Option<Data>) -> K> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Arc<Option<Data>>>,
    /// The current data stored in a hashmap
    data: HashMap<K, (Arc<Option<Data>>, Instant)>,
    /// The hash function used to map incoming data into the hashmap
    hash: F,
    /// The time-to-live of pieces of data in the hashmap
    ttl: Duration,
}

impl<Data, K: Eq + Hash, F: Fn(&Option<Data>) -> K> Subscriber
    for LocalMappedTTLSubscriber<Data, K, F>
{
    type Target = HashMap<K, (Arc<Option<Data>>, Instant)>;

    fn get(&mut self) -> &Self::Target {
        for data in self.rx.try_iter() {
            let label = (self.hash)(&data);
            self.data.insert(label, (data, Instant::now()));
        }

        let now = Instant::now();
        self.data.retain(|_, v| now.duration_since(v.1) <= self.ttl);

        &self.data
    }
}

/// Local Publisher that utilizes a crossbeam multi publisher multi
/// subscriber to send data
pub struct LocalPublisher<Data> {
    /// The transmit pipe that is used to send data to the subscriber
    #[allow(clippy::type_complexity)]
    txs: Arc<Mutex<Vec<Sender<Arc<Option<Data>>>>>>,
    /// The most recent data sent over the tx pipes so new subscribers will
    /// automatically have the most recent data
    #[allow(clippy::type_complexity)]
    data: Arc<Mutex<Option<(Arc<Option<Data>>, Instant)>>>,
}

impl<Data> Default for LocalPublisher<Data> {
    fn default() -> Self {
        Self {
            txs: Arc::new(Mutex::new(Vec::new())),
            data: Arc::new(Mutex::new(None)),
        }
    }
}

impl<Data> LocalPublisher<Data> {
    /// Create a new local publisher
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a local subscriber
    pub fn subscribe(&mut self) -> LocalSubscriber<Data> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let data = self
            .data
            .lock()
            .unwrap()
            .as_ref()
            .map(|data| data.0.clone());

        if let Some(data) = data {
            LocalSubscriber { rx, data }
        } else {
            LocalSubscriber {
                rx,
                data: Arc::new(None),
            }
        }
    }

    /// Create a local buffered subscriber
    pub fn subscribe_buffered(&mut self) -> LocalBufferedSubscriber<Data> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let mut buffer = Vec::new();
        if let Some(data) = self.data.lock().unwrap().as_ref() {
            buffer.push(data.0.clone());
        }

        LocalBufferedSubscriber { rx, buffer }
    }

    /// Create a local subscriber with a specific time-to-live of pieces of data
    pub fn subscribe_ttl(&mut self, timeout: Duration) -> LocalTTLSubscriber<Data> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let data = match self.data.lock().unwrap().as_ref() {
            Some(data) => {
                if Instant::now().duration_since(data.1) > timeout {
                    Some(data.clone())
                } else {
                    None
                }
            }
            None => None,
        };

        LocalTTLSubscriber {
            rx,
            data,
            ttl: timeout,
        }
    }

    /// Create a local subscriber that uses a map function to map data to specific slots in a hashmap.
    ///
    /// Note: This subscriber will only have access to them most recent piece of data so
    /// do not expect that data sent a long time ago will be present in this subscriber's data
    pub fn subscribe_mapped<K: Eq + Hash, F: Fn(&Option<Data>) -> K>(
        &mut self,
        map: F,
    ) -> LocalMappedSubscriber<Data, K, F> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let mut hashmap = HashMap::new();
        if let Some(data) = self.data.lock().unwrap().as_ref() {
            let data = data.0.clone();
            let label = (map)(&data);
            hashmap.insert(label, data);
        }

        LocalMappedSubscriber {
            rx,
            data: hashmap,
            hash: map,
        }
    }

    /// Create a local subscriber that uses a map function to map data to specific slots
    /// in a hashmap
    ///
    /// Note: This subscriber will only have access to the most recent piece of data so
    /// do not expect that data sent a long time ago will be present in this subscriber's data
    pub fn subscribe_mapped_ttl<K: Eq + Hash, F: Fn(&Option<Data>) -> K>(
        &mut self,
        map: F,
        ttl: Duration,
    ) -> LocalMappedTTLSubscriber<Data, K, F> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let mut hashmap = HashMap::new();
        if let Some(data) = self.data.lock().unwrap().as_ref() {
            if Instant::now().duration_since(data.1) <= ttl {
                let data = data.clone();
                let label = (map)(&data.0);
                hashmap.insert(label, data);
            }
        }

        LocalMappedTTLSubscriber {
            rx,
            data: hashmap,
            hash: map,
            ttl,
        }
    }
}

impl<Data> Clone for LocalPublisher<Data> {
    fn clone(&self) -> Self {
        Self {
            txs: self.txs.clone(),
            data: self.data.clone(),
        }
    }
}

impl<Data> Publisher for LocalPublisher<Data> {
    type Data = Data;
    type Error = SendError<Arc<Option<Data>>>;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        let data = Arc::new(Some(data));
        let txs = self.txs.lock().unwrap();
        for tx in txs.iter() {
            tx.send(data.clone())?;
        }
        let mut data_ref = self.data.lock().unwrap();
        *data_ref = Some((data, Instant::now()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::random;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestData {
        num: u64,
    }

    impl TestData {
        pub fn new() -> Self {
            Self { num: random() }
        }
    }

    #[test]
    fn test_publish_local_subscriber() {
        let mut publisher = LocalPublisher::new();
        let mut subscriber = publisher.subscribe();

        let data = TestData::new();
        publisher.publish(data.clone()).unwrap();
        assert_eq!(subscriber.get().unwrap(), data);
    }

    #[test]
    fn test_publish_buffered_subscriber() {
        let mut publisher = LocalPublisher::new();
        let mut subscriber = publisher.subscribe_buffered();

        let datas = vec![TestData::new(); 100];

        for data in datas.iter() {
            publisher.publish(data.clone()).unwrap();
        }

        assert_eq!(
            subscriber
                .get()
                .iter()
                .map(|v| v.unwrap().clone())
                .collect::<Vec<TestData>>(),
            datas
        );
    }

    #[test]
    fn test_publish_ttl_subscriber() {
        let mut publisher = LocalPublisher::new();
        let mut short_subscriber = publisher.subscribe_ttl(Duration::from_nanos(1));
        let mut long_subscriber = publisher.subscribe_ttl(Duration::from_secs(100));

        let data = TestData::new();
        publisher.publish(data.clone()).unwrap();

        std::thread::sleep(Duration::from_millis(100));

        assert_eq!(*short_subscriber.get(), None);
        assert_eq!(
            long_subscriber.get().as_ref().unwrap().0.clone().unwrap(),
            data
        );
    }

    #[test]
    fn test_publish_mapped_subscriber() {
        let mut publisher = LocalPublisher::new();
        let mut subscriber =
            publisher.subscribe_mapped(|data: &Option<TestData>| data.unwrap().num);

        let data = TestData::new();
        publisher.publish(data.clone()).unwrap();

        assert_eq!(
            subscriber
                .get()
                .get(&data.num)
                .unwrap()
                .as_ref()
                .unwrap()
                .clone(),
            data
        );
    }

    #[test]
    fn test_publish_mapped_ttl_subscriber() {
        let mut publisher = LocalPublisher::new();
        let mut short_subscriber = publisher.subscribe_mapped_ttl(
            |data: &Option<TestData>| data.unwrap().num,
            Duration::from_nanos(1),
        );
        let mut long_subscriber = publisher.subscribe_mapped_ttl(
            |data: &Option<TestData>| data.unwrap().num,
            Duration::from_secs(5),
        );

        let data = TestData::new();
        publisher.publish(data.clone()).unwrap();

        assert_eq!(short_subscriber.get().get(&data.num), None);
        assert_eq!(
            long_subscriber
                .get()
                .get(&data.num)
                .unwrap()
                .0
                .as_ref()
                .unwrap()
                .clone(),
            data
        );
    }
}
