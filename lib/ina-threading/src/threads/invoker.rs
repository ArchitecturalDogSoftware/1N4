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
use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

use super::exchanger::Exchanger;
use crate::{Handle, ReceiverHandle, Result, SenderHandle};

/// The thread type that is wrapped by an [`Invoker<S, R>`].
pub(crate) type InvokerInner<S, R> = Exchanger<Tracked<S>, Tracked<R>, Result<(), CallError<S, R>>>;

/// An error that may be returned when calling invoker threads.
#[derive(Debug, thiserror::Error)]
pub enum CallError<S, R> {
    /// Returned if a value cannot be sent into an invoker thread.
    #[error("unable to send into invoker thread: {0}")]
    SendInto(SendError<Tracked<S>>),
    /// Returned if a value cannot be returned from an invoker thread.
    #[error("unable to receive from invoker thread: {0}")]
    SendFrom(SendError<Tracked<R>>),
    /// Returned if the thread's receiving channel was closed.
    #[error("the thread's receiving channel was closed")]
    Closed,
}

/// A value with an associated nonce for response tracking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tracked<T> {
    /// The numeric nonce.
    pub nonce: Option<usize>,
    /// The inner value.
    pub value: T,
}

/// A value that is tracked as an invoker's state.
#[derive(Clone, Debug, Default)]
pub struct Stateful<T, S>
where
    T: ?Sized,
{
    /// The state.
    pub state: Arc<T>,
    /// The value.
    pub value: S,
}

/// A thread that consumes and returns values like a function.
#[derive(Debug)]
pub struct Invoker<S, R> {
    /// The inner exchanger thread.
    exchanger: InvokerInner<S, R>,
    /// A map that contains completed results.
    completed: BTreeMap<usize, R>,
    /// A sequence counter that tracks results.
    sequence: AtomicUsize,
}

