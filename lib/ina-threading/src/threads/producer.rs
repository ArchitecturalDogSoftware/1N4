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

use crate::{Handle, ReceiverHandle, Result, Thread};

/// A thread that accepts values through a receiver channel.
#[derive(Debug)]
pub struct Producer<R, T> {
    /// The inner thread.
    thread: Thread<T>,
    /// The inner receiver channel.
    receiver: Receiver<R>,
}

impl<R, T> Producer<R, T>
where
    R: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new [`Producer<R, T>`] with the given name and task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, capacity: NonZeroUsize, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>) -> T + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity.get());

        Ok(Self { thread: Thread::spawn(name, || f(sender))?, receiver })
    }

    /// Spawns a new [`Producer<R, T>`] with the given name and asynchronous task.
    ///
    /// The created runtime has both IO and time drivers enabled, and is configured to only run on the spawned thread.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn_with_runtime<N, F, O>(name: N, capacity: NonZeroUsize, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>) -> O + Send + 'static,
        O: Future<Output = T>,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity.get());

        Ok(Self { thread: Thread::spawn_with_runtime(name, || f(sender))?, receiver })
    }
}

impl<R, T> Handle for Producer<R, T>
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

impl<R, T> ReceiverHandle<R> for Producer<R, T>
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
