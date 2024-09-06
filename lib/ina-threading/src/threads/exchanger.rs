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

use std::future::Future;
use std::thread::JoinHandle;

use tokio::runtime::Builder;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{ConsumingThread, HandleHolder, ProducingThread, Result, Thread};

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

    /// Spawns a new thread with the given name and asynchronous task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    #[allow(clippy::missing_panics_doc)] // This doesn't actually *panic*, it just causes the thread to fail.
    pub fn spawn_with_runtime<N, F, O>(name: N, call: F, size: usize) -> Result<Self, S>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>, Receiver<S>) -> O + Send + 'static,
        O: Future<Output = T> + Send,
    {
        let call = |sender: Sender<R>, receiver: Receiver<S>| {
            #[allow(clippy::expect_used)] // If this can't spawn, we can't execute anything.
            let runtime = Builder::new_current_thread().enable_all().build().expect("failed to spawn runtime");

            runtime.block_on(async move { call(sender, receiver).await })
        };

        Self::spawn(name, call, size)
    }
}

impl<S, R, T> HandleHolder<T> for Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    fn as_handle(&self) -> &JoinHandle<T> {
        self.thread.as_handle()
    }

    fn as_handle_mut(&mut self) -> &mut JoinHandle<T> {
        self.thread.as_handle_mut()
    }

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
    fn as_sender(&self) -> &Sender<S> {
        &self.sender
    }

    fn as_sender_mut(&mut self) -> &mut Sender<S> {
        &mut self.sender
    }

    fn into_sender(self) -> Sender<S> {
        self.sender
    }

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
    fn as_receiver(&self) -> &Receiver<R> {
        &self.receiver
    }

    fn as_receiver_mut(&mut self) -> &mut Receiver<R> {
        &mut self.receiver
    }

    fn into_receiver(self) -> Receiver<R> {
        self.receiver
    }
}
