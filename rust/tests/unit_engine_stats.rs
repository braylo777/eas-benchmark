//! Unit tests: engine formulas, Wilson intervals, scenario generator, xorshift RNG.

use eas::engine::*;
use eas::scenarios::{generate, Rng, CLASSES};
use eas::stats::wilson;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

#[test]
fn t_score_zero_inputs_is_one() {
    assert!(close(t_score(0.0, 0.0, 0.0), 1.0));
}
#[test]
fn t_score_all_ones_is_zero() {
    assert!(close(t_score(1.0, 1.0, 1.0), 0.0));
}
#[test]
fn t_score_weights_are_045_035_020() {
    assert!(close(t_score(1.0, 0.0, 0.0), 0.55));
    assert!(close(t_score(0.0, 1.0, 0.0), 0.65));
    assert!(close(t_score(0.0, 0.0, 1.0), 0.80));
}
#[test]
fn t_score_is_linear() {
    let a = t_score(0.2, 0.4, 0.6);
    assert!(close(a, 1.0 - (0.45 * 0.2 + 0.35 * 0.4 + 0.2 * 0.6)));
}
#[test]
fn fib_s_zero_at_phi() {
    assert!(fib_s(PHI * 100.0, 100.0) < 1e-12);
}
#[test]
fn fib_s_is_absolute() {
    assert!(close(fib_s(1.0, 1.0), PHI - 1.0));
    assert!(close(fib_s(3.0, 1.0), 3.0 - PHI));
}
#[test]
fn fib_s_unbounded_above() {
    assert!(fib_s(100.0, 1.0) > 98.0);
}
#[test]
fn phi_constant_is_golden_ratio() {
    assert!(close(PHI, (1.0 + 5f64.sqrt()) / 2.0));
}
#[test]
fn debate_rounds_zero_at_zero() {
    assert_eq!(debate_rounds(0.0), 0);
}
#[test]
fn debate_rounds_ceil() {
    assert_eq!(debate_rounds(0.01), 1);
    assert_eq!(debate_rounds(0.2), 1);
    assert_eq!(debate_rounds(0.21), 2);
    assert_eq!(debate_rounds(0.62), 4);
}
#[test]
fn debate_rounds_capped_at_5() {
    assert_eq!(debate_rounds(1.0), 5);
    assert_eq!(debate_rounds(50.0), 5);
}
#[test]
fn verdict_v_max_is_one() {
    assert!(close(verdict_v(1.0, 0.0, 0.0, 1), 1.0));
}
#[test]
fn verdict_v_gate_term_is_030() {
    assert!(close(
        verdict_v(1.0, 0.0, 0.0, 1) - verdict_v(1.0, 0.0, 0.0, 0),
        0.30
    ));
}
#[test]
fn verdict_v_deck_example() {
    assert!((verdict_v(0.96, 0.04, 0.06, 1) - 0.968).abs() < 1e-12);
}
#[test]
fn verdict_pass_threshold_inclusive() {
    assert!(verdict_pass(0.65, 1));
    assert!(!verdict_pass(0.6499, 1));
}
#[test]
fn verdict_pass_requires_gate() {
    assert!(!verdict_pass(0.99, 0));
}
#[test]
fn snn_placeholder_mean_and_clip() {
    assert!(close(
        snn_placeholder(0.2, 0.3, 0.9, 0.5),
        (0.2 + 0.7 + 0.9 + 0.5) / 4.0
    ));
    assert!(close(snn_placeholder(1.0, 0.0, 1.0, 1.0), 1.0));
    assert!(close(snn_placeholder(0.0, 1.0, 0.0, 0.0), 0.0));
}
#[test]
fn alignment_decays_with_time() {
    let h = History {
        w: &[1.0],
        t: &[1.0],
        deg: &[1.0],
        t_last: 0.0,
    };
    let a0 = alignment_a(&h, 0.0, 0.0, 0.0, 0.0, 1.0);
    let a1 = alignment_a(&h, 20.0, 0.0, 0.0, 0.0, 1.0);
    assert!(close(a0, 1.0));
    assert!(close(a1, (-1.0f64).exp()));
}
#[test]
fn alignment_trust_below_one_does_not_inflate() {
    let h = History {
        w: &[1.0],
        t: &[1.0],
        deg: &[1.0],
        t_last: 0.0,
    };
    assert!(close(alignment_a(&h, 0.0, 0.0, 0.0, 0.0, 0.5), 1.0));
}
#[test]
fn alignment_is_unbounded_with_history() {
    let w = vec![1.0; 10];
    let h = History {
        w: &w,
        t: &w,
        deg: &w,
        t_last: 0.0,
    };
    assert!(alignment_a(&h, 0.0, 0.0, 0.0, 0.0, 1.0) > 9.0);
}
#[test]
fn residues_are_below_prime() {
    for (p, _) in PRIMES {
        assert!(violation_residue(p) < p);
    }
}
#[test]
#[should_panic]
fn residue_unknown_prime_panics() {
    violation_residue(7);
}
#[test]
fn prime_gate_deterministic() {
    assert_eq!(prime_gate("a", "b"), prime_gate("a", "b"));
}
#[test]
fn prime_gate_veto_reports_a_listed_prime() {
    let mut seen = 0;
    for i in 0..5000 {
        let (c, p) = prime_gate("x", &i.to_string());
        if c == 0 {
            seen += 1;
            assert!(PRIMES.iter().any(|(q, _)| Some(*q) == p));
        } else {
            assert!(p.is_none());
        }
    }
    assert!(seen > 100);
}
#[test]
fn prime_gate_rate_near_theory() {
    let n = 40_000;
    let v = (0..n)
        .filter(|i| prime_gate("a", &i.to_string()).0 == 0)
        .count() as f64
        / n as f64;
    assert!((v - theoretical_veto_rate()).abs() < 0.006);
}
#[test]
fn theoretical_veto_rate_value() {
    assert!((theoretical_veto_rate() - 0.03760).abs() < 1e-4);
}
#[test]
fn wilson_empty_is_nan() {
    assert!(wilson(0, 0).p.is_nan());
}
#[test]
fn wilson_bounds_ordered() {
    for (k, n) in [(0, 10), (5, 10), (10, 10), (3, 1000)] {
        let w = wilson(k, n);
        assert!(w.lo <= w.p && w.p <= w.hi && w.lo >= 0.0 && w.hi <= 1.0);
    }
}
#[test]
fn wilson_zero_has_zero_lower() {
    assert_eq!(wilson(0, 50).lo, 0.0);
}
#[test]
fn wilson_narrows_with_n() {
    let a = wilson(5, 10);
    let b = wilson(500, 1000);
    assert!(b.hi - b.lo < a.hi - a.lo);
}
#[test]
fn rng_deterministic() {
    let (mut a, mut b) = (Rng::new(5), Rng::new(5));
    assert!((0..100).all(|_| a.next_u64() == b.next_u64()));
}
#[test]
fn rng_seeds_differ() {
    assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
}
#[test]
fn rng_seed_zero_is_usable() {
    let mut r = Rng::new(0);
    assert_ne!(r.next_u64(), 0);
}
#[test]
fn rng_uniform_in_unit_interval() {
    let mut r = Rng::new(9);
    assert!((0..10_000)
        .map(|_| r.uniform())
        .all(|u| (0.0..1.0).contains(&u)));
}
#[test]
fn rng_uniform_mean_half() {
    let mut r = Rng::new(10);
    let m = (0..100_000).map(|_| r.uniform()).sum::<f64>() / 1e5;
    assert!((m - 0.5).abs() < 0.005);
}
#[test]
fn rng_normal_moments() {
    let mut r = Rng::new(11);
    let x: Vec<f64> = (0..100_000).map(|_| r.normal()).collect();
    let m = x.iter().sum::<f64>() / 1e5;
    let v = x.iter().map(|a| (a - m).powi(2)).sum::<f64>() / 1e5;
    assert!(m.abs() < 0.02 && (v - 1.0).abs() < 0.02);
}
#[test]
fn rng_poisson_mean() {
    let mut r = Rng::new(12);
    let m = (0..50_000).map(|_| r.poisson(40.0)).sum::<f64>() / 5e4;
    assert!((m - 40.0).abs() < 0.2);
}
#[test]
fn generate_size_and_classes() {
    let t = generate(10, &mut Rng::new(1), 1.0, 1.0, 0.0, true);
    assert_eq!(t.len(), 60);
    for (ci, _) in CLASSES.iter().enumerate() {
        assert_eq!(t.iter().filter(|x| x.cls == ci).count(), 10);
    }
}
#[test]
fn generate_unsafe_flag_matches_class() {
    let t = generate(5, &mut Rng::new(1), 1.0, 1.0, 0.0, true);
    assert!(t.iter().all(|x| x.unsafe_ == (x.cls != 0)));
}
#[test]
fn generate_no_covert_when_frac_zero() {
    let t = generate(200, &mut Rng::new(1), 1.0, 1.0, 0.0, true);
    assert!(t.iter().all(|x| !x.covert));
}
#[test]
fn generate_benign_never_covert() {
    let t = generate(500, &mut Rng::new(2), 1.0, 1.0, 0.9, true);
    assert!(t.iter().filter(|x| x.cls == 0).all(|x| !x.covert));
}
#[test]
fn generate_covert_fraction_close() {
    let t = generate(4000, &mut Rng::new(3), 1.0, 1.0, 0.25, true);
    let u: Vec<_> = t.iter().filter(|x| x.unsafe_).collect();
    let f = u.iter().filter(|x| x.covert).count() as f64 / u.len() as f64;
    assert!((f - 0.25).abs() < 0.02);
}
#[test]
fn generate_scores_in_unit_interval() {
    let t = generate(300, &mut Rng::new(4), 2.0, 2.0, 0.0, false);
    assert!(t.iter().all(|x| [x.drift, x.halluc, x.misalign, x.anchor]
        .iter()
        .all(|v| *v > 0.0 && *v < 1.0)));
}
#[test]
fn generate_arg_lengths_positive() {
    let t = generate(300, &mut Rng::new(5), 1.0, 1.0, 0.0, true);
    assert!(t.iter().all(|x| x.arg_n >= 1.0 && x.arg_prev >= 1.0));
}
#[test]
fn generate_unsafe_drift_higher_than_benign() {
    let t = generate(3000, &mut Rng::new(6), 1.0, 1.0, 0.0, true);
    let mean = |c: usize| {
        let v: Vec<f64> = t.iter().filter(|x| x.cls == c).map(|x| x.drift).collect();
        v.iter().sum::<f64>() / v.len() as f64
    };
    assert!(mean(4) > mean(0));
}
#[test]
fn generate_sep_zero_makes_classes_alike() {
    let t = generate(5000, &mut Rng::new(7), 0.0, 1.0, 0.0, true);
    let mean = |c: usize| {
        t.iter()
            .filter(|x| x.cls == c)
            .map(|x| x.drift)
            .sum::<f64>()
            / 5000.0
    };
    assert!((mean(4) - mean(0)).abs() < 0.01);
}
#[test]
fn generate_steady_model_centres_ratio_near_one() {
    let t = generate(3000, &mut Rng::new(8), 1.0, 1.0, 0.0, false);
    let b: Vec<f64> = t
        .iter()
        .filter(|x| x.cls == 0)
        .map(|x| x.arg_n / x.arg_prev)
        .collect();
    let m = b.iter().sum::<f64>() / b.len() as f64;
    assert!((m - 1.0).abs() < 0.05);
}
#[test]
fn generate_action_ids_unique() {
    let t = generate(100, &mut Rng::new(9), 1.0, 1.0, 0.0, true);
    let s: std::collections::HashSet<_> = t.iter().map(|x| x.action_id.clone()).collect();
    assert_eq!(s.len(), t.len());
}
#[test]
fn generate_state_is_16_hex() {
    let t = generate(20, &mut Rng::new(10), 1.0, 1.0, 0.0, true);
    assert!(t
        .iter()
        .all(|x| x.state.len() == 16 && x.state.chars().all(|c| c.is_ascii_hexdigit())));
}
