ifeq ($(OS),Windows_NT)
    NAME := $(EXE).exe
else
    NAME := $(EXE)
endif

rule:
	RUSTFLAGS="-Ctarget-cpu=native" cargo build --release
	cp ../target/release/cheers.exe $(NAME)
