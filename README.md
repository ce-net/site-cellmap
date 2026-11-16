# site-cellmap

`site.cellmap@1` (repo:ce/docs/SITES-ON-CELLS.md section 8): every cell of every cell system in repo:ce-net-net
`CE_NET`, on every mesh node: processes registered at that node's door, how cell.install placed it (source, commit),
wanted replicas against live ones, drift between nodes, and each node's door. The Core tab of apps.ce-net.com.

Rights `site.cellmap:read`; `site.cellmap:where:system=<NAME>` caveats cut it to some systems. Its key
(`<ce data dir>/cell-keys/site.cellmap/`, `cell.install:list` + `delegate:1`) asks this node's cell.install; for a node
that roots only its own chains, a grant that node issued, attenuated to exactly `cell.install:list` for this key, sits in
`peers/<node id>.hex`. `cargo test`: 2. `site-cellmap probe` prints one round. Apache-2.0.
