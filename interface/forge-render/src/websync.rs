//! The browser build's link to a served project (research/hosted/DESIGN.md §14): the same
//! forge-sync client as the desktop's, on the page's own WebSocket.
//!
//! The page tells the app where it is served (`app_sync_config`, from `?sync=` and
//! `?sync_token=`, kept for the tab like `forgecad web`'s token); with no such word the browser
//! build is LOCAL exactly as before. The address must be this machine's (phase 1 has no TLS), and
//! the hosted server must accept the page's origin (`forge-server --origin`).
//!
//! Everything runs on the page's thread: the socket's callbacks feed the client, which the part
//! session polls each frame through [`Link`].

use crate::sync_link::Link;
use forge_lang::error::Op;
use forge_sync::wire::{Down, Up};
use forge_sync::{Client, Event};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

thread_local! {
    /// The page's word: (ws:// URL, token).
    static CONFIG: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// The page: this tab is served at `url` with `token` (an empty url: LOCAL again). Refused for
/// an address that is not this machine's.
#[wasm_bindgen]
pub fn app_sync_config(url: String, token: String) -> Result<(), JsValue> {
    if url.is_empty() {
        CONFIG.with(|c| *c.borrow_mut() = None);
        return Ok(());
    }
    let host = url
        .strip_prefix("ws://")
        .and_then(|r| r.split('/').next())
        .ok_or_else(|| JsValue::from_str("?sync= takes ws://HOST:PORT/sync"))?;
    let bare = host
        .rsplit_once(':')
        .map_or(host, |x| x.0)
        .trim_start_matches('[')
        .trim_end_matches(']');
    let loopback = bare.eq_ignore_ascii_case("localhost") || bare.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if !loopback {
        return Err(JsValue::from_str(
            "?sync= must be this machine (phase 1 has no TLS: a token sent elsewhere would cross the network in clear)",
        ));
    }
    CONFIG.with(|c| *c.borrow_mut() = Some((url, token)));
    Ok(())
}

struct Inner {
    client: Client,
    ws: Option<WebSocket>,
    events: Vec<Event>,
    url: String,
    connected: bool,
    /// The server refused for good (a bad token, another kernel, a quarantined document): no
    /// reconnect (it would be refused the same way, for ever).
    stopped: bool,
    waker: Arc<dyn Fn() + Send + Sync>,
    /// The socket's callbacks, kept alive while it is.
    #[allow(clippy::type_complexity)]
    held: Vec<Closure<dyn FnMut(JsValue)>>,
}

impl Inner {
    fn flush(&mut self) {
        let msgs = std::mem::take(&mut self.client.outbox);
        if let (true, Some(ws)) = (self.connected, &self.ws) {
            for m in msgs {
                if ws.send_with_str(&m.text()).is_err() {
                    self.connected = false;
                    break;
                }
            }
        }
        // not connected: dropped; the reconnect says Hello again and re-sends every pending op
    }
}

pub struct WebLink(Rc<RefCell<Inner>>);

/// A served part's link, if the page said the tab is served. `path` is the project path the app
/// opened (`/project/parts/p.fg`): the document's name is the part below `/project/`.
pub fn open(path: &str, waker: Arc<dyn Fn() + Send + Sync>) -> Option<Result<Box<dyn Link>, String>> {
    let (url, token) = CONFIG.with(|c| c.borrow().clone())?;
    let doc = path.strip_prefix("/project/").unwrap_or(path).to_string();
    let inner = Rc::new(RefCell::new(Inner {
        client: Client::new(&doc, &client_id(), &token),
        ws: None,
        events: vec![],
        url,
        connected: false,
        stopped: false,
        waker,
        held: vec![],
    }));
    connect(&inner);
    Some(Ok(Box::new(WebLink(inner))))
}

fn client_id() -> String {
    let r = js_sys::Math::random();
    format!("w{:013x}", (r * 4_503_599_627_370_496.0) as u64)
}

fn connect(inner: &Rc<RefCell<Inner>>) {
    let url = inner.borrow().url.clone();
    let Ok(ws) = WebSocket::new(&url) else {
        return retry(inner);
    };
    let on_open = {
        let i = inner.clone();
        Closure::<dyn FnMut(JsValue)>::new(move |_| {
            let mut s = i.borrow_mut();
            s.connected = true;
            // Hello from the last seq applied, and every pending op again
            s.client.reconnect();
            s.flush();
        })
    };
    let on_message = {
        let i = inner.clone();
        Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
            let Some(text) = e.dyn_ref::<MessageEvent>().and_then(|m| m.data().as_string()) else {
                return;
            };
            let Ok(d) = Down::parse(&text) else { return };
            let waker = {
                let mut s = i.borrow_mut();
                let ev = s.client.receive(d);
                if ev.iter().any(|e| matches!(e, Event::Stopped(_))) {
                    s.stopped = true;
                }
                s.events.extend(ev);
                s.flush();
                s.waker.clone()
            };
            waker();
        })
    };
    let on_close = {
        let i = inner.clone();
        Closure::<dyn FnMut(JsValue)>::new(move |_| {
            {
                let mut s = i.borrow_mut();
                s.connected = false;
                s.ws = None;
                if s.stopped {
                    return;
                }
            }
            retry(&i);
        })
    };
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    let mut s = inner.borrow_mut();
    s.ws = Some(ws);
    s.held = vec![on_open, on_message, on_close];
}

/// Connects again a second later.
fn retry(inner: &Rc<RefCell<Inner>>) {
    let i = inner.clone();
    let again = Closure::once_into_js(move || connect(&i));
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(again.unchecked_ref(), 1000);
    }
}

impl Link for WebLink {
    fn ready(&self) -> bool {
        !self.0.borrow().client.joining
    }
    fn text(&self) -> String {
        self.0.borrow().client.text().to_string()
    }
    fn seq(&self) -> u64 {
        self.0.borrow().client.seq()
    }
    fn pending(&self) -> usize {
        self.0.borrow().client.pending().len()
    }
    fn local(&self, op: Op, after: String) {
        let mut s = self.0.borrow_mut();
        s.client.local(op, after);
        s.flush();
    }
    fn local_group(&self, ops: Vec<Op>, after: String) {
        let mut s = self.0.borrow_mut();
        s.client.local_group(ops, after);
        s.flush();
    }
    fn undo(&self) -> Option<Result<(), String>> {
        let mut s = self.0.borrow_mut();
        let r = s.client.undo();
        s.flush();
        r.map(|r| r.map(|_| ()).map_err(|e| format!("{}: {}", e.kind(), e.text())))
    }
    fn built(&self, seq: u64, hash: u64) {
        let mut s = self.0.borrow_mut();
        s.client.built(seq, hash);
        s.flush();
    }
    fn version(&self, name: &str) {
        let mut s = self.0.borrow_mut();
        s.client.outbox.push(Up::Version { name: name.into() });
        s.flush();
    }
    fn poll(&self) -> Vec<Event> {
        std::mem::take(&mut self.0.borrow_mut().events)
    }
    fn saved(&self) -> String {
        // (the browser keeps no pending ops across a closed tab yet)
        String::new()
    }
}

impl Drop for WebLink {
    fn drop(&mut self) {
        let mut s = self.0.borrow_mut();
        if let Some(ws) = s.ws.take() {
            ws.set_onclose(None);
            let _ = ws.close();
        }
        s.held.clear();
    }
}
