//! Bounded Kernspan Stage1 model reused with retained MIT provenance.
pub const PARSER_REV: &str = "821221767c3c56cb1ebe7da22666197a47c9c645";
pub mod boundary;
pub mod local_input;
pub mod stage1;
pub mod stage1_calc;
pub mod stage1_graph;
pub mod stage1_io;
pub mod stage1_sysml;

pub mod recovery;

pub mod native_views;
pub mod native_views_io;

pub mod native_views_inspect;

pub mod native_views_plan;

pub mod native_navigation;

pub mod native_search;
