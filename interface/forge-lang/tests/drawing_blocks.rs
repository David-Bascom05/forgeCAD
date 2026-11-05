//! Drawings in the same file: `drawing NAME { .. }` blocks of a .fg, the document's model drawn,
//! listed by its run without drawing them, each made on its own, and edited inside its braces.

use forge_lang::drawing::block;
use forge_lang::drawing::ops::DrawOp;
use std::path::Path;

const EX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/unified");

fn knob() -> String {
    std::fs::read_to_string(Path::new(EX).join("knob_drawn.fg")).unwrap()
}

#[test]
fn a_block_parses_and_the_model_skips_it() {
    let src = knob();
    let b = block::blocks(&src);
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].name, "dwg_knob");
    // the word alone is still a drawing file's first line
    assert!(forge_lang::drawing::is_drawing(
        "drawing\nm = part(\"knob.fg\")\n"
    ));
    let opts = forge_lang::Options {
        dir: Some(EX.into()),
        ..Default::default()
    };
    let r = forge_lang::run(&src, opts, Default::default()).0;
    assert!(
        r.ok(),
        "{:?}",
        r.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    let ids: Vec<&str> = r.feats.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["s", "body"]);
    let j = format!("{}", forge_lang::report::json(&r, &src));
    assert!(j.contains("\"drawings\":[{\"id\":\"dwg_knob\""), "{j}");
}

#[test]
fn a_block_draws_the_document_it_is_in() {
    let src = knob();
    let d = block::make(&src, Path::new(EX), None, "knob_drawn.fg", "dwg_knob").unwrap();
    assert!(d.ok(), "{:?}", d.errors);
    assert_eq!(d.views.len(), 2);
    assert!(d.views.iter().all(|v| !v.pieces.is_empty()));
    assert!(block::make(&src, Path::new(EX), None, "knob_drawn.fg", "nope").is_none());
}

#[test]
fn a_blocks_problems_are_where_they_are_in_the_document() {
    let src = knob().replace("  side = projected", "  bad = nowhere(1)\n  side = projected");
    let d = block::read(&src, "knob_drawn.fg", "dwg_knob").unwrap();
    let e = d.errors.first().expect("an error");
    assert_eq!(
        &src[e.span.lo as usize..e.span.lo as usize + 7],
        "nowhere",
        "{e:?}"
    );
}

#[test]
fn an_op_on_a_block_writes_inside_its_braces() {
    let src = knob();
    let op = DrawOp::AddItem {
        text: "n1 = note(\"ALL EDGES 0.5\", sheet: s1, at: (40, 30))".into(),
    };
    let out = block::apply_checked(&src, "dwg_knob", &op).unwrap();
    assert!(
        out.contains("  n1 = note(\"ALL EDGES 0.5\", sheet: s1, at: (40, 30))\n}\n"),
        "{out}"
    );
    assert!(out.starts_with(&src[..src.find("drawing dwg_knob").unwrap()]));
    let d = block::read(&out, "knob_drawn.fg", "dwg_knob").unwrap();
    assert!(d.errors.is_empty() && d.item("n1").is_some());
    let gone = block::apply_checked(&out, "dwg_knob", &DrawOp::DeleteItem { item: "n1".into() }).unwrap();
    assert_eq!(gone, src);
}

#[test]
fn two_blocks_each_have_their_own_ids_and_one_may_draw_another_file() {
    let src = format!(
        "{}\ndrawing other {{\n  m = part(\"knob.fg\")\n  s1 = sheet(\"A\")\n  top = base_view(m, sheet: s1, dir: \"top\", at: (100, 150))\n}}\n",
        knob()
    );
    assert_eq!(block::blocks(&src).len(), 2);
    let d = block::make(&src, Path::new(EX), None, "knob_drawn.fg", "other").unwrap();
    assert!(d.ok(), "{:?}", d.errors);
    assert_eq!(d.views.len(), 1);
}

#[test]
fn a_blocks_write_merges_into_unsaved_text_of_the_rest() {
    let base = knob();
    let mine = base.replace("shared r = 9", "shared r = 11");
    let disk = block::apply_checked(
        &base,
        "dwg_knob",
        &DrawOp::AddItem {
            text: "n1 = note(\"X\", sheet: s1, at: (40, 30))".into(),
        },
    )
    .unwrap();
    let merged = block::merge(&base, &mine, &disk).unwrap();
    assert!(
        merged.contains("shared r = 11") && merged.contains("n1 = note(\"X\""),
        "{merged}"
    );
    // the disk changed outside the blocks too, or both changed the block: no merge
    assert!(block::merge(&base, &mine, &disk.replace("shared h = 12", "shared h = 13")).is_none());
    let mine2 = base.replace(
        "m = part()",
        "m = part()\n  n9 = note(\"Y\", sheet: s1, at: (1, 1))",
    );
    assert!(block::merge(&base, &mine2, &disk).is_none());
    // nothing to merge: mine as it is
    assert_eq!(block::merge(&base, &mine, &base).as_deref(), Some(mine.as_str()));
}

/// An op whose result would not read is refused in a block as in a drawing file: deleting the
/// sheet the views are on (`part()` reading as this document does not hide it).
#[test]
fn a_delete_a_later_item_reads_is_refused_in_a_block() {
    let src = knob();
    let e = block::apply_checked(&src, "dwg_knob", &DrawOp::DeleteItem { item: "s1".into() }).unwrap_err();
    assert!(e.text().contains("no `s1`"), "{}", e.text());
    let e = block::apply_checked(&src, "dwg_knob", &DrawOp::DeleteItem { item: "m".into() }).unwrap_err();
    assert!(e.text().contains("no `m`"), "{}", e.text());
    // a file's drawing refuses the same
    let file = format!(
        "drawing\n{}",
        block::body(&src, "dwg_knob")
            .unwrap()
            .replace("part()", "part(\"knob.fg\")")
    );
    assert!(
        forge_lang::drawing::ops::apply_checked(&file, &DrawOp::DeleteItem { item: "s1".into() }).is_err()
    );
}

/// A delete takes the statement's whole lines and nothing else: every other line as it was.
#[test]
fn a_delete_in_a_block_leaves_the_other_lines_as_written() {
    let src = knob();
    let out = block::apply_checked(&src, "dwg_knob", &DrawOp::DeleteItem { item: "side".into() }).unwrap();
    assert_eq!(out, src.replace("  side = projected(top, at: (200, 150))\n", ""));
    let two = src.replace(
        "  side = projected",
        "  n1 = note(\"A\", sheet: s1, at: (1, 1))\n  side = projected",
    );
    let out = block::apply_checked(&two, "dwg_knob", &DrawOp::DeleteItem { item: "n1".into() }).unwrap();
    assert_eq!(out, src);
    assert!(out.contains("\n  side = projected") && !out.contains("    side"));
}
