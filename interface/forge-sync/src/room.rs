//! A document's room on the server: the sequencer plus who is connected, turning each client
//! message into the messages to send (to whom). No I/O: forge-server feeds it decoded messages
//! from its sockets and writes what it returns; the tests feed it directly.
//!
//! Authentication is the host's (it checks the token before a connection joins and names the
//! author); the room trusts the `author` it is given.

use crate::sequencer::Sequencer;
use crate::wire::{Down, Up};
use crate::{Refusal, Store};

/// The most entries a joining client is sent one by one; further behind, it gets a snapshot.
pub const CATCH_UP: u64 = 1024;
/// ... and at most this many bytes of them.
pub const CATCH_UP_BYTES: usize = 1 << 20;

/// A connection, by the host's number for it.
pub type Conn = u64;

struct Member {
    conn: Conn,
    client: String,
    author: String,
}

pub struct Room {
    pub seq: Sequencer,
    members: Vec<Member>,
}

/// What the room asks the host to do besides sending.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Write the head as a named version (the host owns git).
    Version {
        name: String,
        author: String,
        conn: Conn,
    },
    /// A client's build hash at `seq`, for the host to check against its own build of the text
    /// at `seq` (never against another client's word: one client could fake a divergence for
    /// everyone).
    CheckHash {
        seq: u64,
        hash: u64,
        author: String,
        conn: Conn,
    },
}

impl Room {
    pub fn new(seq: Sequencer) -> Room {
        Room { seq, members: vec![] }
    }

    pub fn members(&self) -> usize {
        self.members.len()
    }

    /// A connection joins (its Hello, after the host checked the token): a snapshot if it has
    /// nothing (`seq` None) or is ahead of the head, else every entry it missed (its own as Acks).
    pub fn join(&mut self, conn: Conn, client: &str, author: &str, seq: Option<u64>) -> Vec<(Conn, Down)> {
        self.members.retain(|m| m.conn != conn);
        self.members.push(Member {
            conn,
            client: client.into(),
            author: author.into(),
        });
        let you = author.to_string();
        let head = self.seq.seq();
        // a catch-up only when the log still has every entry missed and they are few and small;
        // else a snapshot (it is bounded by the document's own size)
        let catch_up = seq.filter(|&s| {
            s <= head
                && head - s <= CATCH_UP
                && s >= self.seq.floor()
                && self.seq.bytes_since(s) <= CATCH_UP_BYTES
        });
        let Some(seq) = catch_up else {
            return vec![(conn, self.snapshot(you))];
        };
        let mut out = vec![(
            conn,
            Down::Welcome {
                seq,
                text: String::new(),
                you,
                snap: false,
            },
        )];
        for e in self.seq.since(seq).into_iter().flatten() {
            out.push((conn, down_for(e, client)));
        }
        out
    }

    fn snapshot(&self, you: String) -> Down {
        Down::Welcome {
            seq: self.seq.seq(),
            text: self.seq.head().to_string(),
            you,
            snap: true,
        }
    }

    pub fn leave(&mut self, conn: Conn) {
        self.members.retain(|m| m.conn != conn);
    }

    /// One message from a joined connection.
    pub fn receive(&mut self, store: &mut dyn Store, conn: Conn, m: Up) -> (Vec<(Conn, Down)>, Vec<Action>) {
        let Some(me) = self.members.iter().find(|x| x.conn == conn) else {
            return (
                vec![(
                    conn,
                    Down::Error {
                        text: "not joined".into(),
                    },
                )],
                vec![],
            );
        };
        let (client, author) = (me.client.clone(), me.author.clone());
        match m {
            Up::Hello { .. } => (
                vec![(
                    conn,
                    Down::Error {
                        text: "already joined".into(),
                    },
                )],
                vec![],
            ),
            Up::Op { base, id, op } => match self.seq.submit(store, &author, &client, base, id, &op) {
                Ok(c) => {
                    let e = &c.entry;
                    let mut out = vec![(conn, down_for(e, &client))];
                    if !c.duplicate {
                        for m in self.members.iter().filter(|m| m.conn != conn) {
                            out.push((m.conn, down_for(e, &m.client)));
                        }
                    }
                    (out, vec![])
                }
                Err(refusal) => (vec![(conn, Down::Refused { id, refusal })], vec![]),
            },
            Up::BuildHash { seq, hash } => {
                if seq == 0 || seq > self.seq.seq() {
                    return (vec![], vec![]);
                }
                (
                    vec![],
                    vec![Action::CheckHash {
                        seq,
                        hash,
                        author,
                        conn,
                    }],
                )
            }
            Up::Version { name } => {
                if !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                    || name.is_empty()
                    || name.len() > 64
                {
                    let r = Refusal::Invalid {
                        why: "a version name is 1-64 letters, digits, _ and -".into(),
                    };
                    return (vec![(conn, Down::Error { text: r.text() })], vec![]);
                }
                (vec![], vec![Action::Version { name, author, conn }])
            }
            Up::Ping { t } => (vec![(conn, Down::Pong { t })], vec![]),
            Up::Resync => (vec![(conn, self.snapshot(author))], vec![]),
        }
    }

    /// Tells every member a version was written.
    pub fn versioned(&self, name: &str, seq: u64, commit: &str) -> Vec<(Conn, Down)> {
        let d = Down::Versioned {
            name: name.into(),
            seq,
            commit: commit.into(),
        };
        self.members.iter().map(|m| (m.conn, d.clone())).collect()
    }
}

/// An entry as `client` receives it: its own op as an Ack, another's as Applied.
fn down_for(e: &crate::Entry, client: &str) -> Down {
    if e.client == client {
        Down::Ack {
            id: e.id,
            seq: e.seq,
            op: e.op.clone(),
            inv: e.inv.clone(),
            th: e.th,
        }
    } else {
        Down::Applied {
            seq: e.seq,
            author: e.author.clone(),
            op: e.op.clone(),
            th: e.th,
        }
    }
}
