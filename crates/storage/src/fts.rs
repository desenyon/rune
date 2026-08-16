use crate::error::Result;
use rune_core::{Node, NodeId};
use rusqlite::{params, Connection};

pub fn upsert(conn: &Connection, node: &Node) -> Result<()> {
    replace(conn, node)
}

pub fn delete(conn: &Connection, id: NodeId) -> Result<()> {
    conn.execute("DELETE FROM nodes_fts WHERE id = ?1", [id.to_string()])?;
    Ok(())
}

pub fn replace(conn: &Connection, node: &Node) -> Result<()> {
    delete(conn, node.id)?;
    conn.execute(
        "INSERT INTO nodes_fts (id, kind, name, body) VALUES (?1, ?2, ?3, ?4)",
        params![
            node.id.to_string(),
            node.kind.as_str(),
            node.name.clone().unwrap_or_default(),
            node.search_body(),
        ],
    )?;
    Ok(())
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<(String, String, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, bm25(nodes_fts) AS rank
         FROM nodes_fts
         WHERE nodes_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![query, limit as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
