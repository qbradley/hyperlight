/*
Copyright 2025  The Hyperlight Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use tracing::{Span, instrument};

use super::{ParameterTuple, SupportedReturnType};
use crate::mem::memory_region::MemoryRegion;
use crate::sandbox::Callable;
use crate::{MultiUseSandbox, Result};
/// A context for calling guest functions.
///
/// Takes ownership of an existing `MultiUseSandbox`.
/// Once created, guest function calls may be made through this and only this context
/// until it is converted back to the `MultiUseSandbox` from which it originated.
///
/// Upon this conversion,the memory associated with the `MultiUseSandbox` it owns will be reset to the state it was in before
/// this context was created.
///
/// Calls made through this context will cause state to be retained across calls, until such time as the context
/// is converted back to a `MultiUseSandbox`
///
/// If dropped, the `MultiUseSandbox` from which it came will be also be dropped as it is owned by the
/// `MultiUseGuestCallContext` until it is converted back to a `MultiUseSandbox`
///
#[derive(Debug)]
pub struct MultiUseGuestCallContext {
    sbox: MultiUseSandbox,
}

impl MultiUseGuestCallContext {
    /// Take ownership  of a `MultiUseSandbox` and
    /// return a new `MultiUseGuestCallContext` instance.
    ///     
    #[instrument(skip_all, parent = Span::current())]
    pub fn start(sbox: MultiUseSandbox) -> Self {
        Self { sbox }
    }

    /// Close out the context and get back the internally-stored
    /// `MultiUseSandbox`. Future contexts opened by the returned sandbox
    /// will have guest state restored.
    #[instrument(err(Debug), skip(self), parent = Span::current())]
    pub fn finish(mut self) -> Result<MultiUseSandbox> {
        self.sbox.restore_state()?;
        Ok(self.sbox)
    }
    /// Close out the context and get back the internally-stored
    /// `MultiUseSandbox`.
    ///
    /// Note that this method is pub(crate) and does not reset the state of the
    /// sandbox.
    ///
    /// It is intended to be used when evolving a MultiUseSandbox to a new state
    /// and is not intended to be called publicly. It allows the state of the guest to be altered
    /// during the evolution of one sandbox state to another, enabling the new state created
    /// to be captured and stored in the Sandboxes state stack.
    ///
    pub(crate) fn finish_no_reset(self) -> MultiUseSandbox {
        self.sbox
    }

    /// Map a region of host memory into the sandbox.
    ///
    /// Depending on the host platform, there are likely alignment
    /// requirements of at least one page for base and len.
    ///
    /// `rgn.region_type` is ignored, since guest PTEs are not created
    /// for the new memory.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that the host side
    /// of the region remains intact and is not written to until this
    /// mapping is removed, either due to the destruction of the
    /// sandbox or due to a state rollback
    pub unsafe fn map_region(&mut self, rgn: &MemoryRegion) -> Result<()> {
        unsafe { self.sbox.map_region(rgn) }
    }

    /// Map the contents of a file into the guest at a particular address
    ///
    /// Returns the length of the mapping
    pub fn map_file_cow(&mut self, fp: &std::path::Path, guest_base: u64) -> Result<u64> {
        self.sbox.map_file_cow(fp, guest_base)
    }
}

impl Callable for MultiUseGuestCallContext {
    /// Call the guest function called `func_name` with the given arguments
    /// `args`, and expect the return value have the same type as
    /// `func_ret_type`.
    ///
    /// Every call to a guest function through this method will be made with the same "context"
    /// meaning that the guest state resulting from any previous call will be present/osbservable
    /// by the guest function called.
    ///
    /// If you want  to reset state, call `finish()` on this `MultiUseGuestCallContext`
    /// and get a new one from the resulting `MultiUseSandbox`
    #[instrument(err(Debug),skip(self, args),parent = Span::current())]
    fn call<Output: SupportedReturnType>(
        &mut self,
        func_name: &str,
        args: impl ParameterTuple,
    ) -> Result<Output> {
        // we are guaranteed to be holding a lock now, since `self` can't
        // exist without doing so. Since GuestCallContext is effectively
        // !Send (and !Sync), we also don't need to worry about
        // synchronization

        let ret = self.sbox.call_guest_function_by_name_no_reset(
            func_name,
            Output::TYPE,
            args.into_value(),
        );
        Output::from_value(ret?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::sync_channel;
    use std::thread::{self, JoinHandle};

    use hyperlight_testing::simple_guest_as_string;

    use super::MultiUseGuestCallContext;
    use crate::sandbox::Callable;
    use crate::sandbox_state::sandbox::EvolvableSandbox;
    use crate::sandbox_state::transition::Noop;
    use crate::{GuestBinary, HyperlightError, MultiUseSandbox, Result, UninitializedSandbox};

    fn new_uninit() -> Result<UninitializedSandbox> {
        let path = simple_guest_as_string().map_err(|e| {
            HyperlightError::Error(format!("failed to get simple guest path ({e:?})"))
        })?;
        UninitializedSandbox::new(GuestBinary::FilePath(path), None)
    }

    /// Test to create a `MultiUseSandbox`, then call several guest functions
    /// on it across different threads.
    ///
    /// This test works by passing messages between threads using Rust's
    /// [mpsc crate](https://doc.rust-lang.org/std/sync/mpsc). Details of this
    /// interaction are as follows.
    ///
    /// One thread acts as the receiver (AKA: consumer) and owns the
    /// `MultiUseSandbox`. This receiver fields requests from N senders
    /// (AKA: producers) to make batches of calls.
    ///
    /// Upon receipt of a message to execute a batch, a new
    /// `MultiUseGuestCallContext` is created in the receiver thread from the
    /// existing `MultiUseSandbox`, and the batch is executed.
    ///
    /// After the batch is complete, the `MultiUseGuestCallContext` is done
    /// and it is converted back to the underlying `MultiUseSandbox`
    #[test]
    fn test_multi_call_multi_thread() {
        let (snd, recv) = sync_channel::<Vec<TestFuncCall>>(0);

        // create new receiver thread and on it, begin listening for
        // requests to execute batches of calls
        let recv_hdl = thread::spawn(move || {
            let mut sbox: MultiUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
            while let Ok(calls) = recv.recv() {
                let mut ctx = sbox.new_call_context();
                for call in calls {
                    call.call(&mut ctx);
                }
                sbox = ctx.finish().unwrap();
            }
        });

        // create new sender threads
        let send_handles: Vec<JoinHandle<()>> = (0..10)
            .map(|i| {
                let sender = snd.clone();
                thread::spawn(move || {
                    let calls = vec![
                        TestFuncCall::new(move |ctx| {
                            let msg = format!("Hello {}", i);
                            let ret: String = ctx.call("Echo", msg.clone()).unwrap();
                            assert_eq!(ret, msg)
                        }),
                        TestFuncCall::new(move |ctx| {
                            let ret: i32 = ctx.call("CallMalloc", i + 2).unwrap();
                            assert_eq!(ret, i + 2)
                        }),
                    ];
                    sender.send(calls).unwrap();
                })
            })
            .collect();

        for hdl in send_handles {
            hdl.join().unwrap();
        }
        // after all sender threads are done, drop the sender itself
        // so the receiver thread can exit. then, ensure the receiver
        // thread has exited.
        drop(snd);
        recv_hdl.join().unwrap();
    }

    pub struct TestSandbox {
        sandbox: MultiUseSandbox,
    }

    impl TestSandbox {
        pub fn new() -> Self {
            let sbox: MultiUseSandbox = new_uninit().unwrap().evolve(Noop::default()).unwrap();
            Self { sandbox: sbox }
        }
        pub fn call_add_to_static_multiple_times(mut self, i: i32) -> Result<TestSandbox> {
            let mut ctx = self.sandbox.new_call_context();
            let mut sum: i32 = 0;
            for n in 0..i {
                let result = ctx.call::<i32>("AddToStatic", n);
                sum += n;
                println!("{:?}", result);
                let result = result.unwrap();
                assert_eq!(result, sum);
            }
            let result = ctx.finish();
            assert!(result.is_ok());
            self.sandbox = result.unwrap();
            Ok(self)
        }

        pub fn call_add_to_static(mut self, i: i32) -> Result<()> {
            for n in 0..i {
                let result = self
                    .sandbox
                    .call_guest_function_by_name::<i32>("AddToStatic", n);
                println!("{:?}", result);
                let result = result.unwrap();
                assert_eq!(result, n);
            }
            Ok(())
        }
    }

    #[test]
    fn ensure_multiusesandbox_multi_calls_dont_reset_state() {
        let sandbox = TestSandbox::new();
        let result = sandbox.call_add_to_static_multiple_times(5);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_multiusesandbox_single_calls_do_reset_state() {
        let sandbox = TestSandbox::new();
        let result = sandbox.call_add_to_static(5);
        assert!(result.is_ok());
    }

    struct TestFuncCall(Box<dyn FnOnce(&mut MultiUseGuestCallContext) + Send>);

    impl TestFuncCall {
        fn new(f: impl FnOnce(&mut MultiUseGuestCallContext) + Send + 'static) -> Self {
            TestFuncCall(Box::new(f))
        }

        fn call(self, ctx: &mut MultiUseGuestCallContext) {
            (self.0)(ctx);
        }
    }
}
