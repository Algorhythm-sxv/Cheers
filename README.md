# Cheers, a lighthearted UCI chess engine in Rust

<div align="center">

  ![Cheers logo](logo.png)

  [![Release](https://img.shields.io/github/v/release/Algorhythm-sxv/Cheers)](https://github.com/Algorhythm-sxv/Cheers/releases/latest)
  [![Commits](https://img.shields.io/github/commits-since/Algorhythm-sxv/Cheers/latest
)](https://github.com/Algorhythm-sxv/Cheers/commits/master)

</div>

Cheers is a free and open source chess engine utilising the UCI protocol, and should be used with a compatible GUI like Cute Chess or a command-line test runner like `cutechess-cli`.

Cheers is not very notable as chess engines go, but serves as an example of a strong chess engine using a 'hand-crafted' evaluation function, as opposed to a neural network evaluation.

## Building Cheers
Prebuilt binaries for x86 Windows and Linux are available in the **Releases** tab, if these are incompatible with your system you can build Cheers from source on any platform with a Rust compiler.

```
git clone https://github.com/Algorhythm-sxv/Cheers
cd Cheers
cargo b -r
```

### Maximising performance
The performance of the resulting binary can be slightly optimised by enabling native CPU code generation and using the `production` cargo profile.

Enabling native CPU code generation involves setting the `RUSTFLAGS` environment variable, which will depend on your system. The `production` profile can be selected with the `--profile` option to `cargo build`.

An example for Linux:
```
$ export RUSTFLAGS=-Ctarget-cpu=native
$ cargo b --profile production
```

## Acknowledgements
Minuskelvin and Analog Hors for allowing me onto their OpenBench instance, which provided invaluable testing without which Cheers would never have gotten this far.

The Chess Programming Discord, who answered many questions while I was still learning the ropes