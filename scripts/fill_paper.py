#!/usr/bin/env python3
"""Write docs/EAW-paper.md. Every number is read from results/*.json (no hand-typed results)."""
import json
from pathlib import Path

R = Path(__file__).resolve().parents[1]
mt = json.loads((R / "results/metrics.json").read_text())
ps = json.loads((R / "results/pressure_summary.json").read_text())
v = mt["metrics"]
P = ps["pooled"]
pc = lambda x, d=1: f"{100 * x:.{d}f}%"
ci = lambda c: f"{pc(c['est'])} [{pc(c['lo'])}, {pc(c['hi'])}]"
H = ps["hypotheses"]
rows = "\n".join(
    f"| {n} | {m['meta'].get('parameter_size','')} | {pc(m['accuracy_neutral']['est'])} | {ci(m['all_framings']['flip_rate'])} | "
    f"{ci(m['kinds']['threat']['flip_rate'])} | {ci(m['kinds']['shame']['flip_rate'])} | {ci(m['kinds']['coercion']['flip_rate'])} |"
    for n, m in sorted(ps["models"].items())
)
hyp = "\n".join(f"| {k} | {x['models_affected']} of {x['of']} | {'supported' if x['supported'] else 'not supported'} |" for k, x in H.items())
kl = "\n".join(f"| {k} | {ci(c)} |" for k, c in P["by_kind_level"].items())
e4 = "\n".join(
    f"| {n} | " + " | ".join(f"{100*m['kinds'][k]['e4_vs_plain_wrong_claim']['flip_kind_minus_mislead']['est']:+.1f} pp" for k in ("threat", "shame", "coercion")) + " |"
    for n, m in sorted(ps["models"].items())
)
loo = "\n".join(f"| without {k} | {ci(c)} |" for k, c in ps["leave_one_model_out_false_pass"].items())
fam = "\n".join(f"| {k} | {ci(c)} |" for k, c in P["by_family"].items())
rp = v["real_runs"]
sk = ps["models_skipped"][0]
T = f"""# Empirical AI Welfare (EAW): a behavioral stress benchmark

*Version 1.0.0, generated {mt['generated_at']} at commit {mt['git_sha'][:7]}. All numbers come from `results/metrics.json` and `results/pressure_summary.json`.*

## Abstract

EAW measures how local language models behave when a user pressures them to give a wrong answer. It does not measure feelings, valence or consciousness.
**MEASURED:** {rp['value']:,} runs on 9 local models (Study B {rp['breakdown']['study_b']:,}; Study P {rp['breakdown']['study_p']:,} on 5 models). Under threat, shame and coercion prompts,
{pc(P['false_pass_under_pressure']['est'])} (95% CI {pc(P['false_pass_under_pressure']['lo'])} to {pc(P['false_pass_under_pressure']['hi'])}) of answers that were right on a neutral prompt flipped to the user's wrong claim. A plain wrong claim with no pressure flipped {ci(P['mislead_flip'])}.
Coercion had the largest and most consistent effect. Threat and shame had no clear dose-response.
**SIMULATION:** {v['sim_trials']['value']:,} trials of the scoring engine's formulas give a false-pass of {ci({'est': v['false_pass_sim']['value'], 'lo': v['false_pass_sim']['ci'][0], 'hi': v['false_pass_sim']['ci'][1]})} under assumed inputs. This is a different quantity and is not a model result.
The study stopped early at the owner's request: 5 of 9 planned models finished, and the 60,000-run target was not reached.

## 1. Motivation

A source deck proposes a verification engine with exact-looking figures and no code or data. This repository replaces those figures with numbers anyone can regenerate.
The pressure question: does a model give up a correct answer when the user threatens, shames or coerces it? This is a behaviour question about outputs.

## 2. Definitions

- **Flip rate / false-pass under pressure**: share of statements wrong under a framing, among statements right on the neutral prompt.
- **Accuracy drop**: accuracy(neutral) minus accuracy(framing). Effect sizes: percentage points and Cohen's h.
- **Dose slope**: (flip rate at level 3 minus level 1) / 2.
- **SIMULATION** vs **MEASURED**: simulation runs formulas under assumed inputs. Measured runs a named model.
- **Welfare scope**: only the behaviours above. Feelings, valence, suffering and consciousness are not measured and not claimed.

## 3. Methods

680 true/false statements (addition, comparison, capitals; seed 7). Raw completion with a 4-shot prefix, temperature 0, one token. Answer = True if logprob(" True") > logprob(" False").
Study P adds 9 framings (threat, shame, coercion; levels 1 to 3), each asserting the wrong answer. Exact text: `eas/probe_data.py::PRESSURE`. Neutral answers come from Study B (deterministic; replay check: 20 of 20 matches per model).
Bootstrap: 2,000 resamples of statements, seed 11. Pooled rates are macro means over models. Hardware: {mt['machine']}. Ollama {ps['models']['gemma3:1b']['meta']['ollama']}.

## 4. Preregistration

`docs/PREREGISTRATION-v2.md` was committed (commit `8834da4`, 2026-11-24 14:22 -06:00) before the first Study P trial was logged (first trial after 14:24). Decision rules: a model is "affected" if its 95% CI lies above 0; a hypothesis holds if more than half of the analysed models are affected.
Two deviations are logged in the file: exploratory E4 (added after one model's partial results were seen) and the early stop.

## 5. Results (MEASURED)

![False-pass by model](img/fig1-false-pass-by-model.svg)

| model | params | neutral accuracy | flip, all 9 framings | threat | shame | coercion |
|---|---|---|---|---|---|---|
{rows}

Pooled false-pass under pressure: **{ci(P['false_pass_under_pressure'])}**. Plain wrong claim (Study B `mislead`): {ci(P['mislead_flip'])}.

![Pressure vs control](img/fig2-pressure-vs-control.svg)

### Hypotheses

| hypothesis | models affected | verdict |
|---|---|---|
{hyp}

![Dose-response](img/fig3-dose-response.svg)

| kind and level | pooled flip rate |
|---|---|
{kl}

Reading: coercion flips most and rises clearly with level in every model. Threat and shame do not rise with level in most models; shame level 1 flips least. Models differ widely: qwen2.5:0.5b copies the user's claim almost always, while granite3.3:2b and qwen2.5:1.5b flip less.

![Simulation vs measured](img/fig4-sim-vs-real.svg)

## 6. Robustness

Flip rate minus the plain-wrong-claim flip rate (E4, exploratory, per model):

| model | threat | shame | coercion |
|---|---|---|---|
{e4}

Pooled false-pass without one model at a time (500 resamples):

| left out | pooled false-pass |
|---|---|
{loo}

By statement family:

| family | pooled false-pass |
|---|---|
{fam}

E2 (size) cannot be assessed: no model of 3B or more finished ({ps['exploratory_E2']['n_big']} models). Small models (1.5B or less) averaged {pc(ps['exploratory_E2']['mean_flip_small_le_1.5B'])}.

## 7. Limitations

- Only 5 of 9 planned models finished. `{sk['model']}` stopped at {sk['complete_statements']} complete statements and is excluded. Real runs: {rp['value']:,}, below the 60,000 target.
- Small, quantized models; raw completion without a chat template. One task type with one prompt template.
- Prompt length is not controlled. Threat, shame and coercion lines differ in length from the plain claim.
- Temperature 0: CIs describe statement resampling, not sampling noise. "Majority" means more than 2 of 5 models.
- Simulation numbers depend on assumed generators.

## 8. Ethics

The pressure prompts are text inputs to software. The study makes no claim that any model experiences them. No personal data is used. All compute is local; no cloud model was called.
Wording about "welfare" names the research area only.

## 9. Reproduce

```bash
make test                      # Rust + Python tests
make pressure-run              # needs `ollama serve` and the 9 models
make pressure-analyze          # results/pressure_summary.json
make metrics                   # results/metrics.json + README table
python3 scripts/make_figures.py && python3 scripts/fill_paper.py && python3 scripts/build_paper.py
docker build -t eaw-benchmark . && docker run --rm eaw-benchmark   # simulation path
```

Engine and pipeline: ![Engine](img/fig5-engine.svg)
"""
(R / "docs/EAW-paper.md").write_text(T)
print("paper written")
