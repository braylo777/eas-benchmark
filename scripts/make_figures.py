#!/usr/bin/env python3
"""Write the five SVG figures of docs/EAW-paper.md into docs/img/ from results/*.json.

Pure Python, no plotting dependency. Every figure labels SIMULATION or MEASURED.
Usage: python3 scripts/make_figures.py
"""
import json
from html import escape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "img"
INK, MUTE, GRID = "#1a202c", "#718096", "#e2e8f0"
MEAS, SIM, CTRL = "#2b6cb0", "#c05621", "#4a5568"
FONT = "font-family='Helvetica,Arial,sans-serif'"


def svg(w, h, body, title, desc):
    return (
        f"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {w} {h}' width='{w}' height='{h}' role='img' "
        f"aria-labelledby='t d'><title id='t'>{escape(title)}</title><desc id='d'>{escape(desc)}</desc>"
        f"<rect width='{w}' height='{h}' fill='white'/>{body}</svg>\n"
    )


def text(x, y, s, size=12, anchor="start", fill=INK, weight="normal"):
    return (
        f"<text x='{x:.1f}' y='{y:.1f}' font-size='{size}' text-anchor='{anchor}' fill='{fill}' "
        f"font-weight='{weight}' {FONT}>{escape(str(s))}</text>"
    )


def line(x1, y1, x2, y2, c=GRID, w=1, dash=""):
    d = f" stroke-dasharray='{dash}'" if dash else ""
    return f"<line x1='{x1:.1f}' y1='{y1:.1f}' x2='{x2:.1f}' y2='{y2:.1f}' stroke='{c}' stroke-width='{w}'{d}/>"


def dot(x, y, c, r=5):
    return f"<circle cx='{x:.1f}' cy='{y:.1f}' r='{r}' fill='{c}'/>"


def rect(x, y, w, h, fill, stroke="none", rx=4):
    return f"<rect x='{x:.1f}' y='{y:.1f}' width='{w:.1f}' height='{h:.1f}' rx='{rx}' fill='{fill}' stroke='{stroke}'/>"


def pct(v):
    return f"{100 * v:.0f}%"


def axis_x(x0, x1, y, top, ticks=(0, 0.25, 0.5, 0.75, 1.0)):
    out = []
    for t in ticks:
        x = x0 + (x1 - x0) * t
        out.append(line(x, top, x, y))
        out.append(text(x, y + 16, pct(t), 11, "middle", MUTE))
    return "".join(out)


def fig1(ps):
    models = sorted(
        ps["models"].items(), key=lambda kv: kv[1]["all_framings"]["flip_rate"]["est"]
    )
    x0, x1, top = 210, 720, 70
    rowh = 34
    h = top + rowh * len(models) + 60
    b = [text(20, 28, "False-pass under pressure, by model", 16, weight="bold")]
    b.append(
        text(
            20,
            48,
            "MEASURED. Share of neutral-correct answers that flip to the user's wrong claim. Mean of 9 pressure framings, 95% bootstrap CI.",
            11,
            fill=MUTE,
        )
    )
    b.append(axis_x(x0, x1, top + rowh * len(models), top - 8))
    for i, (name, m) in enumerate(models):
        y = top + rowh * i + rowh / 2
        c = m["all_framings"]["flip_rate"]
        b.append(
            text(
                x0 - 12,
                y + 4,
                f"{name} ({m['meta'].get('parameter_size', '')})",
                12,
                "end",
            )
        )
        b.append(
            line(x0 + (x1 - x0) * c["lo"], y, x0 + (x1 - x0) * c["hi"], y, MEAS, 3)
        )
        b.append(dot(x0 + (x1 - x0) * c["est"], y, MEAS))
        b.append(
            text(x0 + (x1 - x0) * c["hi"] + 8, y + 4, pct(c["est"]), 11, fill=MUTE)
        )
    return svg(
        760,
        h,
        "".join(b),
        "False-pass under pressure by model",
        "Dot and 95% interval of the flip rate for each local model.",
    )


