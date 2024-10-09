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
use std::future::Future;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tokio::runtime::Builder;
use tokio::sync::RwLock;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{Receiver, Sender};

use super::exchanger::Exchanger;
use crate::{ConsumingThread, Error, HandleHolder, ProducingThread, Result};

/// A value with a nonce for tracking within an invoker.
pub type Nonce<T> = (Option<usize>, T);
/// A stateful invoker's state argument.
pub type State<T> = Arc<RwLock<T>>;
/// A result returned from a stateful invoker.
pub type StatefulResult<I, T, S> = Result<I, Nonce<(State<T>, S)>>;

/// A thread that consumes and produces values like a function.
#[derive(Debug)]
pub struct Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    /// The inner exchanger thread.
    inner: Exchanger<Nonce<S>, Nonce<R>, ()>,
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
    pub fn spawn<N, F>(name: N, call: F, size: usize) -> Result<Self, Nonce<S>>
    where
        N: AsRef<str>,
        F: Fn(S) -> R + Send + 'static,
    {
        let call = move |sender: Sender<Nonce<R>>, mut receiver: Receiver<Nonce<S>>| loop {
            let Some((sequence, input)) = receiver.blocking_recv() else {
                break;
            };

            let response = call(input);

            if sequence.is_some() && sender.blocking_send((sequence, response)).is_err() {
                break;
            }
        };

        Ok(Self {
            inner: Exchanger::spawn(name, call, size)?,
            produced: RwLock::default(),
            sequence: RwLock::default(),
        })
    }

    /// Spawns a new thread with the given name and asynchronous task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    #[expect(clippy::missing_panics_doc, reason = "false-positive; the thread will just exit immediately, not panic")]
    pub fn spawn_with_runtime<N, F, O>(name: N, call: F, size: usize) -> Result<Self, Nonce<S>>
    where
        N: AsRef<str>,
        F: Fn(S) -> O + Send + 'static,
        O: Future<Output = R> + Send,
    {
        let call = move |sender: Sender<Nonce<R>>, mut receiver: Receiver<Nonce<S>>| {
            #[expect(clippy::expect_used, reason = "if the runtime fails to spawn, we can't execute any code")]
            let runtime = Builder::new_current_thread().enable_all().build().expect("failed to spawn runtime");

            runtime.block_on(async move {
                loop {
                    let Some((sequence, input)) = receiver.recv().await else { break };
                    let response = call(input).await;

                    if sequence.is_some() && sender.send((sequence, response)).await.is_err() {
                        break;
                    }
                }
            });
        };

        Ok(Self {
            inner: Exchanger::spawn(name, call, size)?,
            produced: RwLock::default(),
            sequence: RwLock::default(),
        })
    }

    /// Invokes the thread, returning the response of the method when available.
    ///
    /// # Panics
    ///
    /// Panics if enough tasks ([`usize::MAX`] to be exact) are triggered to cause a task to receive the same sequence
    /// ID as another pending task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    pub async fn invoke(&mut self, input: S) -> Result<R, Nonce<S>> {
        // Claim the current sequence ID and increment `self.sequence` to a fresh sequence ID.
        let sequence = *self.sequence.read().await;
        *self.sequence.write().await = sequence.wrapping_add(1);

        // Instruct the thread to evaluate `input`.
        self.as_sender().send((Some(sequence), input)).await?;

        loop {
            let mut produced = self.produced.write().await;

            // If another task received the result first, it will drop it into `self.produced`.
            if let Some(result) = produced.remove(&sequence) {
                return Ok(result);
            }

            drop(produced);

            match self.as_receiver_mut().try_recv() {
                // If the value was returned by the task triggered above, return it.
                Ok((Some(seq), result)) if seq == sequence => return Ok(result),
                // If the value was returned by another task, drop it into `self.produced` so that
                // it can still be consumed.
                Ok((Some(seq), result)) => {
                    // This would require that enough tasks ([`usize::MAX`] to be exact) are
                    // triggered to cause a task to receive the same sequence ID as another pending
                    // task.
                    assert!(self.produced.write().await.insert(seq, result).is_none());
                }
                Ok((None, _)) => unreachable!("requests with no sequence number should not be returned"),
                // If task hasn't returned yet.
                Err(TryRecvError::Empty) => tokio::task::yield_now().await,
                Err(TryRecvError::Disconnected) => return Err(Error::Disconnected),
            };
        }
    }

    /// Invokes the thread, returning the response of the method when available.
    ///
    /// This blocks the current thread.
    ///
    /// # Panics
    ///
    /// Panics if this is called in an asynchronous context.
    ///
    /// Panics if enough tasks ([`usize::MAX`] to be exact) are triggered to cause a task to receive the same sequence
    /// ID as another pending task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    pub fn blocking_invoke(&mut self, input: S) -> Result<R, Nonce<S>> {
        const DURATION: Duration = Duration::from_millis(5);

        // Claim the current sequence ID and increment `self.sequence` to a fresh sequence ID.
        let sequence = *self.sequence.blocking_read();
        *self.sequence.blocking_write() = sequence.wrapping_add(1);

        // Instruct the thread to evaluate `input`.
        self.as_sender().blocking_send((Some(sequence), input))?;

        let mut interval = Instant::now();

        loop {
            let now = Instant::now();

            if now < interval {
                let difference = interval - now;

                std::thread::sleep(difference);
            }

            interval = now + DURATION;

            let mut produced = self.produced.blocking_write();

            // If another task received the result first, it will drop it into `self.produced`.
            if let Some(result) = produced.remove(&sequence) {
                return Ok(result);
            }

            drop(produced);

            match self.as_receiver_mut().try_recv() {
                // If the value was returned by the task triggered above, return it.
                Ok((Some(seq), result)) if seq == sequence => return Ok(result),
                // If the value was returned by another task, drop it into `self.produced` so that
                // it can still be consumed.
                Ok((Some(seq), result)) => {
                    // This would require that enough tasks ([`usize::MAX`] to be exact) are
                    // triggered to cause a task to receive the same sequence ID as another pending
                    // task.
                    assert!(self.produced.blocking_write().insert(seq, result).is_none());
                }
                Ok((None, _)) => unreachable!("requests with no sequence number should not be returned"),
                // If task hasn't returned yet.
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => return Err(Error::Disconnected),
            };
        }
    }

    /// Invokes the thread, ignoring the response of the method.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    pub async fn invoke_and_forget(&mut self, input: S) -> Result<(), Nonce<S>> {
        self.as_sender().send((None, input)).await.map_err(Into::into)
    }

    /// Invokes the thread, ignoring the response of the method.
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
    pub fn blocking_invoke_and_forget(&mut self, input: S) -> Result<(), Nonce<S>> {
        self.as_sender().blocking_send((None, input)).map_err(Into::into)
    }
}

