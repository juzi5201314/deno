// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use deno_core::error::bad_resource_id;
use deno_core::error::AnyError;
use deno_core::AsyncRefCell;
use deno_core::CancelFuture;
use deno_core::CancelHandle;
use deno_core::OpState;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ResourceId;
use deno_core::ZeroCopyBuf;
#[cfg(unix)]
use tokio::signal::unix::{signal, Signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::{ctrl_break, ctrl_c, CtrlBreak, CtrlC};

pub fn init(rt: &mut deno_core::JsRuntime) {
  super::reg_sync(rt, "op_signal_bind", op_signal_bind);
  super::reg_sync(rt, "op_signal_unbind", op_signal_unbind);
  super::reg_async(rt, "op_signal_poll", op_signal_poll);
}

#[cfg(unix)]
/// The resource for signal stream.
/// The second element is the waker of polling future.
struct SignalStreamResource {
  signal: AsyncRefCell<Signal>,
  cancel: CancelHandle,
}

#[cfg(unix)]
impl Resource for SignalStreamResource {
  fn name(&self) -> Cow<str> {
    "signal".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel.cancel();
  }
}

#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)]
fn op_signal_bind(
  state: &mut OpState,
  signo: i32,
  _zero_copy: Option<ZeroCopyBuf>,
) -> Result<ResourceId, AnyError> {
  super::check_unstable(state, "Deno.signal");
  let resource = SignalStreamResource {
    signal: AsyncRefCell::new(signal(SignalKind::from_raw(signo)).expect("")),
    cancel: Default::default(),
  };
  let rid = state.resource_table.add(resource);
  Ok(rid)
}

#[cfg(unix)]
async fn op_signal_poll(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  _zero_copy: Option<ZeroCopyBuf>,
) -> Result<bool, AnyError> {
  super::check_unstable2(&state, "Deno.signal");

  let resource = state
    .borrow_mut()
    .resource_table
    .get::<SignalStreamResource>(rid)
    .ok_or_else(bad_resource_id)?;
  let cancel = RcRef::map(&resource, |r| &r.cancel);
  let mut signal = RcRef::map(&resource, |r| &r.signal).borrow_mut().await;

  match signal.recv().or_cancel(cancel).await {
    Ok(result) => Ok(result.is_none()),
    Err(_) => Ok(true),
  }
}

#[cfg(unix)]
pub fn op_signal_unbind(
  state: &mut OpState,
  rid: ResourceId,
  _zero_copy: Option<ZeroCopyBuf>,
) -> Result<(), AnyError> {
  super::check_unstable(state, "Deno.signal");
  state
    .resource_table
    .close(rid)
    .ok_or_else(bad_resource_id)?;
  Ok(())
}

#[cfg(windows)]
enum WindowsSignal {
  SIGINT(CtrlC),
  SIGBREAK(CtrlBreak),
}

#[cfg(windows)]
impl From<CtrlC> for WindowsSignal {
  fn from(ctrl_c: CtrlC) -> Self {
    WindowsSignal::SIGINT(ctrl_c)
  }
}

#[cfg(windows)]
impl From<CtrlBreak> for WindowsSignal {
  fn from(ctrl_break: CtrlBreak) -> Self {
    WindowsSignal::SIGBREAK(ctrl_break)
  }
}

#[cfg(windows)]
impl WindowsSignal {
  pub async fn recv(&mut self) -> Option<()> {
    match self {
      WindowsSignal::SIGINT(ctrl_c) => ctrl_c.recv().await,
      WindowsSignal::SIGBREAK(ctrl_break) => ctrl_break.recv().await,
    }
  }
}

#[cfg(windows)]
struct SignalStreamResource {
  signal: AsyncRefCell<WindowsSignal>,
  cancel: CancelHandle,
}

#[cfg(windows)]
impl Resource for SignalStreamResource {
  fn name(&self) -> Cow<str> {
    "signal".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel.cancel();
  }
}

#[cfg(windows)]
pub fn op_signal_bind(
  state: &mut OpState,
  signo: i32,
  _zero_copy: Option<ZeroCopyBuf>,
) -> Result<ResourceId, AnyError> {
  super::check_unstable(state, "Deno.signal");
  let resource = SignalStreamResource {
    signal: AsyncRefCell::new(match signo {
      // SIGINT
      2 => ctrl_c().expect("").into(),
      // SIGBREAK
      21 => ctrl_break().expect("").into(),
      _ => unimplemented!(),
    }),
    cancel: Default::default(),
  };
  let rid = state.resource_table.add(resource);
  Ok(rid)
}

#[cfg(windows)]
async fn op_signal_poll(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  _zero_copy: Option<ZeroCopyBuf>,
) -> Result<bool, AnyError> {
  super::check_unstable2(&state, "Deno.signal");

  let resource = state
    .borrow_mut()
    .resource_table
    .get::<SignalStreamResource>(rid)
    .ok_or_else(bad_resource_id)?;
  let cancel = RcRef::map(&resource, |r| &r.cancel);
  let mut signal = RcRef::map(&resource, |r| &r.signal).borrow_mut().await;

  match signal.recv().or_cancel(cancel).await {
    Ok(result) => Ok(result.is_none()),
    Err(_) => Ok(true),
  }
}

#[cfg(windows)]
pub fn op_signal_unbind(
  state: &mut OpState,
  rid: ResourceId,
  _zero_copy: Option<ZeroCopyBuf>,
) -> Result<(), AnyError> {
  super::check_unstable(state, "Deno.signal");
  state
    .resource_table
    .close(rid)
    .ok_or_else(bad_resource_id)?;
  Ok(())
}