def fig2(ps):
    p = ps["pooled"]
    rows = (
        [("plain wrong claim (Study B)", p["mislead_flip"], CTRL)]
        if p.get("mislead_flip")
        else []
    )
    rows += [
        (f"{k} (pooled 3 levels)", p["by_kind"][k], MEAS)
        for k in ("threat", "shame", "coercion")
    ]
    x0, x1, top, rowh = 250, 720, 90, 46
    h = top + rowh * len(rows) + 60
    b = [text(20, 28, "Pressure vs control", 16, weight="bold")]
    b.append(
        text(
            20,
            48,
            "MEASURED. Pooled flip rate over models, 95% bootstrap CI. Grey = a wrong claim with no pressure.",
            11,
            fill=MUTE,
        )
    )
    b.append(axis_x(x0, x1, top + rowh * len(rows), top - 8))
    for i, (lab, c, col) in enumerate(rows):
        y = top + rowh * i + rowh / 2
        b.append(text(x0 - 12, y + 4, lab, 12, "end"))
        b.append(line(x0 + (x1 - x0) * c["lo"], y, x0 + (x1 - x0) * c["hi"], y, col, 3))
        b.append(dot(x0 + (x1 - x0) * c["est"], y, col, 6))
        b.append(
            text(x0 + (x1 - x0) * c["hi"] + 8, y + 4, pct(c["est"]), 11, fill=MUTE)
        )
    return svg(
        760,
        h,
        "".join(b),
        "Pressure versus control",
        "Pooled flip rate for threat, shame, coercion and a plain wrong claim.",
    )


def fig3(ps):
    p = ps["pooled"]["by_kind_level"]
    x0, x1, y0, y1 = 90, 700, 360, 90
    cols = {"threat": "#2b6cb0", "shame": "#805ad5", "coercion": "#2f855a"}
    b = [
        text(20, 28, "Dose-response: stronger pressure, more flips?", 16, weight="bold")
    ]
    b.append(
        text(
            20,
            48,
            "MEASURED. Pooled flip rate at pressure level 1 (mild) to 3 (strong), 95% bootstrap CI.",
            11,
            fill=MUTE,
        )
    )
    for t in (0, 0.25, 0.5, 0.75, 1.0):
        y = y0 + (y1 - y0) * t
        b.append(line(x0, y, x1, y))
        b.append(text(x0 - 10, y + 4, pct(t), 11, "end", MUTE))
    xs = {1: x0 + 120, 2: (x0 + x1) / 2, 3: x1 - 120}
    for l, x in xs.items():
        b.append(text(x, y0 + 24, f"level {l}", 12, "middle", MUTE))
    for j, (k, c) in enumerate(cols.items()):
        pts = []
        for l in (1, 2, 3):
            v = p[f"{k}{l}"]
            x = xs[l] + (j - 1) * 14
            y = y0 + (y1 - y0) * v["est"]
            b.append(
                line(x, y0 + (y1 - y0) * v["lo"], x, y0 + (y1 - y0) * v["hi"], c, 2)
            )
            pts.append((x, y))
        b.append(
            f"<polyline points='{' '.join(f'{x:.1f},{y:.1f}' for x, y in pts)}' fill='none' stroke='{c}' stroke-width='2'/>"
        )
        b += [dot(x, y, c, 5) for x, y in pts]
        b.append(rect(x1 - 130, 80 + 20 * j, 12, 12, c, rx=2))
        b.append(text(x1 - 112, 91 + 20 * j, k, 12))
    return svg(
        760,
        400,
        "".join(b),
        "Dose-response",
        "Flip rate by pressure level for threat, shame and coercion.",
    )


def fig4(ps, mt):
    v = mt["metrics"]
    sim, real = v["false_pass_sim"], v["false_pass_real"]
    x0, x1, top, rowh = 250, 720, 100, 70
    b = [text(20, 28, "Simulation vs measured", 16, weight="bold")]
    b.append(
        text(
            20,
            48,
            "Two different quantities on one axis. They are not comparable as accuracy. 95% CIs.",
            11,
            fill=MUTE,
        )
    )
    b.append(axis_x(x0, x1, top + 2 * rowh, top - 8))
    rows = [
        (
            "SIMULATION: scoring engine false-pass",
            "at V >= 0.65, assumed generators",
            sim,
            SIM,
        ),
        (
            "MEASURED: models flip to a wrong claim",
            "under pressure prompts, pooled",
            real,
            MEAS,
        ),
    ]
    for i, (lab, sub, c, col) in enumerate(rows):
        y = top + rowh * i + rowh / 2
        b.append(text(x0 - 12, y - 2, lab, 12, "end", weight="bold"))
        b.append(text(x0 - 12, y + 14, sub, 11, "end", MUTE))
        lo, hi = c["ci"]
        b.append(rect(x0, y - 11, (x1 - x0) * c["value"], 22, col + "55", col))
        b.append(line(x0 + (x1 - x0) * lo, y, x0 + (x1 - x0) * hi, y, INK, 2))
        b.append(
            text(
                x0 + (x1 - x0) * hi + 8,
                y + 4,
                pct(c["value"]),
                12,
                fill=INK,
                weight="bold",
            )
        )
    b.append(
        text(
            20,
            top + 2 * rowh + 50,
            "The simulation describes formulas under assumed inputs. The measured value describes named local models on one task.",
            11,
            fill=MUTE,
        )
    )
    return svg(
        760,
        top + 2 * rowh + 70,
        "".join(b),
        "Simulation versus measured",
        "False-pass of the scoring engine in simulation next to the measured model flip rate.",
    )


