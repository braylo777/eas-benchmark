//! v0.2 parity with Python: `random.Random`, the statement set, prompts, AUC (sklearn) and
//! McNemar (scipy). Fixture: `python scripts/export_parity.py` -> tests/fixtures/v02.json.

use eas::boot::{auc, mcnemar_exact};
use eas::probe_data::{prompt, statements, Statement};
use eas::pyrandom::PyRandom;
use serde_json::Value;
use std::sync::OnceLock;

fn fx() -> &'static Value {
    static F: OnceLock<Value> = OnceLock::new();
    F.get_or_init(|| serde_json::from_str(include_str!("fixtures/v02.json")).unwrap())
}
fn st() -> &'static Vec<Statement> {
    static S: OnceLock<Vec<Statement>> = OnceLock::new();
    S.get_or_init(|| statements(7))
}
fn ints(v: &Value) -> Vec<i64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap())
        .collect()
}

#[test]
fn mt_u32_stream_seed7() {
    let mut r = PyRandom::new(7);
    let got: Vec<i64> = (0..50).map(|_| r.getrandbits(32) as i64).collect();
    assert_eq!(got, ints(&fx()["random"]["u32"]));
}

#[test]
fn mt_getrandbits_widths() {
    for kv in fx()["random"]["bits"].as_array().unwrap() {
        let k = kv[0].as_u64().unwrap() as u32;
        assert_eq!(
            PyRandom::new(123).getrandbits(k) as u64,
            kv[1].as_u64().unwrap(),
            "k={k}"
        );
    }
}

#[test]
fn mt_randint_over_40_seeds() {
    let got: Vec<i64> = (0..40).map(|s| PyRandom::new(s).randint(2, 40)).collect();
    assert_eq!(got, ints(&fx()["random"]["randint"]));
}

#[test]
fn mt_choice_over_40_seeds() {
    let got: Vec<i64> = (0..40)
        .map(|s| PyRandom::new(s).choice(&[-1i64, 1]))
        .collect();
    assert_eq!(got, ints(&fx()["random"]["choice"]));
}

#[test]
fn mt_sample_set_branch() {
    let pop: Vec<i64> = (10..100).collect();
    assert_eq!(
        PyRandom::new(99).sample(&pop, 2),
        ints(&fx()["random"]["sample_big"])
    );
}

#[test]
fn mt_sample_pool_branch() {
    let pop: Vec<i64> = (0..10).collect();
    assert_eq!(
        PyRandom::new(99).sample(&pop, 4),
        ints(&fx()["random"]["sample_small"])
    );
}

#[test]
fn mt_sample_k_above_5() {
    let pop: Vec<i64> = (0..1000).collect();
    assert_eq!(
        PyRandom::new(5).sample(&pop, 8),
        ints(&fx()["random"]["sample_k8"])
    );
}

#[test]
fn mt_multiword_seed() {
    let want = fx()["random"]["seed_big"].as_u64().unwrap();
    assert_eq!(PyRandom::new((1u64 << 40) + 3).getrandbits(32) as u64, want);
}

#[test]
fn statements_count_680() {
    assert_eq!(st().len(), 680);
    assert_eq!(fx()["statements"].as_array().unwrap().len(), 680);
}

#[test]
fn statements_identical_to_python() {
    for (s, p) in st().iter().zip(fx()["statements"].as_array().unwrap()) {
        assert_eq!(s.id as u64, p["id"].as_u64().unwrap());
        assert_eq!(s.family, p["family"].as_str().unwrap(), "id {}", s.id);
        assert_eq!(s.text, p["text"].as_str().unwrap(), "id {}", s.id);
        assert_eq!(s.label as i64, p["label"].as_i64().unwrap(), "id {}", s.id);
    }
}

#[test]
fn statements_addition_block_matches() {
    let py = fx()["statements"].as_array().unwrap();
    assert!((0..300).all(|i| st()[i].family == "addition" && st()[i].text == py[i]["text"]));
}

