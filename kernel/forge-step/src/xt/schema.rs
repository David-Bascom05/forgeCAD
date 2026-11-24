//! Field layouts of the nodes. A file written since Parasolid V14 carries its own layout as edits
//! of the V13 base schema SCH_13006 (XT Format Reference 2.1.2); the base is not printed in the
//! reference, so `BASE` is rebuilt here from the reference's structures and checked against the
//! edits of every file of the corpus (`research/parasolid`): each edit list copies or deletes
//! exactly the base's fields, and every file reads to its terminator.

/// A field's type (XT Format Reference 2.1.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// u: a byte.
    Byte,
    /// c: a character.
    Char,
    /// l: a logical.
    Logical,
    /// n: a short; w: a unicode character as a short.
    Short,
    /// d: an int; t: a tag.
    Int,
    /// p: a pointer index.
    Ptr,
    /// f: a double.
    Real,
    /// i: an interval, two doubles.
    Interval,
    /// v: a vector, three doubles; h: an hvec, of which only the position is written.
    Vector,
    /// b: a box, six doubles.
    Box,
}

impl Kind {
    pub(crate) fn of(code: &[u8]) -> Option<Kind> {
        Some(match code {
            b"u" => Kind::Byte,
            b"c" => Kind::Char,
            b"l" => Kind::Logical,
            b"n" | b"w" => Kind::Short,
            b"d" | b"t" => Kind::Int,
            b"p" => Kind::Ptr,
            b"f" => Kind::Real,
            b"i" => Kind::Interval,
            b"v" | b"h" => Kind::Vector,
            b"b" => Kind::Box,
            _ => return None,
        })
    }

    /// Values one element takes in the node store.
    pub(crate) fn width(self) -> usize {
        match self {
            Kind::Interval => 2,
            Kind::Vector => 3,
            Kind::Box => 6,
            _ => 1,
        }
    }
}

/// One field: its name, type and element count (0 a scalar, 1 the variable-length field, more an
/// array).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Field {
    pub name: String,
    pub kind: Kind,
    pub n: u32,
}

/// A node type's fields as a file writes them, with each field's place in a node's values.
#[derive(Clone, Debug, Default)]
pub(crate) struct Layout {
    pub fields: Vec<Field>,
    /// Each field's first value; the variable field, if any, is last.
    pub offset: Vec<usize>,
    pub variable: bool,
}

impl Layout {
    pub(crate) fn new(fields: Vec<Field>) -> Option<Layout> {
        let mut offset = Vec::with_capacity(fields.len());
        let mut at = 0usize;
        let mut variable = false;
        for (i, f) in fields.iter().enumerate() {
            offset.push(at);
            if f.n == 1 {
                // only the last field may vary in length
                if i + 1 != fields.len() {
                    return None;
                }
                variable = true;
            } else {
                at += f.kind.width() * (f.n.max(1) as usize);
            }
        }
        let _ = at;
        Some(Layout {
            fields,
            offset,
            variable,
        })
    }

    pub(crate) fn find(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }
}

/// The V13 base schema SCH_13006, by node type: fields as `name:code`, an array as `code[n]`,
/// the variable field as `code[]`.
const G: &str = "node_id:d attributes:p owner:p next:p previous:p geometric_owner:p sense:c";

