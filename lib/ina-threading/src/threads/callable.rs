// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025 Jaxydog
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

//! Defines callable threads.

use std::num::NonZero;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use tokio::sync::mpsc::Sender as MpscSender;
use tokio::sync::mpsc::error::SendError as MpscSendError;
use tokio::sync::oneshot::Sender as OneshotSender;
use tokio::sync::oneshot::error::RecvError as OneshotRecvError;

use crate::{JoinHandle, JoinHandleWrapper};

/// An error that may be returned when invoking a callable join handle.
#[derive(Debug, thiserror::Error)]
pub enum Error<S, R> {
    /// The provided value could not be sent to the thread.
    #[error(transparent)]
    SendToThread(#[from] MpscSendError<(S, OneshotSender<R>)>),
    /// The return value could not be received from the thread.
    #[error(transparent)]
    RecvFromThread(#[from] OneshotRecvError),
}

/// A thread that can be "invoked" like a function.
#[derive(Debug)]
pub struct CallableJoinHandle<S, R> {
    /// The sender-end of the linked channel.
    sender: MpscSender<(S, OneshotSender<R>)>,
    /// The inner join handle.
    handle: JoinHandle<()>,
}

impl<S, R> CallableJoinHandle<S, R> {
    /// Creates a new [`CallableJoinHandle<S, R>`] using the given function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the operating system fails to spawn the thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::callable::CallableJoinHandle;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let capacity = NonZero::new(1).unwrap();
    /// let handle = CallableJoinHandle::spawn(capacity, |(a, b): (i32, i32)| a + b)?;
    ///
    /// assert_eq!(handle.invoke((2, 5)).await.unwrap(), 7);
    /// # handle.into_join_handle().join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[expect(clippy::missing_panics_doc, reason = "the assertion will not directly cause a panic")]
    #[inline]
    pub fn spawn<F>(capacity: NonZero<usize>, f: F) -> std::io::Result<Self>
    where
        S: Send + 'static,
        R: Send + 'static,
        F: Fn(S) -> R + Send + 'static,
    {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<(S, OneshotSender<R>)>(capacity.get());

        JoinHandle::spawn(move || {
            while let Some((value, sender)) = receiver.blocking_recv() {
                assert!(sender.send(f(value)).is_ok(), "the oneshot channel was closed prematurely");
            }
        })
        .map(|handle| Self { sender, handle })
    }

    /// Invokes the thread like a function, sending the given value and awaiting the thread's response.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's channel was closed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// #
    /// # use std::thread::JoinHandle;
    /// #
    /// # use ina_threading::threads::callable::CallableJoinHandle;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let capacity = NonZero::new(1).unwrap();
    /// let handle = CallableJoinHandle::spawn(capacity, |(a, b): (i32, i32)| a + b)?;
    ///
    /// assert_eq!(handle.invoke((2, 5)).await.unwrap(), 7);
    /// # JoinHandle::from(handle).join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invoke(&self, value: S) -> Result<R, Error<S, R>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.sender.send((value, sender)).await?;

        receiver.await.map_err(Into::into)
    }
}

impl<S, R> JoinHandleWrapper for CallableJoinHandle<S, R> {
    type Output = ();

    #[inline]
    fn as_join_handle(&self) -> &std::thread::JoinHandle<()> {
        self.handle.as_join_handle()
    }

    #[inline]
    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<()> {
        self.handle.as_join_handle_mut()
    }

    #[inline]
    fn into_join_handle(self) -> std::thread::JoinHandle<()> {
        self.handle.into_join_handle()
    }
}

impl<S, R> AsRef<std::thread::JoinHandle<()>> for CallableJoinHandle<S, R> {
    #[inline]
    fn as_ref(&self) -> &std::thread::JoinHandle<()> {
        self.as_join_handle()
    }
}

impl<S, R> Deref for CallableJoinHandle<S, R> {
    type Target = std::thread::JoinHandle<()>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_join_handle()
    }
}

impl<S, R> AsMut<std::thread::JoinHandle<()>> for CallableJoinHandle<S, R> {
    #[inline]
    fn as_mut(&mut self) -> &mut std::thread::JoinHandle<()> {
        self.as_join_handle_mut()
    }
}

impl<S, R> DerefMut for CallableJoinHandle<S, R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_join_handle_mut()
    }
}

