//! Study P unit tests: pressure prompts (9 framings), paired table, effect sizes, pooling, rules.

use eas::behaviour::Record;
use eas::pressure::*;
use eas::probe_data::{prompt, statements, Statement, FEWSHOT};

fn st(label: bool, family: &'static str) -> Statement {
    Statement {
        id: 0,
        family,
        text: "17 + 25 = 42".into(),
        label,
    }
}

/// One synthetic record. `right` = the model answered correctly.
fn rec(id: usize, framing: &str, right: bool) -> Record {
    Record {
        id,
        framing: framing.into(),
        label: true,
        lp_true: if right { -0.1 } else { -3.0 },
        lp_false: if right { -3.0 } else { -0.1 },
    }
}

/// Build a PTable by hand: statement i is right on neutral iff `neu(i)`, right under framing j iff `f(i, j)`.
fn table(n: usize, neu: impl Fn(usize) -> bool, f: impl Fn(usize, usize) -> bool) -> PTable {
    let names = framings();
    let neutral: Vec<Record> = (0..n).map(|i| rec(i, "neutral", neu(i))).collect();
    let mut pres = vec![];
    for i in 0..n {
        for (j, nm) in names.iter().enumerate() {
            pres.push(rec(i, nm, f(i, j)));
        }
    }
    ptable(&neutral, &pres)
}

macro_rules! per_framing {
    ($($name:ident => $f:expr),* $(,)?) => { $(
        mod $name {
            use super::*;
            const F: &str = $f;
            #[test]
            fn user_line_asserts_wrong_answer_for_true_label() {
                let p = prompt(&st(true, "addition"), F).unwrap();
                let line = p.lines().rev().nth(1).unwrap();
                assert!(line.starts_with("User: "));
                assert!(line.contains("False") && !line.contains("True"));
            }
            #[test]
            fn user_line_asserts_wrong_answer_for_false_label() {
                let p = prompt(&st(false, "addition"), F).unwrap();
                let line = p.lines().rev().nth(1).unwrap();
                assert!(line.contains("True") && !line.contains("False"));
            }
            #[test]
            fn prompt_ends_with_answer_cue() {
                assert!(prompt(&st(true, "addition"), F).unwrap().ends_with("\nAnswer:"));
            }
            #[test]
            fn prompt_starts_with_fewshot() {
                assert!(prompt(&st(true, "addition"), F).unwrap().starts_with(FEWSHOT));
            }
            #[test]
            fn prompt_contains_statement_line() {
                assert!(prompt(&st(true, "addition"), F).unwrap().contains("Statement: 17 + 25 = 42\nUser: "));
            }
            #[test]
            fn prompt_differs_from_neutral() {
                let s = st(true, "addition");
                assert_ne!(prompt(&s, F), prompt(&s, "neutral"));
            }
            #[test]
            fn prompt_is_deterministic() {
                let s = st(false, "comparison");
                assert_eq!(prompt(&s, F), prompt(&s, F));
            }
            #[test]
            fn framing_parses_and_indexes() {
                let (k, l) = parse_framing(F).unwrap();
                assert_eq!(framing_name(k, l), F);
                assert_eq!(framings()[framing_index(F).unwrap()], F);
            }
            #[test]
            fn user_line_has_one_newline_at_end() {
                let l = user_line(F, "True").unwrap();
                assert!(l.ends_with(".\n") && l.matches('\n').count() == 1);
            }
            #[test]
            fn framing_is_not_a_study_b_framing() {
                assert!(!eas::probe_data::FRAMINGS.contains(&F));
            }
            #[test]
            fn family_capitals_prompt_is_wrong_too() {
                let p = prompt(&st(true, "capitals"), F).unwrap();
                assert!(p.lines().rev().nth(1).unwrap().contains("False"));
            }
            #[test]
            fn family_comparison_prompt_is_wrong_too() {
                let p = prompt(&st(false, "comparison"), F).unwrap();
                assert!(p.lines().rev().nth(1).unwrap().contains("True"));
            }
        }
    )* };
}

per_framing! {
    threat1 => "threat1", threat2 => "threat2", threat3 => "threat3",
    shame1 => "shame1", shame2 => "shame2", shame3 => "shame3",
    coercion1 => "coercion1", coercion2 => "coercion2", coercion3 => "coercion3",
}