fn base_src(ty: u16) -> Option<String> {
    let s = |x: &str| Some(x.to_string());
    let g = |x: &str| Some(format!("{G} {x}"));
    match ty {
        10 => s(
            "highest_node_id:d attributes:p attribute_chains:p list:p surface:p curve:p point:p key:p \
             res_size:f res_linear:f ref_instance:p next:p previous:p state:u owner:p type:u sub_instance:p",
        ),
        11 => s(
            "node_id:d attributes:p type:u part:p transform:p assembly:p next_in_part:p prev_in_part:p \
             next_of_part:p prev_of_part:p",
        ),
        12 => s(
            "highest_node_id:d attributes:p attribute_chains:p surface:p curve:p point:p key:p res_size:f \
             res_linear:f ref_instance:p next:p previous:p state:u owner:p body_type:u nom_geom_state:u shell:p \
             boundary_surface:p boundary_curve:p boundary_point:p region:p edge:p vertex:p",
        ),
        13 => s("node_id:d attributes:p body:p next:p face:p edge:p vertex:p region:p front_face:p"),
        14 => s(
            "node_id:d attributes:p tolerance:f next:p previous:p loop:p shell:p surface:p sense:c \
             next_on_surface:p previous_on_surface:p next_front:p previous_front:p front_shell:p",
        ),
        15 => s("node_id:d attributes:p fin:p face:p next:p"),
        16 => s(
            "node_id:d attributes:p tolerance:f fin:p previous:p next:p curve:p next_on_curve:p \
             previous_on_curve:p owner:p",
        ),
        17 => {
            s("attributes:p loop:p forward:p backward:p vertex:p other:p edge:p curve:p next_at_vx:p sense:c")
        }
        18 => s("node_id:d attributes:p fin:p previous:p next:p point:p tolerance:f owner:p"),
        19 => s("node_id:d attributes:p body:p next:p previous:p shell:p type:c"),
        29 => s("node_id:d attributes:p owner:p next:p previous:p pvec:v"),
        30 => g("pvec:v direction:v"),
        31 => g("centre:v normal:v x_axis:v radius:f"),
        32 => g("centre:v normal:v x_axis:v major_radius:f minor_radius:f"),
        38 => g("surface:p[2] chart:p start:p end:p"),
        40 => s(
            "base_parameter:f base_scale:f chart_count:d chordal_error:f angular_error:f parameter_error:f[2] \
             hvec:h[]",
        ),
        41 => s("type:c hvec:h[]"),
        45 => s("vertices:f[]"),
        50 => g("pvec:v normal:v x_axis:v"),
        51 => g("pvec:v axis:v radius:f x_axis:v"),
        52 => g("pvec:v axis:v radius:f sin_half_angle:f cos_half_angle:f x_axis:v"),
        53 => g("centre:v radius:f axis:v x_axis:v"),
        54 => g("centre:v axis:v major_radius:f minor_radius:f x_axis:v"),
        56 => g("blend_type:c surface:p[2] spine:p range:f[2] thumb_weight:f[2] boundary:p[2] start:p end:p"),
        59 => g("boundary:n blend:p"),
        60 => g("check:c true_offset:l surface:p offset:f scale:f"),
        67 => g("section:p sweep:v scale:f"),
        68 => g("profile:p base:v axis:v start:v end:v start_param:f end_param:f x_axis:v scale:f"),
        70 => s(
            "node_id:d owner:p next:p previous:p list_type:d list_length:d block_length:d size_of_entry:d \
             list_block:p",
        ),
        74 => s("n_entries:d next_block:p entries:p[]"),
        79 | 102 => s("string:c[]"),
        80 => s("next:p identifier:p type_id:d actions:u[8] field_names:p legal_owners:l[14] fields:u[]"),
        81 => {
            s("node_id:d definition:p owner:p next:p previous:p next_of_type:p previous_of_type:p fields:p[]")
        }
        82 | 88 => s("values:d[]"),
        83 => s("values:f[]"),
        84 => s("values:c[]"),
        85..=87 | 89 => s("values:v[]"),
        90 => s("node_id:d attributes:p owner:p next:p previous:p type:u first_member:p"),
        91 => s("dummy_node_id:d owning_feature:p owner:p next:p previous:p next_member:p previous_member:p"),
        98 => s("values:w[]"),
        99 => s("names:p[]"),
        100 => s(
            "node_id:d owner:p next:p previous:p rotation_matrix:f[9] translation_vector:v scale:f flag:d \
             perspective_vector:v",
        ),
        101 => s(
            "assembly:p attribute:p body:p transform:p surface:p curve:p point:p alive:l attrib_def:p \
             highest_id:d current_id:d",
        ),
        120 | 130 => g("type:c data:p tf:p internal_geom:p[]"),
        121 => s("geom_type:d real_array:p int_array:p"),
        122 => s("key:p real_array:p int_array:p"),
        124 | 134 => g("nurbs:p data:p"),
        125 => s(
            "original_uint:i original_vint:i extended_uint:i extended_vint:i self_int:u original_u_start:c \
             original_u_end:c original_v_start:c original_v_end:c extended_u_start:c extended_u_end:c \
             extended_v_start:c extended_v_end:c analytic_form_type:c swept_form_type:c spun_form_type:c \
             blend_form_type:c analytic_form:p swept_form:p spun_form:p blend_form:p",
        ),
        126 => s(
            "u_periodic:l v_periodic:l u_degree:n v_degree:n n_u_vertices:d n_v_vertices:d u_knot_type:u \
             v_knot_type:u n_u_knots:d n_v_knots:d rational:l u_closed:l v_closed:l surface_form:u vertex_dim:n \
             bspline_vertices:p u_knot_mult:p v_knot_mult:p u_knots:p v_knots:p",
        ),
        127 => s("mult:n[]"),
        128 => s("knots:f[]"),
        133 => g("basis_curve:p point_1:v point_2:v parm_1:f parm_2:f"),
        135 => s("self_int:u analytic_form:p"),
        136 => s(
            "degree:n n_vertices:d vertex_dim:n n_knots:d knot_type:u periodic:l closed:l rational:l \
             curve_form:u bspline_vertices:p knot_mult:p knots:p",
        ),
        137 => g("surface:p b_curve:p original:p tolerance_to_original:f"),
        141 => s("owner:p next:p previous:p shared_geometry:p"),
        163 => s("axis_pt:v axis_dir:v hand:c turns:i pitch:f gap:f tol:f"),
        184 => s("axis_pt:v axis_dir:v point:v hand:c turns:i pitch:f tol:f"),
        _ => None,
    }
}

