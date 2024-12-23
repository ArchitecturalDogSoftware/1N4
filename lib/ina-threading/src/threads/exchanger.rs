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

use std::num::NonZeroUsize;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{Handle, ReceiverHandle, Result, SenderHandle, Thread};

/// A thread that both consumes and produces values through channels.
#[derive(Debug)]
pub struct Exchanger<S, R, T> {
    /// The inner thread handle.
    thread: Thread<T>,
    /// The inner sending channel.
    sender: Sender<S>,
    /// The inner receiving channel.
    receiver: Receiver<R>,
}

impl<S, R, T> Exchanger<S, R, T>
where
    S: Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new [`Exchanger<S, R, T>`] with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, capacity: NonZeroUsize, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>, Receiver<S>) -> T + Send + 'static,
    {
        let (local_sender, thread_receiver) = tokio::sync::mpsc::channel(capacity.get());
        let (thread_sender, local_receiver) = tokio::sync::mpsc::channel(capacity.get());
        let thread = Thread::spawn(name, move || f(thread_sender, thread_receiver))?;

        Ok(Self { thread, sender: local_sender, receiver: local_receiver })
    }

    /// Spawns a new [`Exchanger<S, R, T>`] with the given name and asynchronous task.
    ///
    /// The created runtime has both IO and time drivers enabled, and is configured to only run on the spawned thread.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn_with_runtime<N, F, O>(name: N, capacity: NonZeroUsize, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>, Receiver<S>) -> O + Send + 'static,
        O: Future<Output = T> + Send,
    {
        let (local_sender, thread_receiver) = tokio::sync::mpsc::channel(capacity.get());
        let (thread_sender, local_receiver) = tokio::sync::mpsc::channel(capacity.get());
        let thread = Thread::spawn_with_runtime(name, || f(thread_sender, thread_receiver))?;

        Ok(Self { thread, sender: local_sender, receiver: local_receiver })
    }
}

impl<S, R, T> Handle for Exchanger<S, R, T>
where
    T: Send + 'static,
{
    type Output = T;

    fn as_join_handle(&self) -> &std::thread::JoinHandle<Self::Output> {
        self.thread.as_join_handle()
    }

    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<Self::Output> {
        self.thread.as_join_handle_mut()
    }

    fn into_join_handle(self) -> std::thread::JoinHandle<Self::Output> {
        self.thread.into_join_handle()
    }
}

impl<S, R, T> SenderHandle<S> for Exchanger<S, R, T>
where
    S: Send + 'static,
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
}

impl<S, R, T> ReceiverHandle<R> for Exchanger<S, R, T>
where
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
