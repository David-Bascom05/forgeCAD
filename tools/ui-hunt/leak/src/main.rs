//! The heap still held after each of N forge-lang runs of one small part: flat if a run frees
//! what it made. `run` builds the same text N times; `edit` applies a SetParam N times as the
//! app's edits do (`edit::apply_checked_in`, which checks the result by running it).
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

struct Count;
static LIVE: AtomicIsize = AtomicIsize::new(0);
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Count = Count;

const PART: &str = "shared L = 60
shared W = 30
shared t = 6

sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = extrude(plan, t)
";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).cloned().unwrap_or_else(|| "run".into());
    let n: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let mut src = PART.to_string();
    for k in 0..n {
        if mode == "edit" {
            let j = forge_agent::json::Json::parse(&format!(
                r#"{{"op":"SetParam","param":"t","value":{{"value":{},"dim":"Length"}}}}"#,
                5 + k % 2
            ))
            .expect("json");
            let op = forge_lang::error::Op::from_json(&j).expect("an op");
            src = forge_lang::edit::apply_checked_in(&src, &op, None).expect("applies").0;
        } else {
            let _ = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
        }
        if (k + 1) % (n / 4).max(1) == 0 {
            println!("{mode} {} live {} KB", k + 1, LIVE.load(Ordering::Relaxed) / 1024);
        }
    }
}
