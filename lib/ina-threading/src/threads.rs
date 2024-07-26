// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 Jaxydog
//
// This file is part of 1N4.
//
// 1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// 1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with 1N4. If not, see
// <https://www.gnu.org/licenses/>.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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

    /// Invokes the thread, returning the reponse of the method when available.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    pub fn blocking_invoke(&mut self, input: S) -> Result<R, (usize, S)> {
        const DURATION: Duration = Duration::from_millis(5);

        let sequence = *self.sequence.blocking_read();

        *self.sequence.blocking_write() = sequence.wrapping_add(1);

        self.as_sender().blocking_send((sequence, input))?;

        let mut interval = Instant::now();

        loop {
            let now = Instant::now();

            if now < interval {
                let difference = interval - now;

                std::thread::sleep(difference);
            }

            interval = now + DURATION;

            let mut produced = self.produced.blocking_write();

            if let Some(result) = produced.remove(&sequence) {
                return Ok(result);
            }

            drop(produced);

            match self.as_receiver_mut().try_recv() {
                Ok((seq, result)) if seq == sequence => return Ok(result),
                Ok((seq, result)) => self.produced.blocking_write().insert(seq, result),
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

/// A thread that consumes and produces values like a function.
#[derive(Debug)]
pub struct StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    /// The inner invoker thread.
    inner: Invoker<(Arc<RwLock<T>>, S), R>,
    /// The constant state.
    state: Arc<RwLock<T>>,
}

impl<T, S, R> StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
    S: Send + 'static,
    R: Send + Sync + 'static,
{
    /// Spawns a new thread with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    #[allow(clippy::type_complexity)]
    pub fn spawn<N, F>(name: N, state: T, call: F, size: usize) -> Result<Self, (usize, (Arc<RwLock<T>>, S))>
    where
        N: AsRef<str>,
        F: Fn(Arc<RwLock<T>>, S) -> R + Send + 'static,
    {
        let state = Arc::new(RwLock::new(state));

        Ok(Self { inner: Invoker::spawn(name, move |(s, i)| call(s, i), size)?, state })
    }

    /// Invokes the thread, returning the reponse of the method when available.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    #[allow(clippy::type_complexity)]
    #[inline]
    pub async fn invoke(&mut self, input: S) -> Result<R, (usize, (Arc<RwLock<T>>, S))> {
        self.inner.invoke((Arc::clone(&self.state), input)).await
    }

    /// Invokes the thread, returning the reponse of the method when available.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    #[allow(clippy::type_complexity)]
    #[inline]
    pub fn blocking_invoke(&mut self, input: S) -> Result<R, (usize, (Arc<RwLock<T>>, S))> {
        self.inner.blocking_invoke((Arc::clone(&self.state), input))
    }
}

impl<T, S, R> HandleHolder<()> for StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
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

impl<T, S, R> ConsumingThread<(usize, (Arc<RwLock<T>>, S))> for StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    #[inline]
    fn as_sender(&self) -> &Sender<(usize, (Arc<RwLock<T>>, S))> {
        self.inner.as_sender()
    }

    #[inline]
    fn as_sender_mut(&mut self) -> &mut Sender<(usize, (Arc<RwLock<T>>, S))> {
        self.inner.as_sender_mut()
    }

    #[inline]
    fn into_sender(self) -> Sender<(usize, (Arc<RwLock<T>>, S))> {
        self.inner.into_sender()
    }

    #[inline]
    fn clone_sender(&self) -> Sender<(usize, (Arc<RwLock<T>>, S))> {
        self.inner.clone_sender()
    }
}

impl<T, S, R> ProducingThread<(usize, R)> for StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
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
