//! `site.cellmap@1`: the one observability view of ce-net (repo:ce/docs/SITES-ON-CELLS.md section 8). Every cell of
//! every cell system in repo:ce-net-net `CE_NET`, on every serving node: how many processes its door registered, how
//! cell.install placed it (source, commit or sha256), and whether the nodes run the same thing.
//!
//! Rights: `site.cellmap:read`. Row level: `site.cellmap:where:system=<NAME>` caveats (a collaborator sees only
//! their systems); an unknown key hides every system.
#![forbid(unsafe_code)]

ce_wire::api! {
    api SiteCellmap = "site.cellmap", version 1;
    types {
        /// Health contract: these three fields, in this order, in every cell.
        pub struct Health { pub ok: bool, pub version: String, pub uptime_s: u64 }
        /// One node's door: did its cell.install answer the last probe, how fast, and why not.
        pub struct NodeRow { pub name: String, pub id: String, pub up: bool, pub rtt_ms: Option<u32>, pub checked_ms: u64, pub why: String }
        /// A cell on one node. processes: registered at that node's door now. source: "repo", "commit", "artifact",
        /// "release" or "door"; sha256: the commit or artifact hash cell.install recorded.
        pub struct Instance { pub node: String, pub processes: u32, pub source: String, pub sha256: String, pub schemas: u32 }
        /// want: replicas the system description asks for; live: nodes where it has a process now. drift: the live
        /// nodes run different commits or artifacts.
        pub struct CellRow { pub cell: String, pub major: u32, pub want: u32, pub live: u32, pub drift: bool, pub on: Vec<Instance> }
        /// tab: the route the system offers, if any. ok: every cell has its wanted replicas live.
        pub struct SystemRow { pub name: String, pub tab: Option<String>, pub cells: Vec<CellRow>, pub want: u32, pub live: u32, pub ok: bool }
        pub struct CellMap { pub systems: Vec<SystemRow>, pub nodes: Vec<NodeRow>, pub as_of_ms: u64 }
    }
    ops {
        1 => HealthOp(()) -> Health, needs "site.cellmap:health";
        2 => Map(()) -> CellMap, needs "site.cellmap:read", deadline_ms 15000;
    }
}