impl<S, R> Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    /// Spawns a new [`Invoker<S, R>`] with the given name and task.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::Invoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = Invoker::spawn("worker", capacity, |n| {
    ///     assert_eq!(n, 123);
    ///     456
    /// })?;
    ///
    /// let response = thread.blocking_call(123).expect("the channel should not be closed");
    ///
    /// assert_eq!(response, 456);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F>(name: N, capacity: NonZero<usize>, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: Fn(S) -> R + Send + 'static,
    {
        let f = move |sender: Sender<Tracked<R>>, mut receiver: Receiver<Tracked<S>>| loop {
            let Some(received) = receiver.blocking_recv() else { return Ok(()) };
            let response = Tracked { nonce: received.nonce, value: f(received.value) };

            if received.nonce.is_some() {
                sender.blocking_send(response).map_err(CallError::SendFrom)?;
            }
        };

        Ok(Self {
            exchanger: Exchanger::spawn(name, capacity, f)?,
            completed: BTreeMap::new(),
            sequence: AtomicUsize::new(0),
        })
    }

    /// Spawns a new [`Invoker<S, R>`] with the given name and asynchronous task.
    ///
    /// The created runtime has both IO and time drivers enabled, and is configured to only run on the spawned thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::Invoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = Invoker::spawn_with_runtime("worker", capacity, |n| async move {
    ///     assert_eq!(n, 123);
    ///
    ///     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///
    ///     456
    /// })?;
    ///
    /// let response = thread.blocking_call(123).expect("the channel should not be closed");
    ///
    /// assert_eq!(response, 456);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn_with_runtime<N, F, O>(name: N, capacity: NonZero<usize>, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: Fn(S) -> O + Send + 'static,
        O: Future<Output = R> + Send,
    {
        let f = move |sender: Sender<Tracked<R>>, mut receiver: Receiver<Tracked<S>>| async move {
            loop {
                let Some(received) = receiver.recv().await else { return Ok(()) };
                let response = Tracked { nonce: received.nonce, value: f(received.value).await };

                if received.nonce.is_some() {
                    sender.send(response).await.map_err(CallError::SendFrom)?;
                }
            }
        };

        Ok(Self {
            exchanger: Exchanger::spawn_with_runtime(name, capacity, f)?,
            completed: BTreeMap::new(),
            sequence: AtomicUsize::new(0),
        })
    }

    /// Invokes the thread, returning the response of the inner function when available.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::Invoker;
    /// # #[tokio::main]
    /// # async fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = Invoker::spawn("worker", capacity, |(a, b)| a + b)?;
    ///
    /// let response = thread.call((2, 2)).await.expect("the channel should not be closed");
    ///
    /// // Unfortunately, Rust is incorrect and thinks that `2 + 2 != 5`.
    /// assert_eq!(response, 4);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if [`usize::MAX`] tasks have their responses queued, causing a response to be overwritten.
    ///
    /// # Errors
    ///
    /// This function will return an error if either of the thread's sender or receiver channels are closed.
    pub async fn call(&mut self, value: S) -> Result<R, CallError<S, R>> {
        let nonce = self.sequence.fetch_add(1, Ordering::AcqRel);
        let value = Tracked { nonce: Some(nonce), value };

        self.as_sender().send(value).await.map_err(CallError::SendInto)?;

        loop {
            if let Some(completed) = self.completed.remove(&nonce) {
                return Ok(completed);
            }

            match self.as_receiver_mut().recv().await {
                // If the value was returned by the task triggered above, return it.
                Some(Tracked { nonce: Some(completed_nonce), value }) if completed_nonce == nonce => return Ok(value),
                // If the value was returned by another task, store it so that it can still be consumed.
                Some(Tracked { nonce: Some(completed_nonce), value }) => {
                    // A panic here would require that enough tasks ([`usize::MAX`] to be exact) are triggered to cause
                    // a task to receive the same sequence ID as another pending task.
                    assert!(self.completed.insert(completed_nonce, value).is_none());
                }
                Some(Tracked { nonce: None, value: _ }) => unreachable!("values with no nonce should not be returned"),
                None => return Err(CallError::Closed),
            }
        }
    }

    /// Invokes the thread, blocking the current thread until the response of the inner function is available.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::Invoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = Invoker::spawn("worker", capacity, |(a, b)| a + b)?;
    ///
    /// let response = thread.blocking_call((2, 2)).expect("the channel should not be closed");
    ///
    /// // Unfortunately, Rust is incorrect and thinks that `2 + 2 != 5`.
    /// assert_eq!(response, 4);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if [`usize::MAX`] tasks have their responses queued, causing a response to be overwritten, or if this is
    /// called from within an asynchronous runtime.
    ///
    /// # Errors
    ///
    /// This function will return an error if either of the thread's sender or receiver channels are closed.
    pub fn blocking_call(&mut self, value: S) -> Result<R, CallError<S, R>> {
        let nonce = self.sequence.fetch_add(1, Ordering::AcqRel);
        let value = Tracked { nonce: Some(nonce), value };

        self.as_sender().blocking_send(value).map_err(CallError::SendInto)?;

        loop {
            if let Some(completed) = self.completed.remove(&nonce) {
                return Ok(completed);
            }

            match self.as_receiver_mut().blocking_recv() {
                // If the value was returned by the task triggered above, return it.
                Some(Tracked { nonce: Some(completed_nonce), value }) if completed_nonce == nonce => return Ok(value),
                // If the value was returned by another task, store it so that it can still be consumed.
                Some(Tracked { nonce: Some(completed_nonce), value }) => {
                    // A panic here would require that enough tasks ([`usize::MAX`] to be exact) are triggered to cause
                    // a task to receive the same sequence ID as another pending task.
                    assert!(self.completed.insert(completed_nonce, value).is_none());
                }
                Some(Tracked { nonce: None, value: _ }) => unreachable!("values with no nonce should not be returned"),
                None => return Err(CallError::Closed),
            }
        }
    }

    /// Invokes the thread, executing the method but ignoring the return value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::Invoker;
    /// # #[tokio::main]
    /// # async fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = Invoker::spawn("worker", capacity, |(a, b)| {
    ///     println!("{a} + {b} = {}", a + b);
    /// })?;
    ///
    /// thread.call_and_forget((2, 2)).await.expect("the channel should not be closed");
    ///
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's receiving channel is closed.
    pub async fn call_and_forget(&mut self, value: S) -> Result<(), CallError<S, R>> {
        self.as_sender().send(Tracked { nonce: None, value }).await.map_err(CallError::SendInto)
    }

    /// Invokes the thread, executing the method but ignoring the return value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::Invoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = Invoker::spawn("worker", capacity, |(a, b)| {
    ///     println!("{a} + {b} = {}", a + b);
    /// })?;
    ///
    /// thread.blocking_call_and_forget((2, 2)).expect("the channel should not be closed");
    ///
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if this is called from within an asynchronous runtime.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's receiving channel is closed.
    pub fn blocking_call_and_forget(&mut self, value: S) -> Result<(), CallError<S, R>> {
        self.as_sender().blocking_send(Tracked { nonce: None, value }).map_err(CallError::SendInto)
    }
}

