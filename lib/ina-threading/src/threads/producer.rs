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

use crate::{HandleHolder, ProducingThread, Result, Thread};

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

    /// Spawns a new thread with the given name and asynchronous task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    #[allow(clippy::missing_panics_doc)] // this doesn't actually *panic*, it just causes the thread to fail.
    pub fn spawn_with_runtime<N, F, O>(name: N, call: F, size: usize) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Sender<R>) -> O + Send + 'static,
        O: Future<Output = T> + Send,
    {
        let call = |receiver: Sender<R>| {
            #[allow(clippy::expect_used)] // if this can't spawn, we can't execute anything.
            let runtime = Builder::new_current_thread().enable_all().build().expect("failed to spawn runtime");

            runtime.block_on(async move { call(receiver).await })
        };

        Self::spawn(name, call, size)
    }
}

impl<R, T> HandleHolder<T> for Producer<R, T>
where
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

impl<R, T> ProducingThread<R> for Producer<R, T>
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
