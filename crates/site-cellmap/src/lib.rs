//! The one function of `site.cellmap`: fold each node's cell.install `List` into rows per cell system of
//! repo:ce-net-net `CE_NET`, and show a caller only the systems its chain's `where:system=` caveats allow.
#![forbid(unsafe_code)]

use ce_net_net::{Network, CE_NET};
use cr_sources::cell_install as ci;
use site_cellmap_api::*;
use std::collections::BTreeMap;
use ui_chain::Caveats;

pub const CELL: &str = "site.cellmap";
/// The one grant this cell's own key holds: listing what each node's cell.install placed, nothing else.
pub const GRANT: &[&str] = &["cell.install:list", "delegate:1"];
pub const EVERY_S: u64 = 10;
pub const PROBE_TIMEOUT_MS: u64 = 4000;

/// The rows of one system from what each node answered (`lists`: node name -> its List; a node that did not
/// answer is absent and counts as running nothing).
pub fn system_row(sys: &Network, lists: &BTreeMap<String, Vec<ci::Placed>>) -> SystemRow {
    let cells: Vec<CellRow> = sys
        .cells
        .iter()
        .map(|c| {
            let on: Vec<Instance> = lists
                .iter()
                .filter_map(|(node, l)| l.iter().find(|p| p.api == c.cell.name).map(|p| (node, p)))
                .map(|(node, p)| Instance { node: node.clone(), processes: p.processes, source: p.source.clone(), sha256: p.sha256.clone(), schemas: p.schemas.len() as u32 })
                .collect();
            let live: Vec<&Instance> = on.iter().filter(|i| i.processes > 0).collect();
            let shas: std::collections::BTreeSet<&str> = live.iter().map(|i| i.sha256.as_str()).filter(|s| !s.is_empty()).collect();
            CellRow { cell: c.cell.name.into(), major: c.cell.major, want: c.replicas, live: live.len() as u32, drift: shas.len() > 1, on }
        })
        .collect();
    let want = cells.iter().map(|c| c.want).sum();
    let live = cells.iter().map(|c| c.live.min(c.want)).sum();
    let ok = cells.iter().all(|c| c.live >= c.want);
    SystemRow { name: sys.name.into(), tab: sys.tab.map(|t| t.route.to_string()), cells, want, live, ok }
}

/// Every system of CE_NET (not CE_NET itself, which only includes them).
pub fn map(lists: &BTreeMap<String, Vec<ci::Placed>>, nodes: Vec<NodeRow>, as_of_ms: u64) -> CellMap {
    let systems = CE_NET.systems().into_iter().filter(|s| !s.cells.is_empty()).map(|s| system_row(s, lists)).collect();
    CellMap { systems, nodes, as_of_ms }
}

pub fn for_caveats(cv: &Caveats, m: &CellMap) -> CellMap {
    let keep = |s: &SystemRow| cv.0.iter().all(|(k, vals)| k == "system" && vals.iter().any(|v| v == &s.name));
    CellMap { systems: m.systems.iter().filter(|s| keep(s)).cloned().collect(), nodes: m.nodes.clone(), as_of_ms: m.as_of_ms }
}

pub fn for_caller(who: &ce_wire::Caller, m: &CellMap) -> CellMap {
    for_caveats(&Caveats::of(who, CELL), m)
}