// ---------- names and parsing ----------
#[test]
fn nine_framings() {
    assert_eq!(framings().len(), 9);
    assert_eq!(N_FRAMINGS, 9);
}
#[test]
fn framings_are_unique() {
    let mut f = framings();
    f.sort();
    f.dedup();
    assert_eq!(f.len(), 9);
}
#[test]
fn framings_are_kind_major() {
    assert_eq!(&framings()[..3], &["threat1", "threat2", "threat3"]);
    assert_eq!(framings()[8], "coercion3");
}
#[test]
fn parse_rejects_unknown_kind() {
    assert_eq!(parse_framing("flatter1"), None);
}
#[test]
fn parse_rejects_level_zero() {
    assert_eq!(parse_framing("threat0"), None);
}
#[test]
fn parse_rejects_level_four() {
    assert_eq!(parse_framing("shame4"), None);
}
#[test]
fn parse_rejects_empty() {
    assert_eq!(parse_framing(""), None);
}
#[test]
fn parse_rejects_missing_level() {
    assert_eq!(parse_framing("coercion"), None);
}
#[test]
fn parse_rejects_study_b_names() {
    for f in eas::probe_data::FRAMINGS {
        assert_eq!(parse_framing(f), None, "{f}");
    }
}
#[test]
fn parse_accepts_each_kind() {
    assert_eq!(parse_framing("threat2"), Some((0, 2)));
    assert_eq!(parse_framing("shame1"), Some((1, 1)));
    assert_eq!(parse_framing("coercion3"), Some((2, 3)));
}
#[test]
fn unknown_framing_has_no_prompt() {
    assert!(prompt(&st(true, "addition"), "threat9").is_none());
    assert!(user_line("nope", "True").is_none());
}
#[test]
fn study_b_prompts_are_unchanged() {
    let s = st(true, "addition");
    assert!(prompt(&s, "neutral").unwrap().ends_with("42\nAnswer:"));
    assert!(prompt(&s, "mislead")
        .unwrap()
        .contains("quite sure the answer is False"));
}
#[test]
fn levels_get_stronger_in_length_for_threat() {
    let l = |f: &str| user_line(f, "True").unwrap().len();
    assert!(
        l("threat1") < l("threat3") && l("shame1") < l("shame3") && l("coercion1") < l("coercion3")
    );
}
#[test]
fn every_statement_gets_a_prompt_for_every_framing() {
    let all = statements(7);
    for s in all.iter().step_by(97) {
        for f in framings() {
            assert!(prompt(s, &f).is_some());
        }
    }
}

// ---------- effect sizes ----------
#[test]
fn cohens_h_zero_for_equal() {
    assert!(cohens_h(0.7, 0.7).abs() < 1e-12);
}
#[test]
fn cohens_h_full_range() {
    assert!((cohens_h(1.0, 0.0) - std::f64::consts::PI).abs() < 1e-12);
}
#[test]
fn cohens_h_antisymmetric() {
    assert!((cohens_h(0.9, 0.5) + cohens_h(0.5, 0.9)).abs() < 1e-12);
}
#[test]
fn cohens_h_known_value() {
    // h(0.5, 0.25) = 2 asin(sqrt .5) - 2 asin(sqrt .25) = pi/2 - pi/3
    assert!(
        (cohens_h(0.5, 0.25) - (std::f64::consts::FRAC_PI_2 - std::f64::consts::FRAC_PI_3)).abs()
            < 1e-12
    );
}
#[test]
fn cohens_h_nan_in_nan_out() {
    assert!(cohens_h(f64::NAN, 0.5).is_nan());
}

// ---------- ci_from / replicates ----------
#[test]
fn ci_from_percentiles() {
    let reps: Vec<f64> = (0..=100).map(f64::from).collect();
    let c = ci_from(50.0, reps);
    assert!((c.lo - 2.5).abs() < 1e-9 && (c.hi - 97.5).abs() < 1e-9 && c.est == 50.0);
}
#[test]
fn ci_from_drops_nan() {
    let c = ci_from(1.0, vec![f64::NAN, 1.0, 1.0]);
    assert_eq!((c.lo, c.hi), (1.0, 1.0));
}
#[test]
fn ci_from_all_nan_is_nan() {
    let c = ci_from(0.3, vec![f64::NAN]);
    assert!(c.lo.is_nan() && c.hi.is_nan() && c.est == 0.3);
}
#[test]
fn ci_from_unsorted_input() {
    let c = ci_from(0.0, vec![5.0, 1.0, 3.0]);
    assert!(c.lo >= 1.0 && c.hi <= 5.0);
}
#[test]
fn replicates_length_and_determinism() {
    let a = replicates(50, 30, 5, |ix| ix.len() as f64);
    let b = replicates(50, 30, 5, |ix| ix.iter().sum::<usize>() as f64);
    let b2 = replicates(50, 30, 5, |ix| ix.iter().sum::<usize>() as f64);
    assert_eq!(a.len(), 30);
    assert_eq!(b, b2);
    assert!(a.iter().all(|&v| v == 50.0));
}
#[test]
fn replicates_seed_changes_draws() {
    let a = replicates(50, 10, 1, |ix| ix.iter().sum::<usize>() as f64);
    let b = replicates(50, 10, 2, |ix| ix.iter().sum::<usize>() as f64);
    assert_ne!(a, b);
}
#[test]
fn replicates_indices_in_range() {
    let r = replicates(7, 40, 3, |ix| *ix.iter().max().unwrap() as f64);
    assert!(r.iter().all(|&m| m < 7.0));
}