impl<S, R> From<CallableJoinHandle<S, R>> for std::thread::JoinHandle<()> {
    #[inline]
    fn from(value: CallableJoinHandle<S, R>) -> Self {
        value.into_join_handle()
    }
}

/// A thread that can be "invoked" like a function.
#[derive(Debug)]
pub struct StatefulCallableJoinHandle<S, R, V> {
    /// The thread's inner state.
    state: Arc<V>,
    /// The inner join handle.
    handle: CallableJoinHandle<(Arc<V>, S), R>,
}

impl<S, R, V> StatefulCallableJoinHandle<S, R, V> {
    /// Creates a new [`StatefulCallableJoinHandle<S, R, V>`] using the given function.
    ///
    /// # Errors
    ///
    /// This function will return an error if the operating system fails to spawn the thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use std::sync::Arc;
    /// #
    /// # use ina_threading::JoinHandleWrapper;
    /// # use ina_threading::threads::callable::StatefulCallableJoinHandle;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let capacity = NonZero::new(1).unwrap();
    /// let state = Arc::new(5);
    /// let handle =
    ///     StatefulCallableJoinHandle::spawn(capacity, state, |(state, value)| value + *state)?;
    ///
    /// assert_eq!(handle.invoke(2).await.unwrap(), 7);
    /// # handle.into_join_handle().join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn spawn<F>(capacity: NonZero<usize>, state: Arc<V>, f: F) -> std::io::Result<Self>
    where
        S: Send + 'static,
        R: Send + 'static,
        V: Send + Sync + 'static,
        F: Fn((Arc<V>, S)) -> R + Send + 'static,
    {
        CallableJoinHandle::spawn(capacity, f).map(|handle| Self { state, handle })
    }

    /// Invokes the thread like a function, sending the given value and awaiting the thread's response.
    ///
    /// # Errors
    ///
    /// This function will return an error if the thread's channel was closed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::NonZero;
    /// # use std::sync::Arc;
    /// #
    /// # use std::thread::JoinHandle;
    /// #
    /// # use ina_threading::threads::callable::StatefulCallableJoinHandle;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> std::io::Result<()> {
    /// let capacity = NonZero::new(1).unwrap();
    /// let state = Arc::new(5);
    /// let handle =
    ///     StatefulCallableJoinHandle::spawn(capacity, state, |(state, value)| value + *state)?;
    ///
    /// assert_eq!(handle.invoke(2).await.unwrap(), 7);
    /// # JoinHandle::from(handle).join().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invoke(&self, value: S) -> Result<R, Error<(Arc<V>, S), R>>
    where
        S: Send,
        R: Send,
        V: Send + Sync,
    {
        self.handle.invoke((Arc::clone(&self.state), value)).await
    }
}

impl<S, R, V> JoinHandleWrapper for StatefulCallableJoinHandle<S, R, V> {
    type Output = ();

    #[inline]
    fn as_join_handle(&self) -> &std::thread::JoinHandle<()> {
        self.handle.as_join_handle()
    }

    #[inline]
    fn as_join_handle_mut(&mut self) -> &mut std::thread::JoinHandle<()> {
        self.handle.as_join_handle_mut()
    }

    #[inline]
    fn into_join_handle(self) -> std::thread::JoinHandle<()> {
        self.handle.into_join_handle()
    }
}

impl<S, R, V> AsRef<std::thread::JoinHandle<()>> for StatefulCallableJoinHandle<S, R, V> {
    #[inline]
    fn as_ref(&self) -> &std::thread::JoinHandle<()> {
        self.as_join_handle()
    }
}

impl<S, R, V> Deref for StatefulCallableJoinHandle<S, R, V> {
    type Target = std::thread::JoinHandle<()>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_join_handle()
    }
}

impl<S, R, V> AsMut<std::thread::JoinHandle<()>> for StatefulCallableJoinHandle<S, R, V> {
    #[inline]
    fn as_mut(&mut self) -> &mut std::thread::JoinHandle<()> {
        self.as_join_handle_mut()
    }
}

impl<S, R, V> DerefMut for StatefulCallableJoinHandle<S, R, V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_join_handle_mut()
    }
}

impl<S, R, V> From<StatefulCallableJoinHandle<S, R, V>> for std::thread::JoinHandle<()> {
    #[inline]
    fn from(value: StatefulCallableJoinHandle<S, R, V>) -> Self {
        value.into_join_handle()
    }
}
