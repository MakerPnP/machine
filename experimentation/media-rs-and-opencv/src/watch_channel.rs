use std::sync::{Arc, Mutex, Condvar};
use std::time::Duration;

pub struct WatchChannel<T> {
    data: Arc<(Mutex<Option<T>>, Condvar)>,
}

impl<T> WatchChannel<T> {
    pub fn new() -> Self {
        Self {
            data: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    pub fn sender(&self) -> WatchSender<T> {
        WatchSender {
            data: self.data.clone(),
        }
    }

    pub fn receiver(&self) -> WatchReceiver<T> {
        WatchReceiver {
            data: self.data.clone(),
        }
    }

    pub fn split(self) -> (WatchSender<T>, WatchReceiver<T>) {
        (self.sender(), self.receiver())
    }
}

pub struct WatchSender<T> {
    data: Arc<(Mutex<Option<T>>, Condvar)>,
}

impl<T> WatchSender<T> {
    pub fn send(&self, value: T) {
        let (lock, cvar) = &*self.data;
        let mut data = lock.lock().unwrap();
        *data = Some(value);
        cvar.notify_one();
    }
}

pub struct WatchReceiver<T> {
    data: Arc<(Mutex<Option<T>>, Condvar)>,
}

impl<T> WatchReceiver<T> {
    pub fn try_recv(&self) -> Option<T> {
        let (lock, _) = &*self.data;
        let mut data = lock.lock().unwrap();
        data.take()
    }

    pub fn recv(&self) -> T {
        let (lock, cvar) = &*self.data;
        let mut data = lock.lock().unwrap();

        while data.is_none() {
            data = cvar.wait(data).unwrap();
        }

        data.take().unwrap()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Option<T> {
        let (lock, cvar) = &*self.data;
        let mut data = lock.lock().unwrap();

        if data.is_none() {
            let (new_data, _) = cvar.wait_timeout(data, timeout).unwrap();
            data = new_data;
        }

        data.take()
    }
}