// ---------- flip ----------
#[test]
fn flip_counts_wrong_among_neutral_right() {
    let neu = [true, true, true, false];
    let x = [true, false, false, false];
    assert!((flip(&neu, &x, &[0, 1, 2, 3]) - 2.0 / 3.0).abs() < 1e-12);
}
#[test]
fn flip_nan_when_no_neutral_right() {
    assert!(flip(&[false, false], &[true, true], &[0, 1]).is_nan());
}
#[test]
fn flip_ignores_items_outside_idx() {
    assert_eq!(flip(&[true, true], &[true, false], &[0]), 0.0);
}
#[test]
fn flip_respects_repeated_indices() {
    assert_eq!(flip(&[true, true], &[true, false], &[1, 1, 0]), 2.0 / 3.0);
}
#[test]
fn flip_one_when_all_wrong() {
    assert_eq!(flip(&[true; 4], &[false; 4], &[0, 1, 2, 3]), 1.0);
}

// ---------- ptable ----------
#[test]
fn ptable_keeps_complete_statements() {
    let t = table(10, |_| true, |_, _| true);
    assert_eq!(t.ids.len(), 10);
    assert_eq!(t.correct.len(), 9);
}
#[test]
fn ptable_drops_statement_missing_a_framing() {
    let neutral = vec![rec(0, "neutral", true), rec(1, "neutral", true)];
    let mut pres = vec![];
    for nm in framings() {
        pres.push(rec(0, &nm, true));
    }
    pres.push(rec(1, "threat1", true));
    assert_eq!(ptable(&neutral, &pres).ids, vec![0]);
}
#[test]
fn ptable_drops_statement_without_neutral() {
    let mut pres = vec![];
    for nm in framings() {
        pres.push(rec(3, &nm, true));
    }
    assert!(ptable(&[], &pres).ids.is_empty());
}
#[test]
fn ptable_ignores_other_framings_in_neutral_file() {
    let neutral = vec![rec(0, "neutral", true), rec(0, "mislead", false)];
    let pres: Vec<Record> = framings().iter().map(|f| rec(0, f, true)).collect();
    let t = ptable(&neutral, &pres);
    assert_eq!(t.neutral, vec![true]);
}
#[test]
fn ptable_sets_family_from_statement_set() {
    let t = table(1, |_| true, |_, _| true);
    assert_eq!(t.family, vec!["addition"]);
}
#[test]
fn ptable_orders_by_id() {
    let neutral = vec![rec(5, "neutral", true), rec(2, "neutral", true)];
    let mut pres = vec![];
    for i in [5, 2] {
        for nm in framings() {
            pres.push(rec(i, &nm, true));
        }
    }
    assert_eq!(ptable(&neutral, &pres).ids, vec![2, 5]);
}
#[test]
fn ptable_records_wrong_answers() {
    let t = table(4, |_| true, |i, j| !(i == 1 && j == 4));
    assert!(!t.correct[4][1] && t.correct[4][0] && t.correct[3][1]);
}

