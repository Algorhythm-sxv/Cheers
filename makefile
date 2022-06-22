EXE = cheers

rule:
	cargo rustc --release -p cheers -- --C target-cpu=native
