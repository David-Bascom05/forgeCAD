//! forge-cam: CAM on the exact part. The lowering of the B-rep to the certified checker's faces
//! (`lower`); setups, stock, operations and the checks build on it.
pub mod adaptive;
pub mod bore;
pub mod enclose;
pub mod engage;
pub mod fit;
pub mod fixture;
pub mod fixture_model;
pub mod hop;
pub mod leads;
pub mod life;
pub mod lower;
pub mod lower3d;
pub mod movememo;
pub mod ops;
pub mod section;
pub mod stock;
pub mod surf;
pub mod tabs;