impl<S, R> Handle for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    type Output = Result<(), CallError<S, R>>;

    fn as_join_handle(&self) -> &std::thread::JoinHandle<Self::Output> {
        self.exchanger.as_join_handle()
    }

    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<Self::Output> {
        self.exchanger.as_join_handle_mut()
    }

    fn into_join_handle(self) -> std::thread::JoinHandle<Self::Output> {
        self.exchanger.into_join_handle()
    }
}

impl<S, R> SenderHandle<Tracked<S>> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_sender(&self) -> &tokio::sync::mpsc::Sender<Tracked<S>> {
        self.exchanger.as_sender()
    }

    fn as_sender_mut(&mut self) -> &mut tokio::sync::mpsc::Sender<Tracked<S>> {
        self.exchanger.as_sender_mut()
    }

    fn into_sender(self) -> tokio::sync::mpsc::Sender<Tracked<S>> {
        self.exchanger.into_sender()
    }
}

impl<S, R> ReceiverHandle<Tracked<R>> for Invoker<S, R>
where
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_receiver(&self) -> &tokio::sync::mpsc::Receiver<Tracked<R>> {
        self.exchanger.as_receiver()
    }

    fn as_receiver_mut(&mut self) -> &mut tokio::sync::mpsc::Receiver<Tracked<R>> {
        self.exchanger.as_receiver_mut()
    }

    fn into_receiver(self) -> tokio::sync::mpsc::Receiver<Tracked<R>> {
        self.exchanger.into_receiver()
    }
}

/// A thread that consumes and returns values like a function.
///
/// This is a variant of a typical [`Invoker<S, R>`] that has a "state" value that is shared with
/// all invocations.
#[derive(Debug)]
pub struct StatefulInvoker<T, S, R>
where
    T: ?Sized,
{
    /// The inner invoker thread.
    invoker: Invoker<Stateful<T, S>, R>,
    /// The thread's canonical state.
    state: Arc<T>,
}

