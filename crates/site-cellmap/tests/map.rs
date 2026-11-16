use cr_sources::cell_install as ci;
use site_cellmap::*;
use std::collections::BTreeMap;
use ui_chain::Caveats;

fn placed(api: &str, n: u32, sha: &str) -> ci::Placed {
    ci::Placed { api: api.into(), processes: n, schemas: vec![1], source: "repo".into(), sha256: sha.into(), path: String::new(), at: 0 }
}

#[test]
fn wanted_against_running_per_system_with_drift() {
    let mut l = BTreeMap::new();
    l.insert("foreman-ops-1".to_string(), vec![placed("approve.inbox", 1, "aa"), placed("grant.bundle", 1, "g1")]);
    l.insert("leif-desk".to_string(), vec![placed("approve.inbox", 1, "bb"), placed("grant.bundle", 0, "g1")]);
    let m = map(&l, vec![], 1);
    let a = m.systems.iter().find(|s| s.name == "APPROVALS").unwrap();
    assert_eq!(a.tab.as_deref(), Some("/approvals/"));
    let inbox = a.cells.iter().find(|c| c.cell == "approve.inbox").unwrap();
    assert_eq!((inbox.live, inbox.want, inbox.drift), (2, 2, true));
    let gb = a.cells.iter().find(|c| c.cell == "grant.bundle").unwrap();
    assert_eq!((gb.live, gb.drift), (1, false), "a stopped replica is not live");
    assert!(!a.ok);
    assert_eq!(m.systems.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["CORE", "APPROVALS", "CONTROL_ROOM", "FOREMAN"]);
}

#[test]
fn a_system_caveat_shows_only_those_systems() {
    let m = map(&BTreeMap::new(), vec![], 1);
    let cv = Caveats([("system".to_string(), vec!["APPROVALS".to_string()])].into_iter().collect());
    assert_eq!(for_caveats(&cv, &m).systems.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["APPROVALS"]);
    let bad = Caveats([("colour".to_string(), vec!["x".to_string()])].into_iter().collect());
    assert!(for_caveats(&bad, &m).systems.is_empty());
    assert_eq!(for_caveats(&Caveats::default(), &m).systems.len(), 4);
}
