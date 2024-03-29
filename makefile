ifeq ($(OS),Windows_NT)
    NAME := $(EXE).exe
	BIN := cheers.exe
else
    NAME := $(EXE)
	BIN := cheers
endif

rule:
	RUSTFLAGS="-Ctarget-cpu=native" cargo build --release --bin cheers
	cp ./target/release/$(BIN) $(NAME)