impl<S, R> HandleHolder<()> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_handle(&self) -> &JoinHandle<()> {
        self.inner.as_handle()
    }

    fn as_handle_mut(&mut self) -> &mut JoinHandle<()> {
        self.inner.as_handle_mut()
    }

    fn into_handle(self) -> JoinHandle<()> {
        self.inner.into_handle()
    }
}

impl<S, R> ConsumingThread<Nonce<S>> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_sender(&self) -> &Sender<Nonce<S>> {
        self.inner.as_sender()
    }

    fn as_sender_mut(&mut self) -> &mut Sender<Nonce<S>> {
        self.inner.as_sender_mut()
    }

    fn into_sender(self) -> Sender<Nonce<S>> {
        self.inner.into_sender()
    }

    fn clone_sender(&self) -> Sender<Nonce<S>> {
        self.inner.clone_sender()
    }
}

impl<S, R> ProducingThread<Nonce<R>> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_receiver(&self) -> &Receiver<Nonce<R>> {
        self.inner.as_receiver()
    }

    fn as_receiver_mut(&mut self) -> &mut Receiver<Nonce<R>> {
        self.inner.as_receiver_mut()
    }

    fn into_receiver(self) -> Receiver<Nonce<R>> {
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
    inner: Invoker<(State<T>, S), R>,
    /// The constant state.
    state: State<T>,
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
    pub fn spawn<N, F>(name: N, state: T, call: F, size: usize) -> StatefulResult<Self, T, S>
    where
        N: AsRef<str>,
        F: Fn(State<T>, S) -> R + Send + 'static,
    {
        let state = Arc::new(RwLock::new(state));

        Ok(Self { inner: Invoker::spawn(name, move |(s, i)| call(s, i), size)?, state })
    }

    /// Spawns a new thread with the given name and asynchronous task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn_with_runtime<N, F, O>(name: N, state: T, call: F, size: usize) -> StatefulResult<Self, T, S>
    where
        N: AsRef<str>,
        F: Fn(State<T>, S) -> O + Send + 'static,
        O: Future<Output = R> + Send,
    {
        let state = Arc::new(RwLock::new(state));

        Ok(Self { inner: Invoker::spawn_with_runtime(name, move |(s, i)| call(s, i), size)?, state })
    }

    /// Invokes the thread, returning the response of the method when available.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    pub async fn invoke(&mut self, input: S) -> StatefulResult<R, T, S> {
        self.inner.invoke((Arc::clone(&self.state), input)).await
    }

    /// Invokes the thread, returning the response of the method when available.
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
    pub fn blocking_invoke(&mut self, input: S) -> StatefulResult<R, T, S> {
        self.inner.blocking_invoke((Arc::clone(&self.state), input))
    }

    /// Invokes the thread, ignoring the response of the method.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread is disconnected.
    pub async fn invoke_and_forget(&mut self, input: S) -> StatefulResult<(), T, S> {
        self.inner.invoke_and_forget((Arc::clone(&self.state), input)).await
    }

    /// Invokes the thread, ignoring the response of the method.
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
    pub fn blocking_invoke_and_forget(&mut self, input: S) -> StatefulResult<(), T, S> {
        self.inner.blocking_invoke_and_forget((Arc::clone(&self.state), input))
    }
}

impl<T, S, R> HandleHolder<()> for StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_handle(&self) -> &JoinHandle<()> {
        self.inner.as_handle()
    }

    fn as_handle_mut(&mut self) -> &mut JoinHandle<()> {
        self.inner.as_handle_mut()
    }

    fn into_handle(self) -> JoinHandle<()> {
        self.inner.into_handle()
    }
}

impl<T, S, R> ConsumingThread<Nonce<(State<T>, S)>> for StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_sender(&self) -> &Sender<Nonce<(State<T>, S)>> {
        self.inner.as_sender()
    }

    fn as_sender_mut(&mut self) -> &mut Sender<Nonce<(State<T>, S)>> {
        self.inner.as_sender_mut()
    }

    fn into_sender(self) -> Sender<Nonce<(State<T>, S)>> {
        self.inner.into_sender()
    }

    fn clone_sender(&self) -> Sender<Nonce<(State<T>, S)>> {
        self.inner.clone_sender()
    }
}

impl<T, S, R> ProducingThread<Nonce<R>> for StatefulInvoker<T, S, R>
where
    T: Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_receiver(&self) -> &Receiver<Nonce<R>> {
        self.inner.as_receiver()
    }

    fn as_receiver_mut(&mut self) -> &mut Receiver<Nonce<R>> {
        self.inner.as_receiver_mut()
    }

    fn into_receiver(self) -> Receiver<Nonce<R>> {
        self.inner.into_receiver()
    }
}
