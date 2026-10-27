#[path="../tests/support/admission_source.rs"]
mod source;
fn main(){let root=std::path::PathBuf::from(std::env::args().nth(1).expect("fresh output directory"));assert!(root.is_absolute()&&root.starts_with("/workspace")&&!root.exists());std::fs::create_dir_all(&root).unwrap();println!("{}",source::prepare(&root,source::XML.as_bytes()).display());}
