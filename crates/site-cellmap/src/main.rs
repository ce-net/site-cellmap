//! `site-cellmap serve`: the site.cellmap cell behind this machine's door. Every EVERY_S seconds it asks every mesh
//! node's cell.install for its List (the cell's own key, `cell.install:list`) and folds the answers into the map;
//! `Map` answers from it at once. `site-cellmap probe` prints one round.
use ce_wire::{Server, WireError};
use cr_sources::cell_install::{self as ci, CellInstall};
use cr_sources::nodes::{mesh_nodes, Node};
use site_cellmap::*;
use site_cellmap_api::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// The proof for one node's cell.install: a grant that node issued and ops attenuated to this cell's key
/// (`<cell key dir>/peers/<node id>.hex`, exactly `cell.install:list`), else this node's grant attenuated. A node that
/// roots only its own chains refuses the second; the map then says so for that node.
fn token(node_id: &str) -> Result<Vec<u8>, String> {
    let f = ui_chain::cell_key::dir(CELL).join("peers").join(format!("{node_id}.hex"));
    match std::fs::read_to_string(&f) {
        Ok(hex) => {
            let k = ui_chain::cell_key()?;
            let chain = ce_gate::chain_from_hex(hex.trim()).map_err(|e| format!("{}: {e}", f.display()))?;
            Ok(ce_gate::proof::<CellInstall>(&k.id, &chain))
        }
        Err(_) => ui_chain::source_token::<CellInstall>(&["cell.install:list"], 60, None),
    }
}

async fn list(n: &Node) -> (NodeRow, Option<Vec<ci::Placed>>) {
    let mut why = String::from("no address");
    let checked_ms = ui_chain::now_ms();
    for a in &n.addrs {
        let t0 = Instant::now();
        let tok = match token(&n.id) {
            Ok(t) => t,
            Err(e) => return (NodeRow { name: n.name.clone(), id: n.id.clone(), up: false, rtt_ms: None, checked_ms, why: e }, None),
        };
        let d = Duration::from_millis(PROBE_TIMEOUT_MS);
        match tokio::time::timeout(d, ce_wire::Client::<CellInstall>::connect_opts(format!("{a}:7700"), tok, false, d)).await {
            Ok(Ok(mut c)) => {
                let rtt = t0.elapsed().as_millis() as u32;
                c.deadline = Duration::from_secs(5);
                let l = c.call::<ci::List>(&()).await;
                let why = l.as_ref().err().map(|e| e.to_string()).unwrap_or_default();
                return (NodeRow { name: n.name.clone(), id: n.id.clone(), up: true, rtt_ms: Some(rtt), checked_ms, why }, l.ok());
            }
            Ok(Err(WireError::Refused(r))) => {
                return (NodeRow { name: n.name.clone(), id: n.id.clone(), up: true, rtt_ms: Some(t0.elapsed().as_millis() as u32), checked_ms, why: format!("refused: {r:?}") }, None)
            }
            Ok(Err(e)) => why = format!("{a}: {e}"),
            Err(_) => why = format!("{a}: no answer in {PROBE_TIMEOUT_MS} ms"),
        }
    }
    (NodeRow { name: n.name.clone(), id: n.id.clone(), up: false, rtt_ms: None, checked_ms, why }, None)
}

async fn round(st: &Arc<RwLock<CellMap>>) {
    let ns = mesh_nodes();
    let hs: Vec<_> = ns.into_iter().map(|n| tokio::spawn(async move { list(&n).await })).collect();
    let mut rows = Vec::new();
    let mut lists = BTreeMap::new();
    for h in hs {
        if let Ok((r, l)) = h.await {
            if let Some(l) = l {
                lists.insert(r.name.clone(), l);
            }
            rows.push(r);
        }
    }
    *st.write().await = map(&lists, rows, ui_chain::now_ms());
}

async fn serve() -> Result<(), String> {
    let me = ui_chain::init(CELL)?.root();
    let started = Instant::now();
    let st: Arc<RwLock<CellMap>> = Arc::new(RwLock::new(CellMap { systems: vec![], nodes: vec![], as_of_ms: 0 }));
    let bg = st.clone();
    tokio::spawn(async move {
        loop {
            round(&bg).await;
            tokio::time::sleep(Duration::from_secs(EVERY_S)).await;
        }
    });
    let s1 = st.clone();
    let gate = ce_gate::Gate::<SiteCellmap>::new(me, ce_gate::mesh_roots()).into_fn();
    Server::<SiteCellmap>::new(gate)
        .on::<HealthOp, _, _>(move |_: (), _| async move { Ok(Health { ok: true, version: env!("CARGO_PKG_VERSION").into(), uptime_s: started.elapsed().as_secs() }) })
        .on::<Map, _, _>(move |_: (), who| {
            let st = s1.clone();
            async move { Ok(for_caller(&who, &*st.read().await)) }
        })
        .audit(ce_gate::audit::file())
        .serve_door()
        .await
        .map_err(|e| e.to_string())
}

fn main() {
    let rt = || tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string());
    let r = match std::env::args().nth(1).as_deref() {
        Some("serve") => rt().and_then(|rt| rt.block_on(serve())),
        Some("probe") => rt().and_then(|rt| {
            rt.block_on(async {
                ui_chain::init(CELL)?;
                let st = Arc::new(RwLock::new(CellMap { systems: vec![], nodes: vec![], as_of_ms: 0 }));
                round(&st).await;
                let m = st.read().await;
                for n in &m.nodes {
                    println!("node {:<16} {} {:?} {}", n.name, if n.up { "up" } else { "DOWN" }, n.rtt_ms, n.why.chars().take(80).collect::<String>());
                }
                for s in &m.systems {
                    println!("{} {}/{} {}", s.name, s.live, s.want, if s.ok { "ok" } else { "UNDER" });
                    for c in &s.cells {
                        let on: Vec<String> = c.on.iter().map(|i| format!("{}x{} {}:{}", i.node, i.processes, i.source, i.sha256.chars().take(10).collect::<String>())).collect();
                        println!("  {:<18} {}/{}{} {}", c.cell, c.live, c.want, if c.drift { " DRIFT" } else { "" }, on.join(", "));
                    }
                }
                Ok(())
            })
        }),
        _ => Err("usage: site-cellmap serve | probe".into()),
    };
    if let Err(e) = r {
        eprintln!("site-cellmap: {e}");
        std::process::exit(1);
    }
}