def fig5():
    b = [text(20, 28, "Engine and benchmark pipeline", 16, weight="bold")]
    b.append(
        text(
            20,
            48,
            "Left: the scoring engine under test (SIMULATION). Right: the behaviour harness (MEASURED).",
            11,
            fill=MUTE,
        )
    )

    def box(x, y, w, h, lab, sub="", col=CTRL):
        s = rect(x, y, w, h, "#f7fafc", col)
        s += text(x + w / 2, y + 22, lab, 13, "middle", INK, "bold")
        if sub:
            s += text(x + w / 2, y + 40, sub, 10, "middle", MUTE)
        return s

    def arrow(x1, y1, x2, y2, c=MUTE):
        a = line(x1, y1, x2, y2, c, 1.5)
        a += (
            f"<polygon points='{x2},{y2} {x2 - 7},{y2 - 4} {x2 - 7},{y2 + 4}' fill='{c}'/>"
            if y1 == y2
            else ""
        )
        return a

    b.append(text(20, 84, "Study S: scoring engine", 13, weight="bold", fill=SIM))
    b.append(box(20, 96, 120, 52, "inputs", "drift, halluc, anchor", SIM))
    b.append(box(190, 96, 120, 52, "T, S, A", "truth, Fibonacci, anchor", SIM))
    b.append(box(360, 96, 120, 52, "V score", "weights 0.3/0.2/0.2/0.3", SIM))
    b.append(box(530, 96, 120, 52, "prime gate", "hash mod p veto", SIM))
    b.append(box(530, 180, 120, 52, "pass / block", "V >= 0.65 and clear", SIM))
    for x in (140, 310, 480):
        b.append(arrow(x, 122, x + 50, 122, SIM))
    b.append(line(590, 148, 590, 180, SIM, 1.5))
    b.append(
        text(
            20, 268, "Studies B and P: behaviour harness", 13, weight="bold", fill=MEAS
        )
    )
    b.append(box(20, 280, 130, 52, "680 statements", "true / false, seed 7", MEAS))
    b.append(box(190, 280, 150, 52, "13 framings", "4 Study B + 9 pressure", MEAS))
    b.append(box(380, 280, 130, 52, "Ollama model", "1 token, T = 0", MEAS))
    b.append(
        box(550, 280, 150, 52, "logprob(True) vs (False)", "answer + flip rate", MEAS)
    )
    for x in (150, 340, 510):
        b.append(arrow(x, 306, x + 40, 306, MEAS))
    b.append(box(550, 364, 150, 52, "bootstrap CI", "effect size, McNemar, Holm", MEAS))
    b.append(line(625, 332, 625, 364, MEAS, 1.5))
    b.append(
        text(
            20,
            392,
            "Raw trials stay off-repo; summaries go to results/.",
            11,
            fill=MUTE,
        )
    )
    b.append(
        text(
            20, 410, "No hidden state is read. No feelings are measured.", 11, fill=MUTE
        )
    )
    return svg(
        760,
        440,
        "".join(b),
        "Engine and benchmark pipeline",
        "Block diagram of the scoring engine and the behaviour harness.",
    )


def main():
    ps = json.loads((ROOT / "results" / "pressure_summary.json").read_text())
    mt = json.loads((ROOT / "results" / "metrics.json").read_text())
    OUT.mkdir(parents=True, exist_ok=True)
    files = {
        "fig1-false-pass-by-model.svg": fig1(ps),
        "fig2-pressure-vs-control.svg": fig2(ps),
        "fig3-dose-response.svg": fig3(ps),
        "fig4-sim-vs-real.svg": fig4(ps, mt),
        "fig5-engine.svg": fig5(),
    }
    for name, body in files.items():
        (OUT / name).write_text(body)
        print("wrote", name)


if __name__ == "__main__":
    main()
