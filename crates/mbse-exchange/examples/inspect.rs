//! Local qualification receipt without copying private model content into Git.
use std::collections::BTreeMap;
fn main() {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("usage: cargo run -p mbse-exchange --example inspect -- INPUT");
        std::process::exit(2);
    };
    match mbse_exchange::open_external(std::path::Path::new(&path)) {
        Ok(doc) => {
            let mut kinds = BTreeMap::new();
            for node in &doc.nodes {
                *kinds.entry(format!("{:?}", node.kind)).or_insert(0usize) += 1;
            }
            println!("contract={} source_sha256={} original_bytes={} files={} nodes={} references={} resolved={} unresolved={}", doc.contract,doc.sha256,doc.original_bytes.len(),doc.files.len(),doc.nodes.len(),doc.references.len(),doc.references.iter().filter(|r|r.resolved).count(),doc.references.iter().filter(|r|!r.resolved).count());
            println!("node_kinds={kinds:?}");
            println!(
                "diagnostic_codes={:?}",
                doc.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
            );
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
