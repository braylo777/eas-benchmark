//! Reader for the v0.1 probe per-item CSVs (`results/probe_*_items.csv`, existing data).

/// One row: the framing, and whether the surface answer, the probe and the mismatch flag are set.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: usize,
    pub family: String,
    pub framing: String,
    pub surface_correct: bool,
    pub probe_correct: bool,
    pub mismatch: bool,
}

const HEADER: &str = "id,family,framing,statement,truth,user_claim,surface_answer,surface_logit_diff,probe_p_true,probe_answer,surface_correct,probe_correct,mismatch";

fn bit(s: &str) -> Result<bool, String> {
    match s.trim() {
        "1" => Ok(true),
        "0" => Ok(false),
        o => Err(format!("expected 0/1, got {o:?}")),
    }
}

/// Parse the CSV text. Statements in this set contain no commas or quotes, so a plain split is exact;
/// a row with the wrong column count is an error.
pub fn parse(text: &str) -> Result<Vec<Item>, String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some(HEADER) {
        return Err("unexpected header".into());
    }
    let mut out = Vec::new();
    for (n, l) in lines.enumerate() {
        if l.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = l.split(',').collect();
        if c.len() != 13 {
            return Err(format!("row {}: {} columns", n + 2, c.len()));
        }
        out.push(Item {
            id: c[0].parse().map_err(|_| format!("row {}: bad id", n + 2))?,
            family: c[1].into(),
            framing: c[2].into(),
            surface_correct: bit(c[10])?,
            probe_correct: bit(c[11])?,
            mismatch: bit(c[12])?,
        });
    }
    Ok(out)
}
