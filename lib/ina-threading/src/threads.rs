use std::collections::BTreeMap;
use std::thread::JoinHandle;
use std::time::Duration;

use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;

use crate::{ConsumingThread, Error, HandleHolder, ProducingThread, Result, Thread};

/// A thread that consumes values.
#[derive(Debug)]
pub struct Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    /// The sending channel.
    sender: Sender<S>,
    /// The inner thread.
    thread: Thread<T>,
}

impl<S, T> Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new thread with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, call: F, size: usize) -> Result<Self, S>
    where
        N: AsRef<str>,
        F: FnOnce(Receiver<S>) -> T + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(size);
        let thread = Thread::spawn(name, || call(receiver))?;

        Ok(Self { sender, thread })
    }
}

impl<S, T> HandleHolder<T> for Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_handle(&self) -> &JoinHandle<T> {
        self.thread.as_handle()
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut JoinHandle<T> {
        self.thread.as_handle_mut()
    }

    #[inline]
    fn into_handle(self) -> JoinHandle<T> {
        self.thread.into_handle()
    }
}

impl<S, T> ConsumingThread<S> for Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_sender(&self) -> &Sender<S> {
        &self.sender
    }

    #[inline]
    fn as_sender_mut(&mut self) -> &mut Sender<S> {
        &mut self.sender
    }

    #[inline]
    fn into_sender(self) -> Sender<S> {
        self.sender
    }

    #[inline]
    fn clone_sender(&self) -> Sender<S> {
        self.sender.clone()
    }
}

/// A thread that consumes values.
#[derive(Debug)]
pub struct Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    /// The receiving channel.
    receiver: Receiver<R>,
    /// The inner thread.
    thread: Thread<T>,
}

impl<R, T> Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new thread with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, call: F, size: usize) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>) -> T + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(size);
        let thread = Thread::spawn(name, || call(sender))?;

        Ok(Self { receiver, thread })
    }
}

impl<R, T> HandleHolder<T> for Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_handle(&self) -> &JoinHandle<T> {
        self.thread.as_handle()
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut JoinHandle<T> {
        self.thread.as_handle_mut()
    }

    #[inline]
    fn into_handle(self) -> JoinHandle<T> {
        self.thread.into_handle()
    }
}

impl<R, T> ProducingThread<R> for Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_receiver(&self) -> &Receiver<R> {
        &self.receiver
    }

    #[inline]
    fn as_receiver_mut(&mut self) -> &mut Receiver<R> {
        &mut self.receiver
    }

    #[inline]
    fn into_receiver(self) -> Receiver<R> {
        self.receiver
    }
}

/// A thread that consumes and produces values.
#[derive(Debug)]
pub struct Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    /// The sending channel.
    sender: Sender<S>,
    /// The receiving channel.
    receiver: Receiver<R>,
    /// The inner thread.
    thread: Thread<T>,
}

impl<S, R, T> Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new thread with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, call: F, size: usize) -> Result<Self, S>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>, Receiver<S>) -> T + Send + 'static,
    {
        let (local_sender, thread_receiver) = tokio::sync::mpsc::channel(size);
        let (thread_sender, local_receiver) = tokio::sync::mpsc::channel(size);
        let thread = Thread::spawn(name, || call(thread_sender, thread_receiver))?;

        Ok(Self { sender: local_sender, receiver: local_receiver, thread })
    }
}

impl<S, R, T> HandleHolder<T> for Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_handle(&self) -> &JoinHandle<T> {
        self.thread.as_handle()
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut JoinHandle<T> {
        self.thread.as_handle_mut()
    }

    #[inline]
    fn into_handle(self) -> JoinHandle<T> {
        self.thread.into_handle()
    }
}

impl<S, R, T> ConsumingThread<S> for Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_sender(&self) -> &Sender<S> {
        &self.sender
    }

    #[inline]
    fn as_sender_mut(&mut self) -> &mut Sender<S> {
        &mut self.sender
    }

    #[inline]
    fn into_sender(self) -> Sender<S> {
        self.sender
    }

    #[inline]
    fn clone_sender(&self) -> Sender<S> {
        self.sender.clone()
    }
}

