//! Minimal histories the adversarial hunt found breaking regeneration
//! (cargo run --release -p forge-model --example hunt -- 80 --emit-tests ...).
//! Ignored ones still fail; the rest are fixed and guard against coming back.

use forge_math::{Point2, Vec2};
use forge_model::adversarial::fr;
use forge_model::{Combine, Feature, Part};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};

fn regenerates_valid(part: Part) {
    let body = part.regenerate().expect("regenerates");
    let problems = forge_topo::check(&body);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn axis_revolve_inconsistent_classification() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Revolve {
                profile: Profile {
                    plane: fr([37.3, 10.3, 6.8], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, -3.6577774770126528),
                                b: Point2::new(4.3656147182352, -3.6577774770126528),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(4.3656147182352, -3.6577774770126528),
                                b: Point2::new(4.3656147182352, 0.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(4.3656147182352, 0.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(0.0, -3.6577774770126528),
                            },
                            3,
                        ),
                    ])],
                },
                axis: Axis2 {
                    origin: Point2::new(0.0, 0.0),
                    dir: Vec2::new(0.0, 1.0),
                },
                angle: 6.283185307179586,
                op: Combine::Add,
            },
            Feature::Revolve {
                profile: Profile {
                    plane: fr([39.4, 12.2, 8.8], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(3.4105136832344267, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(3.4105136832344267, 0.0),
                                b: Point2::new(0.0, -7.215102687640401),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, -7.215102687640401),
                                b: Point2::new(0.0, 0.0),
                            },
                            2,
                        ),
                    ])],
                },
                axis: Axis2 {
                    origin: Point2::new(0.0, 0.0),
                    dir: Vec2::new(0.0, 1.0),
                },
                angle: 6.283185307179586,
                op: Combine::Add,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Internal('face split crossed itself'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn ball_poles_face_split_crossed_itself() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Ball {
                frame: fr(
                    [26.724128641656122, 13.726026243631509, 10.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                radius: 4.397437810582629,
                op: Combine::Add,
            },
            Feature::Ball {
                frame: fr(
                    [26.724128641656122, 22.520901864796766, 10.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                radius: 4.397437810582629,
                op: Combine::Add,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Internal('inconsistent classification'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn ball_poles_inconsistent_classification() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Ball {
                frame: fr(
                    [17.72934714875436, 17.20846420377849, 10.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                radius: 4.885411415681668,
                op: Combine::Add,
            },
            Feature::Ball {
                frame: fr(
                    [17.72934714875436, 26.979287035141827, 10.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                radius: 4.885411415681668,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn coaxial_coplanar_faces_share_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([28.0, 17.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.5253599146953674,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 22.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([26.0, 19.0, 9.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 5.725907494383594,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 3.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([26.0, 19.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 2.862953747191797,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 22.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([22.0, 15.0, 9.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 4.101846793506851,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 8.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([19.0, 20.0, 9.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 4.4698939063050265,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 8.0,
                op: Combine::Add,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn coaxial_intersection_curve_runs_along_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([38.0, 23.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 2.6639636638569613,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 22.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([36.0, 17.0, 9.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 6.545672883430711,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 6.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([43.0, 22.0, 9.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 6.9201213772094565,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 4.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([43.0, 22.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.4600606886047283,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 22.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn coincident_walls_inconsistent_classification() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([8.3, 8.1, 7.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(5.380801697109082, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(5.380801697109082, 0.0),
                                b: Point2::new(5.380801697109082, 4.404231996225681),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(5.380801697109082, 4.404231996225681),
                                b: Point2::new(0.0, 4.404231996225681),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 4.404231996225681),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 4.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([5.4, 8.7, 7.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(5.278901177770495, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(5.278901177770495, 0.0),
                                b: Point2::new(5.278901177770495, 3.7457119221891952),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(5.278901177770495, 3.7457119221891952),
                                b: Point2::new(0.0, 3.7457119221891952),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 3.7457119221891952),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 4.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([8.1, 8.7, 5.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(5.278901177770495, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(5.278901177770495, 0.0),
                                b: Point2::new(5.278901177770495, 3.7457119221891952),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(5.278901177770495, 3.7457119221891952),
                                b: Point2::new(0.0, 3.7457119221891952),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 3.7457119221891952),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 6.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Unsupported('coplanar faces share an edge'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn coincident_walls_coplanar_faces_share_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([39.0, 12.0, 5.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(9.204734303677737, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.204734303677737, 0.0),
                                b: Point2::new(9.204734303677737, 5.114352035777794),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.204734303677737, 5.114352035777794),
                                b: Point2::new(0.0, 5.114352035777794),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 5.114352035777794),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 6.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([34.0, 12.0, 7.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(9.204734303677737, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.204734303677737, 0.0),
                                b: Point2::new(9.204734303677737, 5.114352035777794),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.204734303677737, 5.114352035777794),
                                b: Point2::new(0.0, 5.114352035777794),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 5.114352035777794),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 3.0,
                op: Combine::Add,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn coplanar_stack_coplanar_faces_share_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(80.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 0.0),
                                b: Point2::new(80.0, 60.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 60.0),
                                b: Point2::new(0.0, 60.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 60.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([7.0, 19.0, 6.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(12.118603353403007, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(12.118603353403007, 0.0),
                                b: Point2::new(12.118603353403007, 11.399533936320953),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(12.118603353403007, 11.399533936320953),
                                b: Point2::new(0.0, 11.399533936320953),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 11.399533936320953),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 5.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([9.0, 18.0, 6.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 2.362300895501718,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 5.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([11.0, 31.0, 6.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 2.230240262586193,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 5.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn coplanar_stack_intersection_curve_runs_along_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(80.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 0.0),
                                b: Point2::new(80.0, 60.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 60.0),
                                b: Point2::new(0.0, 60.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 60.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([19.0, 31.0, 6.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.775817614389506, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.775817614389506, 0.0),
                                b: Point2::new(10.775817614389506, 12.362631745738302),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.775817614389506, 12.362631745738302),
                                b: Point2::new(0.0, 12.362631745738302),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 12.362631745738302),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 5.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([30.0, 29.0, 6.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.0071167339505704,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 5.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([27.0, 25.0, 6.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(6.8208125944676885, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(6.8208125944676885, 0.0),
                                b: Point2::new(6.8208125944676885, 6.735844355853899),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(6.8208125944676885, 6.735844355853899),
                                b: Point2::new(0.0, 6.735844355853899),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 6.735844355853899),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 5.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn coplanar_stack_untouched_face_lies_on_the_other() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(80.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 0.0),
                                b: Point2::new(80.0, 60.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 60.0),
                                b: Point2::new(0.0, 60.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 60.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([34.0, 34.0, 10.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 4.264022875916782,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 5.0,
                op: Combine::Add,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Unsupported('intersection curve runs along an edge'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn exact_patterns_intersection_curve_runs_along_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(80.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 0.0),
                                b: Point2::new(80.0, 60.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 60.0),
                                b: Point2::new(0.0, 60.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 60.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [6.200775058431333, 9.879458984861778, 7.0],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(4.729886562492163, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(4.729886562492163, 0.0),
                                b: Point2::new(4.729886562492163, 4.729886562492163),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(4.729886562492163, 4.729886562492163),
                                b: Point2::new(0.0, 4.729886562492163),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 4.729886562492163),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 4.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [8.565718339677414, 16.97428882860002, 7.0],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 2.3649432812460813,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 4.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn fuzz_face_split_crossed_itself() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(143.83983367484734, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(143.83983367484734, 0.0),
                                b: Point2::new(143.83983367484734, 196.682832930769),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(143.83983367484734, 196.682832930769),
                                b: Point2::new(0.0, 196.682832930769),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 196.682832930769),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 34.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [127.0, 75.0, -4.0],
                        [-0.23474927403342583, 0.19601531504090183, 0.952087587677833],
                        [0.564259330691903, 0.8250254795692799, -0.03073053511746113],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 7.886259031424085,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 26.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [110.0, 147.0, -3.0],
                        [0.32201973956247965, 0.39017533639194, 0.8625928901883837],
                        [-0.35399950347023273, 0.8946585911796064, -0.2725258827549241],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-11.742993316835202, -7.828662211223468),
                                b: Point2::new(11.742993316835202, -7.828662211223468),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(11.742993316835202, -7.828662211223468),
                                b: Point2::new(11.742993316835202, 7.672088966999),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(11.742993316835202, 7.672088966999),
                                b: Point2::new(-11.742993316835202, 7.672088966999),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-11.742993316835202, 7.672088966999),
                                b: Point2::new(-11.742993316835202, -7.828662211223468),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 27.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [112.0, 71.0, -10.0],
                        [0.0, 0.0, 1.0],
                        [0.834157961600844, 0.5515256069285677, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-6.954968064619511, -14.815973705163554),
                                b: Point2::new(6.954968064619511, -14.815973705163554),
                            },
                            0,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(6.954968064619511, -6.760689337613524),
                                radius: 8.05528436755003,
                                start: -1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(15.01025243216954, -6.760689337613524),
                                b: Point2::new(15.01025243216954, 6.760689337613524),
                            },
                            2,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(6.954968064619511, 6.760689337613524),
                                radius: 8.05528436755003,
                                start: 0.0,
                                sweep: 1.5707963267948966,
                            },
                            3,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(6.954968064619511, 14.815973705163554),
                                b: Point2::new(-6.954968064619511, 14.815973705163554),
                            },
                            4,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-6.954968064619511, 6.760689337613524),
                                radius: 8.05528436755003,
                                start: 1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            5,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-15.01025243216954, 6.760689337613524),
                                b: Point2::new(-15.01025243216954, -6.760689337613524),
                            },
                            6,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-6.954968064619511, -6.760689337613524),
                                radius: 8.05528436755003,
                                start: 3.141592653589793,
                                sweep: 1.5707963267948966,
                            },
                            7,
                        ),
                    ])],
                },
                distance: 20.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn fuzz_inconsistent_classification() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(195.45293409287765, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(195.45293409287765, 0.0),
                                b: Point2::new(195.45293409287765, 167.89744286934663),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(195.45293409287765, 167.89744286934663),
                                b: Point2::new(0.0, 167.89744286934663),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 167.89744286934663),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 19.8,
                op: Combine::New,
            },
            Feature::Ball {
                frame: fr([195.4, 144.9, 12.8], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
                radius: 14.1,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [186.5, 143.7, 8.1],
                        [0.0, 0.0, 1.0],
                        [-0.6906495681319675, 0.7231895837462864, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 14.568206970399162,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 8.9,
                op: Combine::Add,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: invalid: VertexOffEdge"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn fuzz_invalid_vertexoffedge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(73.74689306008864, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(73.74689306008864, 0.0),
                                b: Point2::new(73.74689306008864, 87.41562212432996),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(73.74689306008864, 87.41562212432996),
                                b: Point2::new(0.0, 87.41562212432996),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 87.41562212432996),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 13.076,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [4.704, 17.363, 7.033],
                        [-0.17426376786987874, 0.05025670504538412, 0.9834156815943977],
                        [-0.29758245860190424, 0.9493123180781784, -0.10124624969588458],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-2.6205732693849053, -4.688487928764906),
                                b: Point2::new(2.6205732693849053, -4.688487928764906),
                            },
                            0,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(2.6205732693849053, -2.401848844873362),
                                radius: 2.286639083891544,
                                start: -1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(4.907212353276449, -2.401848844873362),
                                b: Point2::new(4.907212353276449, 2.401848844873362),
                            },
                            2,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(2.6205732693849053, 2.401848844873362),
                                radius: 2.286639083891544,
                                start: 0.0,
                                sweep: 1.5707963267948966,
                            },
                            3,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(2.6205732693849053, 4.688487928764906),
                                b: Point2::new(-2.6205732693849053, 4.688487928764906),
                            },
                            4,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-2.6205732693849053, 2.401848844873362),
                                radius: 2.286639083891544,
                                start: 1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            5,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-4.907212353276449, 2.401848844873362),
                                b: Point2::new(-4.907212353276449, -2.401848844873362),
                            },
                            6,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-2.6205732693849053, -2.401848844873362),
                                radius: 2.286639083891544,
                                start: 3.141592653589793,
                                sweep: 1.5707963267948966,
                            },
                            7,
                        ),
                    ])],
                },
                distance: 9.718,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [5.071, 8.397, 8.818],
                        [0.0, 0.0, 1.0],
                        [-0.991794043841121, 0.12784590177739935, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 4.424934251350758,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 4.074,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Internal('face split crossed itself'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn near_tolerance_face_split_crossed_itself() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [20.829744830901628, 31.98774163479715, 7.00001],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.0,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 3.00001,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Unsupported('coplanar faces share an edge'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn near_tolerance_coplanar_faces_share_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([43.0, 17.0, 10.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 3.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([50.0, 22.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.000005, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.000005, 0.0),
                                b: Point2::new(10.000005, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.000005, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([50.0, 14.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.00002, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.00002, 0.0),
                                b: Point2::new(10.00002, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.00002, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Unsupported('two vertices of one body meet at a cut'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn near_tolerance_two_vertices_of_one_body_meet() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([50.0, 24.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(9.9999, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.9999, 0.0),
                                b: Point2::new(9.9999, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.9999, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([50.0, 21.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.000001, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.000001, 0.0),
                                b: Point2::new(10.000001, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.000001, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([50.0, 18.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(9.99999, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.99999, 0.0),
                                b: Point2::new(9.99999, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(9.99999, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn repeated_coplanar_faces_share_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Ball {
                frame: fr([40.0, 12.0, 10.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
                radius: 3.0,
                op: Combine::Cut,
            },
            Feature::Ball {
                frame: fr([40.0, 12.0, 10.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
                radius: 3.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn rounded_everywhere_invalid_vertexoffedge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(6.0, 0.0),
                                b: Point2::new(74.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(74.0, 6.0),
                                radius: 6.0,
                                start: -1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(80.0, 6.0),
                                b: Point2::new(80.0, 54.0),
                            },
                            2,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(74.0, 54.0),
                                radius: 6.0,
                                start: 0.0,
                                sweep: 1.5707963267948966,
                            },
                            3,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(74.0, 60.0),
                                b: Point2::new(6.0, 60.0),
                            },
                            4,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(6.0, 54.0),
                                radius: 6.0,
                                start: 1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            5,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 54.0),
                                b: Point2::new(0.0, 6.0),
                            },
                            6,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(6.0, 6.0),
                                radius: 6.0,
                                start: 3.141592653589793,
                                sweep: 1.5707963267948966,
                            },
                            7,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [16.27, 14.22, 9.1],
                        [0.0, 0.0, 1.0],
                        [0.8133573086819977, 0.5817644612844424, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-1.9793862756892011, -3.021679170099138),
                                b: Point2::new(1.9793862756892011, -3.021679170099138),
                            },
                            0,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(1.9793862756892011, -0.7189941803451347),
                                radius: 2.3026849897540034,
                                start: -1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(4.2820712654432045, -0.7189941803451347),
                                b: Point2::new(4.2820712654432045, 0.7189941803451347),
                            },
                            2,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(1.9793862756892011, 0.7189941803451347),
                                radius: 2.3026849897540034,
                                start: 0.0,
                                sweep: 1.5707963267948966,
                            },
                            3,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(1.9793862756892011, 3.021679170099138),
                                b: Point2::new(-1.9793862756892011, 3.021679170099138),
                            },
                            4,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-1.9793862756892011, 0.7189941803451347),
                                radius: 2.3026849897540034,
                                start: 1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            5,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-4.2820712654432045, 0.7189941803451347),
                                b: Point2::new(-4.2820712654432045, -0.7189941803451347),
                            },
                            6,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-1.9793862756892011, -0.7189941803451347),
                                radius: 2.3026849897540034,
                                start: 3.141592653589793,
                                sweep: 1.5707963267948966,
                            },
                            7,
                        ),
                    ])],
                },
                distance: 5.38,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [18.33, 8.55, 2.86],
                        [0.0, 0.0, 1.0],
                        [-0.19671518758724155, 0.9804606748730499, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-4.303830462131129, -5.2909724998579115),
                                b: Point2::new(4.303830462131129, -5.2909724998579115),
                            },
                            0,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(4.303830462131129, -2.3692721716589715),
                                radius: 2.92170032819894,
                                start: -1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(7.225530790330069, -2.3692721716589715),
                                b: Point2::new(7.225530790330069, 2.3692721716589715),
                            },
                            2,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(4.303830462131129, 2.3692721716589715),
                                radius: 2.92170032819894,
                                start: 0.0,
                                sweep: 1.5707963267948966,
                            },
                            3,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(4.303830462131129, 5.2909724998579115),
                                b: Point2::new(-4.303830462131129, 5.2909724998579115),
                            },
                            4,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-4.303830462131129, 2.3692721716589715),
                                radius: 2.92170032819894,
                                start: 1.5707963267948966,
                                sweep: 1.5707963267948966,
                            },
                            5,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-7.225530790330069, 2.3692721716589715),
                                b: Point2::new(-7.225530790330069, -2.3692721716589715),
                            },
                            6,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::new(-4.303830462131129, -2.3692721716589715),
                                radius: 2.92170032819894,
                                start: 3.141592653589793,
                                sweep: 1.5707963267948966,
                            },
                            7,
                        ),
                    ])],
                },
                distance: 14.76,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: error: Boolean(Internal('face split crossed itself'))"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn slivers_face_split_crossed_itself() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [21.360463538630533, 18.096888868845262, 7.952475190783721],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 2.0237624046081395,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [31.148932184141152, 12.611746948708099, 7.952622593363382],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 2.023688703318309,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[ignore = "known failure: invalid: EdgeUses"]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn slivers_invalid_edgeuses() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([49.943, 24.343, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([50.052, 19.985, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([49.94, 20.418, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 8.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 8.0),
                                b: Point2::new(0.0, 8.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 8.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn tangent_intersection_curve_runs_along_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [55.25565938235356, 31.998550727257975, -1.0],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 4.744340617646442,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn tangent_invalid_edgeuses() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [63.104726564431104, 20.503103401826998, -1.0],
                        [0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.104726564431106,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 12.0,
                op: Combine::Add,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn through_vertex_inconsistent_classification() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([50.0, 0.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 2.3621728979030614,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn through_vertex_coplanar_faces_share_an_edge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([-5.0, -5.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 10.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 10.0),
                                b: Point2::new(0.0, 10.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 10.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr([-5.0, -5.0, -1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 0.0),
                                b: Point2::new(10.0, 10.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(10.0, 10.0),
                                b: Point2::new(0.0, 10.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 10.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn tilted_face_split_crossed_itself() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [7.0, 20.0, 6.0],
                        [-0.6064147293195341, 0.08036245040807177, 0.7910771470777841],
                        [-0.40086594032153844, 0.8283024783826776, -0.39143518262318056],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.086276136684638,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 10.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [54.0, 33.0, 5.0],
                        [-0.3936714742650959, -0.01919630869861348, 0.9190507451073054],
                        [-0.21469102572379714, 0.9740527659961667, -0.07161684528713072],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-3.0, -2.0),
                                b: Point2::new(3.0, -2.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(3.0, -2.0),
                                b: Point2::new(3.0, 2.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(3.0, 2.0),
                                b: Point2::new(-3.0, 2.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-3.0, 2.0),
                                b: Point2::new(-3.0, -2.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 13.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [7.0, 17.0, 9.0],
                        [-0.8499600445477712, -0.30542882453118786, 0.4292798106338658],
                        [-0.31723851258499947, 0.9472377672310904, 0.045829449744851314],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-3.0, -2.0),
                                b: Point2::new(3.0, -2.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(3.0, -2.0),
                                b: Point2::new(3.0, 2.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(3.0, 2.0),
                                b: Point2::new(-3.0, 2.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-3.0, 2.0),
                                b: Point2::new(-3.0, -2.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 8.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn tilted_inconsistent_classification() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [40.0, 19.0, 7.0],
                        [0.5666524769710187, 0.0142852073528501, 0.8238330554144421],
                        [0.11993365200805907, 0.9877709963410995, -0.09962117196320025],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.901222098907976,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 9.0,
                op: Combine::Cut,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [45.0, 16.0, 6.0],
                        [0.5172325767038999, 0.7653554581187136, 0.38301629772653034],
                        [-0.7977152146549948, 0.5932483757196286, -0.10819797139483706],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 3.7108375921356487,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 12.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
fn tilted_invalid_vertexoffedge() {
    regenerates_valid(Part {
        features: vec![
            Feature::Extrude {
                profile: Profile {
                    plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 0.0),
                                b: Point2::new(60.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 0.0),
                                b: Point2::new(60.0, 40.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(60.0, 40.0),
                                b: Point2::new(0.0, 40.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 40.0),
                                b: Point2::new(0.0, 0.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [11.0, 32.0, 5.0],
                        [0.31908210132340264, 0.030676446866199327, 0.947230472600363],
                        [-0.14251678482619148, 0.9896637471410057, 0.015957244048670636],
                    ),
                    loops: vec![ProfileLoop::new(vec![(
                        Segment::Circle {
                            center: Point2::new(0.0, 0.0),
                            radius: 4.866330339970743,
                            ccw: true,
                        },
                        0,
                    )])],
                },
                distance: 13.0,
                op: Combine::Add,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: fr(
                        [8.0, 30.0, 10.0],
                        [-0.1504179657209733, 0.4172628145333923, 0.8962511808617247],
                        [-0.16633750753798293, 0.8829600268456804, -0.43899137187275894],
                    ),
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::new(-3.0, -2.0),
                                b: Point2::new(3.0, -2.0),
                            },
                            0,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(3.0, -2.0),
                                b: Point2::new(3.0, 2.0),
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(3.0, 2.0),
                                b: Point2::new(-3.0, 2.0),
                            },
                            2,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(-3.0, 2.0),
                                b: Point2::new(-3.0, -2.0),
                            },
                            3,
                        ),
                    ])],
                },
                distance: 10.0,
                op: Combine::Cut,
            },
        ],
    });
}

#[test]
fn self_intersecting_profile_is_rejected() {
    // a corner radius over half the short side turns the outline inside out
    let lp = Profile::rounded_rectangle(Point2::new(-3.0, -5.0), 6.0, 10.0, 4.0, 0);
    let profile = Profile {
        plane: fr([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        loops: vec![lp],
    };
    assert_eq!(
        profile.normalized().err(),
        Some(forge_ops::ProfileError::SelfIntersecting { loop_index: 0 })
    );
}