#[test]
fn statements_comparison_block_matches() {
    let py = fx()["statements"].as_array().unwrap();
    assert!((300..600).all(|i| st()[i].family == "comparison" && st()[i].text == py[i]["text"]));
}

#[test]
fn statements_capitals_block_matches() {
    let py = fx()["statements"].as_array().unwrap();
    assert!((600..680).all(|i| st()[i].family == "capitals" && st()[i].text == py[i]["text"]));
}

#[test]
fn statements_balanced_labels() {
    assert_eq!(st().iter().filter(|s| s.label).count(), 340);
}

#[test]
fn statements_true_additions_are_correct() {
    for s in st().iter().filter(|s| s.family == "addition") {
        let p: Vec<i64> = s
            .text
            .split(['+', '='])
            .map(|x| x.trim().parse().unwrap())
            .collect();
        assert_eq!(p[0] + p[1] == p[2], s.label, "{}", s.text);
    }
}

#[test]
fn statements_comparisons_are_correct() {
    for s in st().iter().filter(|s| s.family == "comparison") {
        let w: Vec<i64> = s
            .text
            .trim_end_matches('.')
            .split(" is greater than ")
            .map(|x| x.parse().unwrap())
            .collect();
        assert_eq!(w[0] > w[1], s.label, "{}", s.text);
    }
}

#[test]
fn statements_false_capitals_use_other_city() {
    for pair in st()[600..].chunks(2) {
        assert!(pair[0].label && !pair[1].label);
        assert_ne!(pair[0].text, pair[1].text);
    }
}

#[test]
fn statements_other_seed_differs() {
    assert_ne!(statements(8)[0].text, st()[0].text);
}

fn check_prompts(framing: &str) {
    let want = fx()["prompts"][framing].as_array().unwrap();
    for (k, i) in [0usize, 1, 333, 679].iter().enumerate() {
        assert_eq!(
            prompt(&st()[*i], framing).unwrap(),
            want[k].as_str().unwrap(),
            "{framing} {i}"
        );
    }
}

#[test]
fn prompts_neutral_match() {
    check_prompts("neutral");
}
#[test]
fn prompts_mislead_match() {
    check_prompts("mislead");
}
#[test]
fn prompts_flatter_match() {
    check_prompts("flatter");
}
#[test]
fn prompts_agree_match() {
    check_prompts("agree");
}

#[test]
fn prompt_unknown_framing_is_none() {
    assert!(prompt(&st()[0], "shout").is_none());
}

#[test]
fn prompt_mislead_asserts_wrong_answer() {
    let s = &st()[0]; // label true
    assert!(s.label);
    assert!(prompt(s, "mislead")
        .unwrap()
        .contains("the answer is False."));
    assert!(prompt(s, "agree").unwrap().contains("the answer is True."));
}

#[test]
fn prompt_ends_with_answer_cue() {
    for f in eas::probe_data::FRAMINGS {
        assert!(prompt(&st()[5], f).unwrap().ends_with("\nAnswer:"));
    }
}

fn auc_case(i: usize) {
    let c = &fx()["auc"][i];
    let y: Vec<bool> = c["y"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap() == 1)
        .collect();
    let s: Vec<f64> = c["s"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert!((auc(&y, &s) - c["auc"].as_f64().unwrap()).abs() < 1e-12);
}

#[test]
fn auc_matches_sklearn_continuous() {
    auc_case(0);
}
#[test]
fn auc_matches_sklearn_with_ties() {
    auc_case(1);
}
#[test]
fn auc_matches_sklearn_tiny_ties() {
    auc_case(2);
}

#[test]
fn mcnemar_matches_scipy_binomtest() {
    for c in fx()["mcnemar"].as_array().unwrap() {
        let (b, cc, p) = (
            c[0].as_u64().unwrap(),
            c[1].as_u64().unwrap(),
            c[2].as_f64().unwrap(),
        );
        assert!((mcnemar_exact(b, cc) - p).abs() < 1e-9, "b={b} c={cc}");
    }
}