impl<S, R, T> ProducingThread<R> for Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    #[inline]
    fn as_receiver(&self) -> &Receiver<R> {
        &self.receiver
    }

    #[inline]
    fn as_receiver_mut(&mut self) -> &mut Receiver<R> {
        &mut self.receiver
    }

    #[inline]
    fn into_receiver(self) -> Receiver<R> {
        self.receiver
    }
}

/// A thread that consumes and produces values like a function.
#[derive(Debug)]
pub struct Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    /// The inner exchanger thread.
    inner: Exchanger<(usize, S), (usize, R), ()>,
    /// A map of missed results.
    produced: RwLock<BTreeMap<usize, R>>,
    /// A sequence counter for each input.
    sequence: RwLock<usize>,
}

impl<S, R> Invoker<S, R>
where
    S: Send + 'static,
    R: Send + Sync + 'static,
{
    /// Spawns a new thread with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, call: F, size: usize) -> Result<Self, (usize, S)>
    where
        N: AsRef<str>,
        F: Fn(S) -> R + Send + 'static,
    {
        let call = move |sender: Sender<(usize, R)>, mut receiver: Receiver<(usize, S)>| {
            loop {
                let Some((sequence, input)) = receiver.blocking_recv() else {
                    break;
                };
                if sender.blocking_send((sequence, call(input))).is_err() {
                    break;
                }
            }
        };

        Ok(Self {
            inner: Exchanger::spawn(name, call, size)?,
            produced: RwLock::default(),
            sequence: RwLock::default(),
        })
    }

    /// Invokes the thread, returning the reponse of the method when available.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    pub async fn invoke(&mut self, input: S) -> Result<R, (usize, S)> {
        let sequence = *self.sequence.read().await;

        *self.sequence.write().await = sequence.wrapping_add(1);

        self.as_sender().send((sequence, input)).await?;

        let mut interval = tokio::time::interval(Duration::from_millis(5));

        loop {
            interval.tick().await;

            let mut produced = self.produced.write().await;

            if let Some(result) = produced.remove(&sequence) {
                return Ok(result);
            }

            drop(produced);

            match self.as_receiver_mut().try_recv() {
                Ok((seq, result)) if seq == sequence => return Ok(result),
                Ok((seq, result)) => self.produced.write().await.insert(seq, result),
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => return Err(Error::Disconnected),
            };
        }
    }
}

impl<S, R> HandleHolder<()> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    #[inline]
    fn as_handle(&self) -> &JoinHandle<()> {
        self.inner.as_handle()
    }

    #[inline]
    fn as_handle_mut(&mut self) -> &mut JoinHandle<()> {
        self.inner.as_handle_mut()
    }

    #[inline]
    fn into_handle(self) -> JoinHandle<()> {
        self.inner.into_handle()
    }
}

impl<S, R> ConsumingThread<(usize, S)> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    #[inline]
    fn as_sender(&self) -> &Sender<(usize, S)> {
        self.inner.as_sender()
    }

    #[inline]
    fn as_sender_mut(&mut self) -> &mut Sender<(usize, S)> {
        self.inner.as_sender_mut()
    }

    #[inline]
    fn into_sender(self) -> Sender<(usize, S)> {
        self.inner.into_sender()
    }

    #[inline]
    fn clone_sender(&self) -> Sender<(usize, S)> {
        self.inner.clone_sender()
    }
}

impl<S, R> ProducingThread<(usize, R)> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    #[inline]
    fn as_receiver(&self) -> &Receiver<(usize, R)> {
        self.inner.as_receiver()
    }

    #[inline]
    fn as_receiver_mut(&mut self) -> &mut Receiver<(usize, R)> {
        self.inner.as_receiver_mut()
    }

    #[inline]
    fn into_receiver(self) -> Receiver<(usize, R)> {
        self.inner.into_receiver()
    }
}
