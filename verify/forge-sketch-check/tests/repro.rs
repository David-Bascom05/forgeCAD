use forge_math::{Frame, Point2};
use forge_sketch::{Constraint, Sketch, Status};

/// A group `solve_after` takes from `prev` adds nothing to the residual it reports.
#[test]
fn reused_unsolved_group_reports_solved() {
    let make = || {
        let mut s = Sketch::new(Frame::WORLD);
        let a = s.add_point(Point2::new(0.0, 0.0));
        let b = s.add_point(Point2::new(10.0, 0.0));
        // two distances between the same points: no configuration meets both
        s.add_constraint(Constraint::Distance(a, b, 10.0)).unwrap();
        s.add_constraint(Constraint::Distance(a, b, 20.0)).unwrap();
        s
    };
    let mut prev = make();
    let r = prev.solve();
    eprintln!("first solve: {r:?}");
    assert_eq!(r.status, Status::NotConverged);
    let mut again = make();
    let r2 = again.solve_after(Some(&prev));
    eprintln!("again, from prev: {r2:?}");
    let (_, conflicting) = again.freedom();
    eprintln!("conflicting after: {conflicting:?}");
}
