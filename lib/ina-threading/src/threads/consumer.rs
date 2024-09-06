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

use crate::{ConsumingThread, HandleHolder, Result, Thread};

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

    /// Spawns a new thread with the given name and asynchronous task.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    #[expect(clippy::missing_panics_doc, reason = "false-positive; the thread will just exit immediately, not panic")]
    pub fn spawn_with_runtime<N, F, O>(name: N, call: F, size: usize) -> Result<Self, S>
    where
        N: AsRef<str>,
        F: FnOnce(Receiver<S>) -> O + Send + 'static,
        O: Future<Output = T> + Send,
    {
        let call = |receiver: Receiver<S>| {
            #[expect(clippy::expect_used, reason = "if the runtime fails to spawn, we can't execute any code")]
            let runtime = Builder::new_current_thread().enable_all().build().expect("failed to spawn runtime");

            runtime.block_on(async move { call(receiver).await })
        };

        Self::spawn(name, call, size)
    }
}

impl<S, T> HandleHolder<T> for Consumer<S, T>
where
    S: Send + 'static,
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

impl<S, T> ConsumingThread<S> for Consumer<S, T>
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

    fn clone_sender(&self) -> Sender<S> {
        self.sender.clone()
    }
}
