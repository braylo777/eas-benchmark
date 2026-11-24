# EAW benchmark. `make metrics` rebuilds results/metrics.json and the README number table.
.PHONY: test cov metrics check pressure-run pressure-analyze paper docker

test:
	cd rust && cargo test --release
	python3 tests/test_engine.py

cov:
	cd rust && cargo llvm-cov --summary-only --ignore-filename-regex 'src/bin' --fail-under-lines 90

metrics:
	cd rust && cargo run --release --bin metrics -- --coverage
	python3 scripts/readme_numbers.py --write

check:
	python3 scripts/readme_numbers.py --check

pressure-run:
	cd rust && cargo run --release --bin pressure -- run --models qwen2.5:0.5b,gemma3:1b,llama3.2:1b,qwen2.5:1.5b,granite3.3:2b,qwen2.5:3b,llama3.2:3b,phi4-mini:3.8b,qwen2.5-coder:7b

pressure-analyze:
	cd rust && cargo run --release --bin pressure -- analyze

paper:
	python3 scripts/make_figures.py
	python3 scripts/build_paper.py

docker:
	docker build -t eaw-benchmark .
