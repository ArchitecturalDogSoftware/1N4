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

use crate::{Handle, Result, SenderHandle, Thread};

/// A thread that receives values through a sender channel.
#[derive(Debug)]
pub struct Consumer<S, T> {
    /// The inner thread.
    thread: Thread<T>,
    /// The inner sender channel.
    sender: Sender<S>,
}

impl<S, T> Consumer<S, T>
where
    S: Send + 'static,
    T: Send + 'static,
{
    /// Spawns a new [`Consumer<S, T>`] with the given name and task.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZeroUsize;
    /// # use ina_threading::{Handle, SenderHandle};
    /// # use ina_threading::threads::consumer::Consumer;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZeroUsize::new(1).unwrap();
    /// let thread = Consumer::spawn("worker", capacity, |mut r| {
    ///     assert_eq!(r.blocking_recv(), Some(123));
    /// })?;
    ///
    /// thread.as_sender().blocking_send(123).expect("the channel should not be closed");
    ///
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, capacity: NonZeroUsize, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Receiver<S>) -> T + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity.get());

        Ok(Self { thread: Thread::spawn(name, || f(receiver))?, sender })
    }

    /// Spawns a new [`Consumer<S, T>`] with the given name and asynchronous task.
    ///
    /// The created runtime has both IO and time drivers enabled, and is configured to only run on the spawned thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZeroUsize;
    /// # use ina_threading::{Handle, SenderHandle};
    /// # use ina_threading::threads::consumer::Consumer;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZeroUsize::new(1).unwrap();
    /// let thread = Consumer::spawn_with_runtime("worker", capacity, |mut r| async move {
    ///     assert_eq!(r.recv().await, Some(123));
    /// })?;
    ///
    /// thread.as_sender().blocking_send(123).expect("the channel should not be closed");
    ///
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn_with_runtime<N, F, O>(name: N, capacity: NonZeroUsize, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: FnOnce(Receiver<S>) -> O + Send + 'static,
        O: Future<Output = T>,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity.get());

        Ok(Self { thread: Thread::spawn_with_runtime(name, || f(receiver))?, sender })
    }
}

impl<S, T> Handle for Consumer<S, T>
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

impl<S, T> SenderHandle<S> for Consumer<S, T>
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