impl<T, S, R> StatefulInvoker<T, S, R>
where
    T: ?Sized + Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    /// Spawns a new [`StatefulInvoker<T, S, R>`] with the given name and task.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::StatefulInvoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = StatefulInvoker::spawn("worker", capacity, 2, |args| {
    ///     // `args` carries both the value and the thread's state.
    ///     args.value + *args.state
    /// })?;
    ///
    /// let response = thread.blocking_call(2).expect("the channel should not be closed");
    ///
    /// // Unfortunately, Rust is incorrect and thinks that `2 + 2 != 5`.
    /// assert_eq!(response, 4);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn<N, F, U>(name: N, capacity: NonZero<usize>, state: U, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: Fn(Stateful<T, S>) -> R + Send + 'static,
        U: Into<Arc<T>>,
    {
        Ok(Self { invoker: Invoker::spawn(name, capacity, f)?, state: state.into() })
    }

    /// Spawns a new [`StatefulInvoker<T, S, R>`] with the given name and asynchronous task.
    ///
    /// The created runtime has both IO and time drivers enabled, and is configured to only run on the spawned thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::StatefulInvoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread =
    ///     StatefulInvoker::spawn_with_runtime("worker", capacity, 2, |args| async move {
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///
    ///         args.value + *args.state
    ///     })?;
    ///
    /// let response = thread.blocking_call(2).expect("the channel should not be closed");
    ///
    /// // Unfortunately, Rust is incorrect and thinks that `2 + 2 != 5`.
    /// assert_eq!(response, 4);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread fails to spawn.
    pub fn spawn_with_runtime<N, F, O, U>(name: N, capacity: NonZero<usize>, state: U, f: F) -> Result<Self>
    where
        N: AsRef<str>,
        F: Fn(Stateful<T, S>) -> O + Send + 'static,
        O: Future<Output = R> + Send,
        U: Into<Arc<T>>,
    {
        Ok(Self { invoker: Invoker::spawn_with_runtime(name, capacity, f)?, state: state.into() })
    }

    /// Invokes the thread, returning the response of the inner function when available.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::StatefulInvoker;
    /// # #[tokio::main]
    /// # async fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = StatefulInvoker::spawn("worker", capacity, 2, |args| {
    ///     // `args` carries both the value and the thread's state.
    ///     args.value + *args.state
    /// })?;
    ///
    /// let response = thread.call(2).await.expect("the channel should not be closed");
    ///
    /// // Unfortunately, Rust is incorrect and thinks that `2 + 2 != 5`.
    /// assert_eq!(response, 4);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if [`usize::MAX`] tasks have their responses queued, causing a response to be overwritten.
    ///
    /// # Errors
    ///
    /// This function will return an error if either of the thread's sender or receiver channels are closed.
    pub async fn call(&mut self, value: S) -> Result<R, CallError<Stateful<T, S>, R>> {
        self.invoker.call(Stateful { state: Arc::clone(&self.state), value }).await
    }

    /// Invokes the thread, blocking the current thread until the response of the inner function is available.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::StatefulInvoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = StatefulInvoker::spawn("worker", capacity, 2, |args| {
    ///     // `args` carries both the value and the thread's state.
    ///     args.value + *args.state
    /// })?;
    ///
    /// let response = thread.blocking_call(2).expect("the channel should not be closed");
    ///
    /// // Unfortunately, Rust is incorrect and thinks that `2 + 2 != 5`.
    /// assert_eq!(response, 4);
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if [`usize::MAX`] tasks have their responses queued, causing a response to be overwritten, or if this is
    /// called from within an asynchronous runtime.
    ///
    /// # Errors
    ///
    /// This function will return an error if either of the thread's sender or receiver channels are closed.
    pub fn blocking_call(&mut self, value: S) -> Result<R, CallError<Stateful<T, S>, R>> {
        self.invoker.blocking_call(Stateful { state: Arc::clone(&self.state), value })
    }

    /// Invokes the thread, executing the method but ignoring the return value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::StatefulInvoker;
    /// # #[tokio::main]
    /// # async fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = StatefulInvoker::spawn("worker", capacity, 2, |args| {
    ///     println!("{} + {} = {}", args.value, args.state, args.value + *args.state);
    /// })?;
    ///
    /// thread.call_and_forget(2).await.expect("the channel should not be closed");
    ///
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's receiving channel is closed.
    pub async fn call_and_forget(&mut self, value: S) -> Result<(), CallError<Stateful<T, S>, R>> {
        self.invoker.call_and_forget(Stateful { state: Arc::clone(&self.state), value }).await
    }

    /// Invokes the thread, executing the method but ignoring the return value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use ina_threading::Handle;
    /// # use ina_threading::threads::invoker::StatefulInvoker;
    /// # fn main() -> ina_threading::Result<()> {
    /// let capacity = NonZero::<usize>::new(1).unwrap();
    /// let mut thread = StatefulInvoker::spawn("worker", capacity, 2, |args| {
    ///     println!("{} + {} = {}", args.value, args.state, args.value + *args.state);
    /// })?;
    ///
    /// thread.blocking_call_and_forget(2).expect("the channel should not be closed");
    ///
    /// assert!(thread.into_join_handle().join().is_ok());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if this is called from within an asynchronous runtime.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's receiving channel is closed.
    pub fn blocking_call_and_forget(&mut self, value: S) -> Result<(), CallError<Stateful<T, S>, R>> {
        self.invoker.blocking_call_and_forget(Stateful { state: Arc::clone(&self.state), value })
    }
}

impl<T, S, R> Handle for StatefulInvoker<T, S, R>
where
    T: ?Sized + Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    type Output = Result<(), CallError<Stateful<T, S>, R>>;

    fn as_join_handle(&self) -> &std::thread::JoinHandle<Self::Output> {
        self.invoker.as_join_handle()
    }

    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<Self::Output> {
        self.invoker.as_join_handle_mut()
    }

    fn into_join_handle(self) -> std::thread::JoinHandle<Self::Output> {
        self.invoker.into_join_handle()
    }
}

impl<T, S, R> SenderHandle<Tracked<Stateful<T, S>>> for StatefulInvoker<T, S, R>
where
    T: ?Sized + Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_sender(&self) -> &Sender<Tracked<Stateful<T, S>>> {
        self.invoker.as_sender()
    }

    fn as_sender_mut(&mut self) -> &mut Sender<Tracked<Stateful<T, S>>> {
        self.invoker.as_sender_mut()
    }

    fn into_sender(self) -> Sender<Tracked<Stateful<T, S>>> {
        self.invoker.into_sender()
    }
}

impl<T, S, R> ReceiverHandle<Tracked<R>> for StatefulInvoker<T, S, R>
where
    T: ?Sized + Send + Sync + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    fn as_receiver(&self) -> &Receiver<Tracked<R>> {
        self.invoker.as_receiver()
    }

    fn as_receiver_mut(&mut self) -> &mut Receiver<Tracked<R>> {
        self.invoker.as_receiver_mut()
    }

    fn into_receiver(self) -> Receiver<Tracked<R>> {
        self.invoker.into_receiver()
    }
}