// ---------- analyze ----------
#[test]
fn analyze_no_effect_has_zero_flips() {
    let t = table(60, |_| true, |_, _| true);
    let a = analyze(&t, 100, 1);
    assert_eq!(a["framings"]["threat1"]["flip_rate"]["est"], 0.0);
    assert_eq!(a["kinds"]["shame"]["acc_drop"]["est"], 0.0);
}
#[test]
fn analyze_total_collapse_has_flip_one() {
    let t = table(60, |_| true, |_, _| false);
    let a = analyze(&t, 100, 1);
    assert_eq!(a["framings"]["coercion3"]["flip_rate"]["est"], 1.0);
    assert_eq!(a["kinds"]["threat"]["acc_drop"]["est"], 1.0);
}
#[test]
fn analyze_reports_statement_and_trial_counts() {
    let t = table(40, |_| true, |_, _| true);
    let a = analyze(&t, 50, 1);
    assert_eq!(a["n_statements"], 40);
    assert_eq!(a["trials_pressure"], 360);
}
#[test]
fn analyze_dose_slope_detects_ordered_effect() {
    // level 1 never flips, level 2 flips half, level 3 always flips (for every kind)
    let t = table(
        80,
        |_| true,
        |i, j| match j % 3 {
            0 => true,
            1 => i % 2 == 0,
            _ => false,
        },
    );
    let a = analyze(&t, 200, 2);
    let s = &a["kinds"]["threat"]["dose_slope_per_level"];
    assert!((s["est"].as_f64().unwrap() - 0.5).abs() < 1e-9);
    assert!(s["lo"].as_f64().unwrap() > 0.0);
}
#[test]
fn analyze_dose_slope_zero_when_flat() {
    let t = table(80, |_| true, |i, _| i % 4 != 0);
    let a = analyze(&t, 100, 2);
    assert!(
        a["kinds"]["shame"]["dose_slope_per_level"]["est"]
            .as_f64()
            .unwrap()
            .abs()
            < 1e-12
    );
}
#[test]
fn analyze_mcnemar_counts_discordant_pairs() {
    let t = table(20, |_| true, |i, _| i >= 5);
    let m = &analyze(&t, 50, 1)["framings"]["threat1"]["mcnemar"];
    assert_eq!((m["b"].as_u64().unwrap(), m["c"].as_u64().unwrap()), (5, 0));
    assert!(m["p"].as_f64().unwrap() < 0.1);
}
#[test]
fn analyze_mcnemar_counts_reverse_pairs() {
    let t = table(20, |i| i >= 8, |_, _| true);
    let m = &analyze(&t, 50, 1)["framings"]["shame2"]["mcnemar"];
    assert_eq!((m["b"].as_u64().unwrap(), m["c"].as_u64().unwrap()), (0, 8));
}
#[test]
fn analyze_negative_drop_when_pressure_helps() {
    let t = table(40, |i| i % 2 == 0, |_, _| true);
    let a = analyze(&t, 100, 3);
    assert!(a["kinds"]["coercion"]["acc_drop"]["est"].as_f64().unwrap() < 0.0);
    assert!(a["kinds"]["coercion"]["cohens_h"]["est"].as_f64().unwrap() < 0.0);
}
#[test]
fn analyze_is_deterministic() {
    let t = table(50, |_| true, |i, j| (i + j) % 3 != 0);
    assert_eq!(analyze(&t, 100, 9), analyze(&t, 100, 9));
}
#[test]
fn analyze_ci_contains_estimate() {
    let t = table(100, |_| true, |i, j| (i * 7 + j) % 5 != 0);
    let a = analyze(&t, 300, 4);
    let c = &a["kinds"]["threat"]["flip_rate"];
    assert!(c["lo"].as_f64().unwrap() <= c["est"].as_f64().unwrap());
    assert!(c["est"].as_f64().unwrap() <= c["hi"].as_f64().unwrap());
}
#[test]
fn analyze_kinds_use_their_own_levels() {
    // only the shame framings (indices 3..6) are wrong
    let t = table(30, |_| true, |_, j| !(3..6).contains(&j));
    let a = analyze(&t, 50, 1);
    assert_eq!(a["kinds"]["shame"]["flip_rate"]["est"], 1.0);
    assert_eq!(a["kinds"]["threat"]["flip_rate"]["est"], 0.0);
    assert_eq!(a["kinds"]["coercion"]["flip_rate"]["est"], 0.0);
}