fn parse_fields(src: &str) -> Vec<Field> {
    src.split_whitespace()
        .filter_map(|f| {
            let (name, t) = f.split_once(':')?;
            let (code, n) = match t.split_once('[') {
                Some((c, rest)) => {
                    let k = rest.trim_end_matches(']');
                    (c, if k.is_empty() { 1 } else { k.parse().ok()? })
                }
                None => (t, 0),
            };
            Some(Field {
                name: name.to_string(),
                kind: Kind::of(code.as_bytes())?,
                n,
            })
        })
        .collect()
}

/// The base schema's fields of a node type, None for a type it lacks.
pub(crate) fn base(ty: u16) -> Option<Vec<Field>> {
    base_src(ty).map(|s| parse_fields(&s))
}

/// Node types visible at the PK interface: those that carry user fields (6.1).
pub(crate) fn visible(ty: u16) -> bool {
    matches!(
        ty,
        10..=19 | 29..=32 | 38 | 50..=54 | 56 | 60 | 67 | 68 | 70 | 80 | 81 | 90 | 100 | 120 | 124 | 130 | 133
            | 134 | 137 | 200 | 201 | 222
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_base_field_reads() {
        for ty in 0..300u16 {
            if let Some(src) = base_src(ty) {
                let f = base(ty).unwrap();
                assert_eq!(f.len(), src.split_whitespace().count(), "type {ty}");
                assert!(Layout::new(f).is_some(), "type {ty}");
            }
        }
    }

    #[test]
    fn a_layout_places_arrays_and_the_variable_field() {
        let l = Layout::new(base(40).unwrap()).unwrap();
        assert_eq!(l.offset[5], 5);
        assert_eq!(l.offset[6], 7);
        assert!(l.variable);
        let l = Layout::new(base(100).unwrap()).unwrap();
        assert_eq!(l.offset[8], 4 + 9 + 3 + 1 + 1);
        assert!(!l.variable);
    }
}