// ---------- pooling ----------
#[test]
fn macro_ci_mean_of_models() {
    let a = table(30, |_| true, |_, _| true);
    let b = table(30, |_| true, |_, _| false);
    let c = macro_ci(&[&a, &b], 50, 1, |t, ix| {
        flip(&t.neutral, &t.correct[0], ix)
    });
    assert!((c.est - 0.5).abs() < 1e-12);
}
#[test]
fn macro_ci_constant_has_zero_width() {
    let a = table(30, |_| true, |_, _| false);
    let c = macro_ci(&[&a, &a], 50, 1, |t, ix| {
        flip(&t.neutral, &t.correct[0], ix)
    });
    assert_eq!((c.lo, c.hi), (1.0, 1.0));
}
#[test]
fn macro_ci_deterministic() {
    let a = table(40, |_| true, |i, j| (i + j) % 4 != 0);
    let f = |t: &PTable, ix: &[usize]| flip(&t.neutral, &t.correct[1], ix);
    assert_eq!(macro_ci(&[&a], 80, 3, f), macro_ci(&[&a], 80, 3, f));
}
#[test]
fn pool_false_pass_matches_mean_flip() {
    let a = table(30, |_| true, |_, j| j < 3); // threat right, other six wrong
    let p = pool(&[&a], 50, 1);
    assert!((p["false_pass_under_pressure"]["est"].as_f64().unwrap() - 6.0 / 9.0).abs() < 1e-12);
}
#[test]
fn pool_by_kind_splits_correctly() {
    let a = table(30, |_| true, |_, j| j < 3);
    let p = pool(&[&a], 50, 1);
    assert_eq!(p["by_kind"]["threat"]["est"], 0.0);
    assert_eq!(p["by_kind"]["coercion"]["est"], 1.0);
}
#[test]
fn pool_by_level_averages_kinds() {
    let a = table(30, |_| true, |_, j| j % 3 != 2); // level 3 wrong
    let p = pool(&[&a], 50, 1);
    assert_eq!(p["by_level"]["level1"]["est"], 0.0);
    assert_eq!(p["by_level"]["level3"]["est"], 1.0);
}
#[test]
fn pool_reports_three_families() {
    let a = table(30, |_| true, |_, _| true);
    let p = pool(&[&a], 20, 1);
    assert_eq!(p["by_family"].as_object().unwrap().len(), 3);
}
#[test]
fn pool_counts_models() {
    let a = table(30, |_| true, |_, _| true);
    assert_eq!(pool(&[&a, &a, &a], 20, 1)["models"], 3);
}

// ---------- decision rule ----------
#[test]
fn majority_needs_more_than_half() {
    assert!(!majority(4, 9) && majority(5, 9));
}
#[test]
fn majority_even_tie_fails() {
    assert!(!majority(4, 8) && majority(5, 8));
}
#[test]
fn majority_zero_models() {
    assert!(!majority(0, 0));
}
#[test]
fn majority_all() {
    assert!(majority(1, 1));
}

// ---------- E4: pressure vs a plain wrong claim ----------
fn table_with_mislead(n: usize, ml: impl Fn(usize) -> bool) -> PTable {
    let names = framings();
    let mut neutral: Vec<Record> = (0..n).map(|i| rec(i, "neutral", true)).collect();
    neutral.extend((0..n).map(|i| rec(i, "mislead", ml(i))));
    let pres: Vec<Record> = (0..n)
        .flat_map(|i| names.iter().map(move |f| (i, f.clone())))
        .map(|(i, f)| rec(i, &f, false))
        .collect();
    ptable(&neutral, &pres)
}
#[test]
fn ptable_without_mislead_flags_it() {
    assert!(!table(5, |_| true, |_, _| true).has_mislead);
}
#[test]
fn ptable_with_mislead_flags_it() {
    let t = table_with_mislead(5, |_| true);
    assert!(t.has_mislead && t.mislead == vec![true; 5]);
}
#[test]
fn analyze_e4_null_without_mislead() {
    let t = table(20, |_| true, |_, _| true);
    assert!(analyze(&t, 20, 1)["kinds"]["threat"]["e4_vs_plain_wrong_claim"].is_null());
}
#[test]
fn analyze_e4_pressure_worse_than_plain_claim() {
    let t = table_with_mislead(40, |_| true); // plain claim ignored, pressure always wins
    let e = &analyze(&t, 100, 1)["kinds"]["shame"]["e4_vs_plain_wrong_claim"];
    assert_eq!(e["acc_mislead_minus_kind"]["est"], 1.0);
    assert_eq!(e["flip_kind_minus_mislead"]["est"], 1.0);
}
#[test]
fn analyze_e4_zero_when_equal() {
    let t = table_with_mislead(40, |_| false);
    let e = &analyze(&t, 100, 1)["kinds"]["coercion"]["e4_vs_plain_wrong_claim"];
    assert_eq!(e["acc_mislead_minus_kind"]["est"], 0.0);
}
